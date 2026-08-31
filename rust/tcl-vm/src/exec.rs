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

use std::collections::HashMap;
use std::rc::Rc;

use tcl_bytecode::{ErrorRegion, FunctionAsm, INDEX_END, Instruction, ModuleAsm, Op, Operand};
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::expr::{BinOp, UnaryOp};
use tcl_syntax::value::string_char_len;

use crate::command::{Command, ProcDef};
use crate::expr;
use crate::interp::{CmdTraceEntry, CommandSidecarHandle, CommandSidecarKey, Vm, err, ok};
use crate::value::Value;

/// Active `foreach` iteration state (C Tcl `ForeachInfo` + the loop counters).
struct ForeachState {
    /// Loop-variable groups (from the `FOREACH_START` aux).
    var_groups: Vec<Vec<String>>,
    /// The value list for each group.
    lists: Vec<Vec<Value>>,
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
/// local slot named in the `dictFirst` / `dictNext` operand. Holds the dict's
/// key/value pairs in insertion order plus the current position — the analogue
/// of C Tcl's `Tcl_DictSearch` stored in the iterator temp var.
struct DictIterState {
    pairs: Vec<(Value, Value)>,
    pos: usize,
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

/// One activation record: a bytecode function in mid-execution. `pub(crate)` so
/// a suspended coroutine can own its frozen activation stack (`Vec<Frame>`); the
/// fields stay private to `exec` (a coroutine only stores/moves whole frames and
/// pushes its resume value via [`Frame::push_operand`]).
pub(crate) struct Frame {
    asm: Rc<FunctionAsm>,
    /// Dialect-profile generation under which this activation's bytecode was
    /// compiled.
    /// Stored frames cannot be rewound/recompiled after a profile switch (in
    /// particular a suspended coroutine has a live PC and operand stack), so
    /// the trampoline rejects stale execution before another specialised
    /// opcode can run.
    profile_generation: u64,
    off2idx: Rc<HashMap<i32, usize>>,
    /// `FOREACH_START` index → paired `FOREACH_STEP` index (the implicit jump).
    foreach_pairs: Rc<HashMap<usize, usize>>,
    pc: usize,
    stack: Vec<Value>,
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
    /// Last value dropped by `POP` — the `DONE` result when the stack is empty.
    last_result: Value,
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
    /// For a script activation, the `errorInfo` body label the uncompiled command
    /// would add on error (`("eval" body line N)` / `("uplevel" body line N)`).
    /// `None` for a command-substitution `EVAL_STK`, which adds no body frame.
    body_label: Option<&'static str>,
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
    /// back in by `each_loop`'s collect/continue/break rules (issue #1311).
    each_loop: Option<Box<EachLoopState>>,
    /// Set on a **try-phase** activation: runs one phase (body/handler/
    /// `finally`) of a `try` construct's real bytecode (unlike `subst`/
    /// `each_loop`, not scanner-driven — the phase's script dispatches
    /// normally). On completion, `Vm::unwind` calls `cmd_try::advance_try`
    /// with the taken state, which decides whether to push a fresh try-phase
    /// activation for the next phase or deliver the construct's final
    /// completion (issue #1311).
    ///
    /// A completed phase is transparent to the caller's enclosing loop when
    /// no handler/finally clause consumes its `break`/`continue`, just like an
    /// `eval` body. The common unwind hand-off below owns that decision for all
    /// child activations, including a proc boundary that turned
    /// `return -code continue` into `TCL_CONTINUE`.
    try_ctx: Option<Box<crate::cmd_try::TryState>>,
    /// Execution-trace leave contexts this activation settles on completion
    /// (M16.3): each traced dispatch that deferred its body to this frame.  A
    /// `tailcall` transfers the issuing frame's contexts to its replacement,
    /// hence a Vec.  Fired (and step scopes popped) as the frame unwinds.
    exec_leave: Vec<ExecLeaveCtx>,
    /// A command name to delete once this activation completes, regardless of
    /// completion code — `apply`'s temporary lambda proc (issue #1311), torn
    /// down the same way whether the call returned, errored, or unwound a
    /// `break`/`continue`/`return`. `None` for every other script activation.
    cleanup_proc: Option<String>,
}

/// One traced dispatch's leave-side state: the invoked command string, the
/// trace-table key to fire `leave` on, and how many step scopes the dispatch
/// pushed (popped before firing).
pub(crate) struct ExecLeaveCtx {
    cmd_string: String,
    key: CommandSidecarHandle,
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
}

/// A `catch` body deferred to the explicit stack: the compiled body plus the
/// variable names to bind once it completes. Mirrors `pending_eval`'s tuple, but
/// its completion is absorbed (see [`Frame::catch`]).
pub(crate) struct CatchReq {
    pub(crate) asm: Rc<FunctionAsm>,
    pub(crate) resvar: Option<Value>,
    pub(crate) optvar: Option<Value>,
}

/// A `subst` deferred to the explicit stack: the template plus its three
/// substitution switches. Parked in `Vm.pending_subst` by `cmd_subst` and drained
/// into a subst activation (see [`Frame::subst`]), so a `yield` inside a `[…]`
/// stays yieldable.
pub(crate) struct SubstReq {
    pub(crate) template: String,
    pub(crate) backslashes: bool,
    pub(crate) commands: bool,
    pub(crate) variables: bool,
}

/// One `foreach`/`lmap` variable-binding group: its loop variable names and
/// this group's flattened value list (`each_loop`'s `(varList, list)` pair).
pub(crate) struct EachLoopGroup {
    pub(crate) vars: Vec<String>,
    pub(crate) values: Vec<Value>,
}

/// A `foreach`/`lmap` runtime-fallback loop deferred to the explicit stack:
/// each iteration's body runs as a yieldable child script frame (issue #1311
/// — this is what a value-consumed `lmap`, e.g. `set r [lmap x {1 2} { yield
/// $x }]`, needs: reached via generic command dispatch rather than the
/// compiler's inline `LMAP_COLLECT` loop, its body used to run through
/// `Vm::eval_source`'s nested drive). Parked in `Vm.pending_each_loop` by
/// `each_loop` and drained into an each-loop activation (see
/// [`Frame::each_loop`]).
pub(crate) struct EachLoopReq {
    pub(crate) name: &'static str,
    pub(crate) collect: bool,
    pub(crate) groups: Vec<EachLoopGroup>,
    pub(crate) iterations: usize,
    pub(crate) body: Rc<FunctionAsm>,
}

/// [`EachLoopReq`]'s live iteration state, carried on the activation
/// ([`Frame::new_each_loop`]) between ticks.
struct EachLoopState {
    name: &'static str,
    collect: bool,
    groups: Vec<EachLoopGroup>,
    iterations: usize,
    body: Rc<FunctionAsm>,
    /// The next iteration to run (`0..iterations`); set to `iterations` early
    /// by a `break` to end the loop on the next tick without running more
    /// bodies.
    it: usize,
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

impl Frame {
    pub(crate) fn new(asm: Rc<FunctionAsm>, is_proc: bool, profile_generation: u64) -> Self {
        let off2idx = Rc::new(build_off2idx(&asm));
        let foreach_pairs = Rc::new(pair_foreach(&asm));
        Self {
            asm,
            profile_generation,
            off2idx,
            foreach_pairs,
            pc: 0,
            stack: Vec::new(),
            foreach_stack: Vec::new(),
            dict_iters: HashMap::new(),
            expand_markers: Vec::new(),
            catch_ranges: Vec::new(),
            last_result: Value::empty(),
            is_proc,
            is_script: false,
            body_label: None,
            catch: None,
            subst: None,
            each_loop: None,
            try_ctx: None,
            exec_leave: Vec::new(),
            cleanup_proc: None,
        }
    }

    /// A transparent script activation (see [`Frame::is_script`]) for a body run
    /// yieldably on the explicit stack (`EVAL_STK`, and the `eval`/`uplevel`
    /// builtins routed through it). `label` is the `errorInfo` body-frame label
    /// (`Some("eval")`/`Some("uplevel")`), or `None` for a command substitution.
    pub(crate) fn new_script(
        asm: Rc<FunctionAsm>,
        label: Option<&'static str>,
        profile_generation: u64,
    ) -> Self {
        let mut f = Self::new(asm, false, profile_generation);
        f.is_script = true;
        f.body_label = label;
        f
    }

    /// A **catch** activation: the body runs on the explicit stack (yieldable),
    /// but its completion is absorbed by the catch epilogue rather than delivered
    /// to the parent (see [`Frame::catch`]).
    pub(crate) fn new_catch(req: CatchReq, profile_generation: u64) -> Self {
        let mut f = Self::new(req.asm, false, profile_generation);
        f.catch = Some(Box::new(CatchCtx {
            resvar: req.resvar,
            optvar: req.optvar,
        }));
        f
    }

    /// A **subst** activation: a scanner-driven frame (empty placeholder asm — it
    /// never executes bytecode) carrying the resumable scan state. `tick` runs the
    /// scanner instead of the bytecode dispatch when `subst` is set.
    pub(crate) fn new_subst(req: SubstReq, profile_generation: u64) -> Self {
        let mut f = Self::new(Rc::new(FunctionAsm::default()), false, profile_generation);
        f.subst = Some(Box::new(crate::subst::SubstState::new(
            req.template,
            req.backslashes,
            req.commands,
            req.variables,
        )));
        f
    }

    /// An **each-loop** activation: a scanner-driven frame (empty placeholder
    /// asm, like `subst`) carrying the resumable `foreach`/`lmap` iteration
    /// state. `tick` runs the loop driver instead of the bytecode dispatch when
    /// `each_loop` is set.
    pub(crate) fn new_each_loop(req: EachLoopReq, profile_generation: u64) -> Self {
        let mut f = Self::new(Rc::new(FunctionAsm::default()), false, profile_generation);
        f.each_loop = Some(Box::new(EachLoopState {
            name: req.name,
            collect: req.collect,
            groups: req.groups,
            iterations: req.iterations,
            body: req.body,
            it: 0,
            collected: Vec::new(),
        }));
        f
    }

    /// A **try-phase** activation ([`Frame::try_ctx`]): runs `req.asm` (the
    /// current phase's compiled script) as ordinary bytecode, tagged with the
    /// state `Vm::unwind` hands to `cmd_try::advance_try` on completion.
    pub(crate) fn new_try(req: crate::cmd_try::TryReq, profile_generation: u64) -> Self {
        let mut f = Self::new(req.asm, false, profile_generation);
        f.try_ctx = Some(Box::new(req.state));
        f
    }

    /// Push `v` onto this frame's operand stack. Used by a coroutine `resume` to
    /// deliver the resume value as the result of the `yield` that suspended here
    /// — exactly where the normal builtin-return path (`f.stack.push`) would have
    /// put the command's result, so the following instruction is oblivious.
    pub(crate) fn push_operand(&mut self, v: Value) {
        self.stack.push(v);
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
    Call { proc: Rc<ProcDef>, argv: Vec<Value> },
    /// Run a compiled script on the explicit stack — push a *transparent* script
    /// activation ([`Frame::new_script`]). Used by `EVAL_STK` (and the
    /// `eval`/`uplevel`/`apply` builtins routed through it) so a `yield` inside
    /// the body stays yieldable instead of re-entering the evaluator on the
    /// native stack. `label` is the `errorInfo` body-frame label (`None` for
    /// command subst); `cleanup_proc` is a command name to delete once the
    /// pushed frame completes (`apply`'s temporary lambda proc).
    PushScript {
        asm: Rc<FunctionAsm>,
        label: Option<&'static str>,
        cleanup_proc: Option<String>,
    },
    /// Run a `catch` body on the explicit stack (yieldable) via a catch
    /// activation ([`Frame::new_catch`]); its completion is absorbed by the catch
    /// epilogue. Drained from `Vm.pending_catch`, mirroring `PushScript`.
    PushCatch(CatchReq),
    /// Run a `subst` on the explicit stack (yieldable) via a subst activation
    /// ([`Frame::new_subst`]); its `[…]` bodies run as child script frames and are
    /// folded back by subst rules. Drained from `Vm.pending_subst`.
    PushSubst(SubstReq),
    /// Run a `foreach`/`lmap` runtime-fallback loop on the explicit stack
    /// (yieldable) via an each-loop activation ([`Frame::new_each_loop`]); each
    /// iteration's body runs as a child script frame and is folded back by
    /// `each_loop`'s collect/continue/break rules. Drained from
    /// `Vm.pending_each_loop` (issue #1311).
    PushEachLoop(EachLoopReq),
    /// Run one phase (body/handler/`finally`) of a `try` on the explicit stack
    /// (yieldable) via a try-phase activation ([`Frame::new_try`]); its
    /// completion decides the next phase via `cmd_try::advance_try`. Drained
    /// from `Vm.pending_try` (issue #1311).
    PushTry(crate::cmd_try::TryReq),
    /// `tailcall cmd ?arg …?` — the current proc finishes and `cmd args` runs in
    /// its place (in the caller's activation), its result becoming the proc's.
    /// `words` is `[cmd, arg, …]` (the `tailcall` prefix word already dropped).
    Tailcall(Vec<Value>),
    /// `yield`/`yieldto` — suspend the running coroutine, freezing the whole
    /// activation stack. Only reachable in [`DriveMode::CoroDriver`]; the driver
    /// returns [`RunExit::Yielded`] leaving `acts` intact (pc already past the
    /// suspend point).
    Suspend(YieldReq),
}

/// A coroutine suspend request, produced by the `yield`/`yieldto` builtins and
/// carried out of [`Vm::drive`](Vm) to the coroutine's `resume`.
pub(crate) enum YieldReq {
    /// `yield ?value?` — `value` is what the resumer receives.
    Yield(Value),
    /// `yieldto cmd ?arg …?` — the resume runs `cmd args` in the resumer's
    /// context (a tail-call-flavoured handoff).
    YieldTo(Vec<Value>),
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

/// Re-flatten `(key, value)` pairs back into a dict `Value`.
fn dict_from_pairs(ps: &[(String, Value)]) -> Value {
    let mut v = Vec::with_capacity(ps.len() * 2);
    for (k, val) in ps {
        v.push(Value::string(k.as_str()));
        v.push(val.clone());
    }
    Value::list(v)
}

/// Re-word a list-parse failure as the **dict** failure C reports, and attach
/// the matching `-errorcode`.
///
/// `SetDictFromAny` (tclDictObj.c) hands `FindElement` the type strings
/// `dict`/`DICTIONARY`, so the same malformed input that gives
/// `list element in braces followed by "c" instead of space` /
/// `TCL VALUE LIST JUNK` from `llength` gives the `dict …` /
/// `TCL VALUE DICTIONARY JUNK` pair from `dict size`. The shared codec speaks
/// list, so the noun is swapped back on the dict side (issue #1573).
///
/// `missing value to go with key` is already dict-specific, but C still tags it
/// `TCL VALUE DICTIONARY` where this VM left it `NONE`.
///
/// Messages that are not value-parse failures at all — `wrong # args:`, a
/// missing key — are returned untouched, so they keep the `errorCode` their own
/// rules give them.
pub(crate) fn dict_parse_err(message: &str) -> Completion<Value> {
    let worded = tcl_cmd_core::dict::worded_parse_error(message);
    let (msg, code) = if worded.starts_with("dict element in ") {
        (worded, "TCL VALUE DICTIONARY JUNK")
    } else if worded == "unmatched open brace in dict" {
        (
            "unmatched open brace in dict".to_owned(),
            "TCL VALUE DICTIONARY BRACE",
        )
    } else if worded == "unmatched open quote in dict" {
        (
            "unmatched open quote in dict".to_owned(),
            "TCL VALUE DICTIONARY QUOTE",
        )
    } else if worded == "missing value to go with key" {
        (worded, "TCL VALUE DICTIONARY")
    } else {
        return err(worded);
    };
    crate::command::err_with_code(msg, code)
}

/// The dict decode/rebuild helpers behind the `dict*` opcodes.
///
/// Every one of them starts from [`Vm::dict_pairs`], the VM's binding of the
/// shared [`ValueOps::dict_pairs`] owner — so the opcodes see the same
/// **canonical** pair list `dict size`, `dict get` via `dispatch_canon`, and
/// the WASM runtime see: first-occurrence position, **last value winning** on a
/// duplicate key (`SetDictFromAny`, tclDictObj.c(9.0.4):589, feeding
/// `Tcl_DictObjPut`'s hash overwrite). Decoding the list rep straight into
/// `chunks_exact(2)` pairs instead — as these opcodes used to — leaves both
/// values of a duplicate key in the list, and every `find` then reads or
/// rewrites the *first* one (issue #1427).
impl Vm {
    /// The dict's canonical ordered `(key-string, value)` pairs.
    ///
    /// A parse failure is reported as a **dict**, not a list: C's
    /// `SetDictFromAny` passes the type strings `dict`/`DICTIONARY` down to
    /// `FindElement`, so `dict size {a 1 {b}c d}` says `dict element in braces
    /// …` with `errorCode` `TCL VALUE DICTIONARY JUNK`. The shared codec is
    /// list-worded (it is the *list* element codec), so the noun is restored
    /// here — the one place every VM dict path now decodes through, which is
    /// what makes this a single fix rather than eleven (issue #1573).
    pub(crate) fn dict_pairs(
        &mut self,
        v: &Value,
    ) -> Result<Vec<(String, Value)>, Completion<Value>> {
        let pairs = tcl_syntax::value::ValueOps::dict_pairs(self, v)
            .map_err(|e| dict_parse_err(&e.message()))?;
        Ok(pairs
            .into_iter()
            .map(|(k, val)| (k.to_str().to_string(), val))
            .collect())
    }

    /// Set the nested `keys` path of dict `cur` to `value`, creating
    /// intermediate dicts as needed. Returns the new top-level dict value.
    fn dict_set_path(
        &mut self,
        cur: &Value,
        keys: &[Value],
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let mut ps = self.dict_pairs(cur)?;
        let k = keys[0].to_str().to_string();
        let newv = if keys.len() == 1 {
            value
        } else {
            let sub = ps
                .iter()
                .find(|(pk, _)| pk == &k)
                .map_or_else(Value::empty, |(_, v)| v.clone());
            self.dict_set_path(&sub, &keys[1..], value)?
        };
        if let Some(slot) = ps.iter_mut().find(|(pk, _)| pk == &k) {
            slot.1 = newv;
        } else {
            ps.push((k, newv));
        }
        Ok(dict_from_pairs(&ps))
    }

    /// Remove the nested `keys` path from dict `cur`. Returns the new top-level
    /// dict value (a no-op if the path is absent, matching `dict unset`).
    fn dict_unset_path(&mut self, cur: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
        let mut ps = self.dict_pairs(cur)?;
        let k = keys[0].to_str().to_string();
        if keys.len() == 1 {
            ps.retain(|(pk, _)| pk != &k);
        } else if let Some(idx) = ps.iter().position(|(pk, _)| pk == &k) {
            let inner = ps[idx].1.clone();
            ps[idx].1 = self.dict_unset_path(&inner, &keys[1..])?;
        }
        Ok(dict_from_pairs(&ps))
    }

    /// Update the single-level `key` of dict `cur` via `f` (given the current
    /// value if present), returning the new top-level dict value. Shared by the
    /// `DICT_INCR_IMM` / `DICT_APPEND` / `DICT_LAPPEND` opcodes.
    fn dict_update_single(
        &mut self,
        cur: &Value,
        key: &str,
        f: impl FnOnce(Option<&Value>) -> Result<Value, Completion<Value>>,
    ) -> Result<Value, Completion<Value>> {
        let mut ps = self.dict_pairs(cur)?;
        let newv = f(ps.iter().find(|(k, _)| k == key).map(|(_, v)| v))?;
        if let Some(slot) = ps.iter_mut().find(|(k, _)| k == key) {
            slot.1 = newv;
        } else {
            ps.push((key.to_string(), newv));
        }
        Ok(dict_from_pairs(&ps))
    }
}

/// Whether `s` can be safely wrapped in `{…}`: braces are balanced (ignoring
/// `\{`/`\}`) and it does not end in an escaping backslash.
fn brace_safe(s: &str) -> bool {
    let b = s.as_bytes();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' if i + 1 < b.len() => {
                i += 2;
                continue;
            }
            b'\\' => return false, // trailing lone backslash
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    depth == 0
}

/// Assemble alias-target words (`target prefix… args…`) into a script whose
/// re-parse dispatches them as one command — each word quoted for a *script*
/// context ([`quote_for_script`]) so compiled control commands (`try`, `if`,
/// …) and brace bodies survive.  Shared by the same-interp and cross-interp
/// alias dispatch paths.
pub(crate) fn alias_invoke_script(argv: &[Value]) -> String {
    argv.iter()
        .map(|v| quote_for_script(&v.to_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote a word for re-parsing in a *script* context: brace-wrap when safe
/// (preserving a `\<newline>` continuation for the target to collapse),
/// otherwise fall back to canonical list-element quoting.
fn quote_for_script(s: &str) -> String {
    if brace_safe(s) {
        format!("{{{s}}}")
    } else {
        tcl_syntax::list::list_element(s)
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
/// `INDEX_END`, not just `<= INDEX_END`. Detecting only `<= INDEX_END` (the old
/// test) misread `end+N` as a huge negative plain index, so e.g. `lrange $l
/// end+1 0` and `lrange $l 0 end+1` disagreed with the uncompiled command
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

/// Resolve a runtime index value, erroring on a non-integer spec (`bad index`)
/// — the validating form C's index-taking opcodes all use, since every one of
/// them routes through `TclGetIntForIndexM` and treats its failure as an error
/// (`INST_STR_INDEX`, `INST_STR_RANGE`, `INST_STR_REPLACE`, `INST_LREPLACE4`);
/// the non-validating `unwrap_or` forms it replaced turned a garbage index into
/// a plausible-looking value.
fn checked_index_value(v: &Value, len: usize) -> Result<isize, Completion<Value>> {
    let s = v.to_str();
    crate::command::resolve_index(&s, len).ok_or_else(|| {
        err(format!(
            "bad index \"{s}\": must be integer?[+-]integer? or end?[+-]integer?"
        ))
    })
}

/// List element at signed index, or empty when out of range.
fn get_at(items: &[Value], i: isize) -> Value {
    usize::try_from(i)
        .ok()
        .and_then(|x| items.get(x))
        .cloned()
        .unwrap_or_else(Value::empty)
}

/// Set the element at index `path` of `list` to `value`, returning the new
/// (sub)list — the shared core of `INST_LSET_LIST` / `INST_LSET_FLAT` (C's
/// `TclLsetList` / `TclLsetFlat`), and the runtime `lset` builtin's fallback
/// (`cmd_list.rs::cmd_lset`). An empty `path` replaces the whole value
/// (`lset x {} v` == `set x v`); each index is `end`/`end±N`-aware, with range
/// `0..=len` where `len` appends a fresh (possibly nested) slot. Error messages
/// match tclsh 9.0 (the reference standard).
///
/// Issue #996: this used to recurse once per path segment natively, with no
/// depth cap — trivially inflated via a long flat index path (`INST_LSET_FLAT`
/// / `lset listVar {*}[lrepeat 100000 0] v`). Empirically (a throwaway
/// `zzz_probe_depth lset <depth>` harness, deleted before this fix landed),
/// unguarded input overflowed the native stack (SIGABRT) between depth 1800
/// and 2000 on a 2 MiB thread. Rewritten iteratively — an explicit
/// work-stack instead of one native call per index — which eliminates the
/// native-stack risk entirely rather than just capping it: walk down `path`
/// recording each level's element vector and the index being set (or
/// appended to), then rebuild bottom-up. This is on the hot bytecode path
/// (`INST_LSET_LIST`/`INST_LSET_FLAT`), so the signature (and its two
/// `exec.rs` callers) is unchanged.
pub(crate) fn lset_descend(
    list: &Value,
    path: &[Value],
    value: Value,
) -> Result<Value, Completion<Value>> {
    let mut frames: Vec<(Vec<Value>, usize)> = Vec::with_capacity(path.len());
    let mut cur = list.clone();
    for spec in path {
        let elems = match cur.as_list() {
            Ok(e) => e,
            Err(e) => return Err(err(e.message)),
        };
        let len = elems.len();
        let spec_str = spec.to_str();
        let Some(idx) = crate::command::resolve_index(&spec_str, len) else {
            return Err(err(format!(
                "bad index \"{spec_str}\": must be integer?[+-]integer? or end?[+-]integer?"
            )));
        };
        if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) > len {
            return Err(err(format!("index \"{spec_str}\" out of range")));
        }
        let idx = usize::try_from(idx).unwrap_or(0);
        let appending = idx == len;
        let child = if appending {
            Value::list(Vec::new())
        } else {
            elems[idx].clone()
        };
        frames.push(((*elems).clone(), idx));
        cur = child;
    }
    let mut new_value = value;
    for (mut out, idx) in frames.into_iter().rev() {
        if idx == out.len() {
            out.push(new_value);
        } else {
            out[idx] = new_value;
        }
        new_value = Value::list(out);
    }
    Ok(new_value)
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

/// Character substring `[lo..=hi]` (clamped), empty when the range is empty.
fn char_range(chars: &[char], lo: isize, hi: isize) -> Value {
    let len = isize::try_from(chars.len()).unwrap_or(isize::MAX);
    if hi < 0 || lo >= len {
        return Value::string("");
    }
    let lo = usize::try_from(lo.max(0)).unwrap_or(0);
    let hi = usize::try_from(hi.min(len - 1)).unwrap_or(0);
    if lo > hi {
        return Value::string("");
    }
    Value::string(chars[lo..=hi].iter().collect::<String>())
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

fn bin(f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::arith(op, &a, &b) {
        Ok(v) => {
            f.stack.push(v);
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

fn cmp(f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::compare(op, &a, &b) {
        Ok(t) => {
            f.stack.push(Value::bool(t));
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

/// Eager (non-short-circuit) logical AND/OR: pop both operands, coerce each
/// to boolean, and push the boolean result.
///
/// Codegen never emits [`Op::LAND`]/[`Op::LOR`] directly — `&&`/`||` compile
/// to short-circuit jump sequences instead
/// (`tcl_compiler::codegen::expressions::emit_expr_binary`) — but the two
/// opcodes are still reachable through [`Op::from_binop`] (which maps
/// `BinOp::And`/`BinOp::Or` to them) and are part of the artifact's public
/// surface, so dispatch gives them real semantics rather than falling
/// through to an "opcode not implemented" error.
fn land_lor(f: &mut Frame, is_and: bool) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    let a_true = a.as_bool().map_err(|e| err(e.message))?;
    let b_true = b.as_bool().map_err(|e| err(e.message))?;
    f.stack.push(Value::bool(if is_and {
        a_true && b_true
    } else {
        a_true || b_true
    }));
    Ok(())
}

fn un(f: &mut Frame, op: UnaryOp) -> Result<(), Completion<Value>> {
    let v = pop(f);
    match expr::unary(op, &v) {
        Ok(r) => {
            f.stack.push(r);
            Ok(())
        }
        Err(e) => Err(err(e.message)),
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
fn irule(f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::irule_binary(op, &a, &b) {
        Ok(v) => {
            f.stack.push(v);
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

/// The Tcl `wrong # args` usage message for a proc.
fn proc_usage(proc: &ProcDef) -> String {
    // `proc.name` is the VM's unrooted key: construction-inverse tail
    // (#934) — an `rsplit` would misread a lone-colon name.
    let simple = proc.usage_name.as_deref().map_or_else(
        || crate::interp::key_holder_and_tail_unrooted(&proc.name).1,
        str::to_owned,
    );
    // Each desired-arg word is list-quoted (C's `Tcl_WrongNumArgs`), so a param
    // name containing spaces shows as `{a b c}` and an empty name as `{}`
    // (proc-3.6/3.7). A trailing `args` becomes the raw `?arg ...?` suffix — not
    // a list element, so its space isn't braced.
    //
    // A multi-word `usage_name` (`apply lambdaExpr`, a method's `<obj> <method>`)
    // is already several leading words, not one name — split it so each is its
    // own (unbraced) list element rather than one `{apply lambdaExpr}` blob.
    let mut elems: Vec<Value> = if proc.usage_name.is_some() {
        simple.split(' ').map(Value::string).collect()
    } else {
        vec![Value::string(simple)]
    };
    let mut suffix = "";
    for p in &proc.params {
        if p.name == "args" {
            suffix = " ?arg ...?";
            break;
        } else if p.default.is_some() {
            elems.push(Value::string(format!("?{}?", p.name)));
        } else {
            elems.push(Value::string(p.name.clone()));
        }
    }
    let joined = Value::list(elems).to_str();
    format!("wrong # args: should be \"{joined}{suffix}\"")
}

impl Vm {
    /// Run a module: register its compiled procs, then run the top-level script.
    pub fn run_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        if let Err(error) = self.validate_module_profile(module) {
            return crate::command::completion_from_tcl_error(error);
        }
        self.claim_number_grammar();
        self.merge_procs(&module.procedures);
        self.run_function_unchecked(&module.top_level)
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
    /// [`Vm::invoke_function`] instead, which pays that copy once (issue
    /// #1373 finding 3).
    pub fn run_function(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        if !self.dialect_profile().is_fallback() {
            return err(format!(
                "profile-less bytecode cannot run under dialect profile {}",
                self.dialect_profile().name
            ));
        }
        self.run_function_unchecked(asm)
    }

    /// Execute a function whose containing module has already passed the
    /// exact-profile check, or whose ownership is otherwise VM-internal.
    fn run_function_unchecked(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        self.run_function_rc(Rc::new(asm.clone()))
    }

    /// Run an already-`Rc`-wrapped function to completion — the clone-free
    /// path behind [`Vm::invoke_function`].
    pub(crate) fn run_function_rc(&mut self, asm: Rc<FunctionAsm>) -> Completion<Value> {
        self.run_activation(Frame::new(asm, false, self.profile_generation()))
    }

    /// Run one activation stack to completion (the ordinary, non-coroutine path).
    /// The initial frame's `is_proc` decides whether the outermost body is a proc
    /// call (so `unwind` pops a call-frame + namespace and absorbs `return`):
    /// `false` for a module top-level,
    /// `true` for [`invoke_command`](Self::invoke_command) running a proc body.
    fn run_activation(&mut self, initial: Frame) -> Completion<Value> {
        let mut acts: Vec<Frame> = vec![initial];
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
    /// (`enter_proc`/`unwind`/`run_tailcall`/inline-loop redirection) is shared.
    ///
    /// `activation_depth` counts nested `drive` invocations — the host re-entry
    /// counter `yield` consults to reject a suspend across a `catch`/`uplevel`/
    /// `eval`/OO-method boundary (`cannot yield: C stack busy`).
    fn drive(&mut self, acts: &mut Vec<Frame>, mode: DriveMode) -> RunExit {
        self.activation_depth += 1;
        let exit = self.drive_loop(acts, mode);
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

    /// Enter an already-validated procedure body.  The frame deliberately
    /// carries the generation at which the body was compiled, rather than the
    /// VM's current generation: if a source-less stored body could not be
    /// refreshed after a profile change, `drive_loop` must reject it before an
    /// inline opcode can run.
    fn push_proc_frame(&mut self, acts: &mut Vec<Frame>, proc: &Rc<ProcDef>) {
        let mut frame = Frame::new(
            Rc::clone(&proc.body),
            true,
            proc.compiled_profile_generation,
        );
        if let Some(ctx) = self.pending_exec_leave.take() {
            frame.exec_leave.push(ctx);
        }
        acts.push(frame);
    }

    fn drive_loop(&mut self, acts: &mut Vec<Frame>, mode: DriveMode) -> RunExit {
        loop {
            // Enforce `interp limit $i time` for unbounded bytecode loops: the
            // counter is Vm-scoped so it survives the short activations a
            // command-driven loop re-enters (see `limit_check_tick`).
            if let Some(c) = self.limit_check_tick() {
                if let Some(done) = self.unwind(acts, c) {
                    return RunExit::Done(done);
                }
                continue;
            }
            match self.unwind_stale_profile_frame(acts) {
                Ok(false) => {}
                Ok(true) => continue,
                Err(exit) => return exit,
            }
            let tick = {
                let top = acts.last_mut().expect("activation stack is non-empty");
                self.tick(top)
            };
            match tick {
                Tick::Continue => {}
                Tick::Call { proc, argv } => match self.enter_proc(&proc, &argv) {
                    Ok(()) => self.push_proc_frame(acts, &proc),
                    Err(c) => {
                        // A traced dispatch that failed at proc entry (arity)
                        // still settles its leave with the error.
                        let c = match self.pending_exec_leave.take() {
                            Some(ctx) => self.finish_exec_leave(&ctx, &c).unwrap_or(c),
                            None => c,
                        };
                        if let Some(done) = self.unwind(acts, c) {
                            return RunExit::Done(done);
                        }
                    }
                },
                Tick::PushScript {
                    asm,
                    label,
                    cleanup_proc,
                } => {
                    let mut fr = Frame::new_script(asm, label, self.profile_generation());
                    fr.cleanup_proc = cleanup_proc;
                    if let Some(ctx) = self.pending_exec_leave.take() {
                        fr.exec_leave.push(ctx);
                    }
                    acts.push(fr);
                }
                Tick::PushCatch(req) => {
                    let mut fr = Frame::new_catch(req, self.profile_generation());
                    if let Some(ctx) = self.pending_exec_leave.take() {
                        fr.exec_leave.push(ctx);
                    }
                    acts.push(fr);
                }
                Tick::PushSubst(req) => {
                    let mut fr = Frame::new_subst(req, self.profile_generation());
                    if let Some(ctx) = self.pending_exec_leave.take() {
                        fr.exec_leave.push(ctx);
                    }
                    acts.push(fr);
                }
                Tick::PushEachLoop(req) => {
                    let mut fr = Frame::new_each_loop(req, self.profile_generation());
                    if let Some(ctx) = self.pending_exec_leave.take() {
                        fr.exec_leave.push(ctx);
                    }
                    acts.push(fr);
                }
                Tick::PushTry(req) => {
                    let mut fr = Frame::new_try(req, self.profile_generation());
                    if let Some(ctx) = self.pending_exec_leave.take() {
                        fr.exec_leave.push(ctx);
                    }
                    acts.push(fr);
                }
                Tick::Return(c) => {
                    if let Some(done) = self.settle_completion(acts, c) {
                        return RunExit::Done(done);
                    }
                }
                Tick::Tailcall(words) => {
                    if let Some(done) = self.run_tailcall(acts, &words) {
                        return RunExit::Done(done);
                    }
                }
                Tick::Suspend(req) => match mode {
                    // A coroutine `yield`/`yieldto` freezes `acts` in place (pc
                    // already past the suspend point) and hands the request out
                    // to its `resume`.
                    DriveMode::CoroDriver => {
                        return RunExit::Yielded(req);
                    }
                    // Defensive: the `yield` builtin's boundary check rejects a
                    // top-level suspend (`cannot yield: C stack busy`) before it
                    // reaches here.
                    DriveMode::Plain => {
                        let c = err("cannot yield: C stack busy");
                        if let Some(done) = self.unwind(acts, c) {
                            return RunExit::Done(done);
                        }
                    }
                },
            }
        }
    }

    /// Reject a frame whose profile changed while it was live. Stored
    /// activations can have a PC and operand stack in the middle of a lowered
    /// command, so they cannot be safely recompiled like an unentered proc.
    /// `Ok(true)` means unwinding consumed the stale frame and the caller must
    /// continue its drive loop; an empty stack is returned as a completed run.
    fn unwind_stale_profile_frame(&mut self, acts: &mut Vec<Frame>) -> Result<bool, RunExit> {
        if acts
            .last()
            .is_some_and(|frame| frame.profile_generation != self.profile_generation())
        {
            // This is a command-like failure at the current instruction
            // boundary. Route it through the ordinary settlement seam so an
            // enclosing inline `catch` observes it before the activation is
            // unwound.
            return match self.settle_completion(
                acts,
                err("cannot continue bytecode after dialect profile changed"),
            ) {
                Some(done) => Err(RunExit::Done(done)),
                None => Ok(true),
            };
        }
        Ok(false)
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
            self.log_command_info(&r.cmd_text, "", r.cmd_line);
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
        let Some(pos) = f.catch_ranges.iter().rposition(|r| !r.in_handler) else {
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
        let options = self.digest_catch_options(&c);
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

    fn unwind(
        &mut self,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        loop {
            let mut act = acts.pop().expect("unwinding a non-empty stack");
            // An error unwinding through an inlined command body (`eval {…}`)
            // adds the body frames the uncompiled command would, before this
            // activation's own proc frame (innermost first) — the compiled
            // analogue of C's `CmdFrame` trace.
            if c.code == Code::Error {
                self.apply_error_regions(&act);
                // A transparent `eval`/`uplevel` body adds its `("eval" body line
                // N)` frame as the error unwinds out of it — the compiled-stack
                // analogue of the post-`eval_source` `append_body_frame` the
                // builtins used to do (eval-2.5) — then logs the `eval`/`uplevel`
                // command's own `invoked from within "eval …"` call-site frame
                // (the enclosing instruction that pushed this body), as the
                // nested-drive builtins' propagated error did.
                if let Some(label) = act.body_label {
                    self.append_body_frame(label);
                    if let Some((cmd, line)) = acts.last().and_then(|parent| {
                        parent
                            .asm
                            .instructions
                            .get(parent.pc.saturating_sub(1))
                            .map(|i| (i.source_cmd_text.clone(), i.source_line))
                    }) {
                        self.log_command_info(&cmd, "", line);
                    }
                }
            }
            if act.is_proc {
                self.unwind_proc_frame(&act, acts, &mut c);
            }
            // Settle any execution-trace leave contexts riding on this frame
            // (M16.3) — a leave-trace error replaces the completion
            // (tclsh-pinned).  For a traced `catch` this fires with the
            // pre-absorb completion (the body's), a documented divergence.
            if !act.exec_leave.is_empty() {
                for ctx in act.exec_leave.drain(..).rev() {
                    if let Some(replacement) = self.finish_exec_leave(&ctx, &c) {
                        c = replacement;
                    }
                }
            }
            // `apply`'s temporary lambda proc is torn down here, once its script
            // activation completes — on every completion code, mirroring the old
            // nested-drive `cmd_apply`'s unconditional `vm.take_command` after
            // `eval_source` returned (issue #1311).
            if let Some(name) = act.cleanup_proc.take() {
                self.take_command_unchecked(&name);
            }
            // A `catch` activation absorbs the body's completion of *any* code:
            // its epilogue binds the result / options variables and yields the
            // status code as an integer, delivered to the parent as this `catch`
            // command's result (`exit` stays uncatchable — `finish_catch` passes
            // it straight through).
            if let Some(ctx) = act.catch.take() {
                let fc = self.finish_catch(c, ctx.resvar.as_ref(), ctx.optvar.as_ref());
                match acts.last_mut() {
                    None => return Some(fc),
                    Some(parent) => {
                        if fc.code.is_ok() {
                            parent.stack.push(fc.result);
                            return None;
                        }
                        // A rare `set`-into-the-result-var failure (or an
                        // uncatchable `exit`) keeps unwinding.
                        c = fc;
                        continue;
                    }
                }
            }
            // A try-phase activation (body/handler/`finally`) just completed:
            // `advance_try` decides whether another phase follows (pushed here,
            // directly — `act` itself is already gone, unlike `each_loop`'s
            // parent-stays-put pattern) or the whole `try` is done, in which
            // case `c` becomes its completion and unwinding continues below
            // exactly as for any other completed activation (issue #1311).
            if let Some(ctx) = act.try_ctx.take() {
                match crate::cmd_try::advance_try(self, *ctx, c) {
                    crate::cmd_try::TryOutcome::Push(req) => {
                        acts.push(Frame::new_try(req, self.profile_generation()));
                        return None;
                    }
                    crate::cmd_try::TryOutcome::Deliver(fc) => c = fc,
                }
            }
            // A subst activation's `[…]` child (`act`) just completed: fold its
            // result into the enclosing subst frame's scan by subst rules (see
            // [`Vm::fold_subst_bracket`]). `Resume` re-ticks the subst frame;
            // `Unwind` drops it and keeps unwinding (a `break`'s output / an error).
            if acts.last().is_some_and(|p| p.subst.is_some()) {
                let parent = acts.last_mut().expect("subst parent present");
                match Self::fold_subst_bracket(parent, c) {
                    SubstFold::Resume => return None,
                    SubstFold::Unwind(nc) => {
                        c = nc;
                        continue;
                    }
                }
            }
            // An `each_loop` (`foreach`/`lmap` runtime-fallback) activation's body
            // child (`act`) just completed: fold its result into the enclosing
            // loop's iteration state (issue #1311 — this is what makes a
            // value-consumed `lmap`, e.g. `set r [lmap x {1 2} { yield $x }]`,
            // yieldable). `Resume` re-ticks the loop frame for the next iteration
            // (or its final result, once exhausted); `Unwind` drops it and keeps
            // unwinding (an error, or an uncaught `return`).
            if acts.last().is_some_and(|p| p.each_loop.is_some()) {
                let parent = acts.last_mut().expect("each_loop parent present");
                match self.fold_each_loop(parent, c) {
                    EachLoopFold::Resume => return None,
                    EachLoopFold::Unwind(nc) => {
                        c = nc;
                        continue;
                    }
                }
            }
            match acts.last_mut() {
                None => return Some(c),
                Some(parent) => {
                    if c.code.is_ok() {
                        parent.stack.push(c.result);
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

    /// Unwind one proc activation `act`: on error add its `(procedure "name" line
    /// N)` errorInfo frame, pop the call-frame + namespace, apply
    /// `TclUpdateReturnInfo` (a proc boundary decrements a carried `-code`/`-level`
    /// return), and on error log the caller's `invoked from
    /// within "…"` frame. Mutates `c` in place. Split out of [`Vm::unwind`].
    fn unwind_proc_frame(&mut self, act: &Frame, acts: &[Frame], c: &mut Completion<Value>) {
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
        if c.code == Code::Return {
            // While the carried return level stays positive the TCL_RETURN keeps
            // unwinding one proc level at a time; when it reaches 0 the carried
            // `-code` takes effect. `Code::from_int` (never a 0..=4-only map):
            // `return -code N` for a non-standard N must surface `Code::Other(N)`,
            // not collapse to `Ok` (coroutine-2.4).
            let level = crate::command::opt_get(&c.options, "-level")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(1);
            if level > 1 {
                c.options = crate::command::with_return_level(&c.options, level - 1);
            } else {
                let code = crate::command::opt_get(&c.options, "-code")
                    .and_then(|v| v.as_int().ok())
                    .and_then(|n| i32::try_from(n).ok())
                    .map_or(Code::Ok, Code::from_int);
                c.options = crate::command::with_return_level(&c.options, 0);
                c.code = code;
            }
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
    fn fold_subst_bracket(parent: &mut Frame, c: Completion<Value>) -> SubstFold {
        match c.code {
            Code::Ok | Code::Return | Code::Other(_) => {
                let v = c.result.to_str();
                parent.subst.as_mut().expect("subst state").out.push_str(&v);
                SubstFold::Resume
            }
            Code::Continue => SubstFold::Resume,
            Code::Break => {
                let out = std::mem::take(&mut parent.subst.as_mut().expect("subst state").out);
                parent.subst = None;
                SubstFold::Unwind(ok(Value::string(out)))
            }
            Code::Error => {
                parent.subst = None;
                SubstFold::Unwind(c)
            }
        }
    }

    /// Push a call-frame and bind `argv` to the proc's parameters.
    fn enter_proc(&mut self, proc: &ProcDef, argv: &[Value]) -> Result<(), Completion<Value>> {
        // Recursion bound (catchable, not a host stack overflow). Checked before
        // the frame push, matching C's `interp recursionlimit`.
        if self.recursion_depth() >= self.recursion_limit() {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        let simple = crate::interp::key_holder_and_tail_unrooted(&proc.name).1;
        let mut call_argv = Vec::with_capacity(argv.len() + 1);
        call_argv.push(Value::string(simple));
        call_argv.extend(argv.iter().cloned());
        self.push_call_frame(Some(proc.name.clone()), call_argv);

        let mut i = 0;
        let n = proc.params.len();
        for (idx, p) in proc.params.iter().enumerate() {
            if proc.has_args && idx == n - 1 {
                let rest: Vec<Value> = argv.get(i..).unwrap_or(&[]).to_vec();
                self.set_local("args", Value::list(rest));
                i = argv.len();
            } else if i < argv.len() {
                self.set_local(&p.name, argv[i].clone());
                i += 1;
            } else if let Some(d) = &p.default {
                self.set_local(&p.name, d.clone());
            } else {
                self.pop_call_frame();
                return Err(err(proc_usage(proc)));
            }
        }
        if i < argv.len() {
            self.pop_call_frame();
            return Err(err(proc_usage(proc)));
        }
        // The body resolves names in the proc's defining namespace. Pushed only
        // on success; `unwind` pops it together with the call-frame.
        self.push_ns(proc.namespace.clone());
        Ok(())
    }

    /// Unset the array element `name(key)`, honouring C's `TCL_LEAVE_ERR_MSG`
    /// (`complain`). Shared by `INST_UNSET_ARRAY` (array in an LVT slot,
    /// `tclExecute.c:3813-3864`) and `INST_UNSET_ARRAY_STK` (array name on the
    /// stack, `3866-3872`): the first reaches the message through
    /// `slowUnsetArray` → `TclLookupArrayElement(…, flags, "unset", …)` →
    /// `errorInUnset`, the second through `TclObjUnsetVar2(part1, part2, flags)`,
    /// and both land on the same `tclVar.c` three-way text — `NOSUCHELEMENT`
    /// (119-122: "no such element in array") / `NEEDARRAY` ("variable isn't
    /// array") / `NOSUCHVAR` ("no such variable"). With the flag clear neither
    /// form reports anything (C's early `NEXT_INST_F(6, 1, 0)` at 3843-3848).
    fn unset_array_elem_checked(
        &mut self,
        name: &str,
        key: &str,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        if complain && self.get_array_elem(name, key).is_none() {
            let what = if self.var_is_array(name) {
                "no such element in array"
            } else if self.exists_var(name) {
                "variable isn't array"
            } else {
                "no such variable"
            };
            return Err(err(format!("can't unset \"{name}({key})\": {what}")));
        }
        self.array_unset_elem(name, key);
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
            crate::subst::SubstStep::Done(out) => Tick::Return(ok(Value::string(out))),
            crate::subst::SubstStep::Error(msg) => Tick::Return(err(msg)),
            crate::subst::SubstStep::Bracket(inner) => match self.compile_source_cached(&inner) {
                Ok(asm) => Tick::PushScript {
                    asm,
                    label: None,
                    cleanup_proc: None,
                },
                Err(e) => Tick::Return(err(e.message)),
            },
        }
    }

    /// One driver step of an each-loop activation: bind the next iteration's
    /// loop variables (Rust-side — the synchronous `each_loop` this replaces
    /// can't yield here either, since it's plain `Vm::set_var`) and push its
    /// body as a yieldable child script frame, or — once every iteration has
    /// run (or a `break` set `it` to `iterations` early, see
    /// [`Vm::fold_each_loop`]) — return the collected/empty result.
    fn tick_each_loop(&mut self, f: &mut Frame) -> Tick {
        let done = {
            let st = f.each_loop.as_ref().expect("each_loop frame carries state");
            st.it >= st.iterations
        };
        if done {
            let st = f.each_loop.take().expect("each_loop frame carries state");
            return Tick::Return(ok(if st.collect {
                Value::list(st.collected)
            } else {
                Value::empty()
            }));
        }
        let body = {
            let st = f.each_loop.as_mut().expect("each_loop frame carries state");
            for group in &st.groups {
                for (k, var) in group.vars.iter().enumerate() {
                    let val = group
                        .values
                        .get(st.it * group.vars.len() + k)
                        .cloned()
                        .unwrap_or_else(Value::empty);
                    if let Err(c) = self.set_var(var, val) {
                        return Tick::Return(c);
                    }
                }
            }
            st.it += 1;
            Rc::clone(&st.body)
        };
        Tick::PushScript {
            asm: body,
            label: None,
            cleanup_proc: None,
        }
    }

    /// Fold an each-loop body child's completion into the loop's iteration
    /// state, matching the synchronous engine this replaces exactly: `Ok`
    /// collects the result (`lmap`) and continues; `Continue` skips collection
    /// and continues; `Break` stops the loop (delivered as the final result on
    /// the next tick, since `it` is set to `iterations`); `Error` adds the
    /// `each_loop`'s own body-frame label (`vm.append_body_frame(name)`) and
    /// unwinds; `Return`/`Other` propagate immediately, uncollected, exactly as
    /// C Tcl's `EachloopCmd` passes a body `return` straight out of the loop.
    fn fold_each_loop(&mut self, parent: &mut Frame, c: Completion<Value>) -> EachLoopFold {
        if matches!(c.code, Code::Error | Code::Return | Code::Other(_)) {
            let name = parent.each_loop.as_ref().expect("each_loop state").name;
            parent.each_loop = None;
            if c.code == Code::Error {
                self.append_body_frame(name);
            }
            return EachLoopFold::Unwind(c);
        }
        let st = parent.each_loop.as_mut().expect("each_loop state");
        match c.code {
            Code::Ok => {
                if st.collect {
                    st.collected.push(c.result);
                }
            }
            Code::Continue => {}
            Code::Break => st.it = st.iterations,
            Code::Error | Code::Return | Code::Other(_) => unreachable!("handled above"),
        }
        EachLoopFold::Resume
    }

    /// Execute a single instruction of the top activation.
    #[allow(clippy::too_many_lines)] // One match over every opcode; the VM's central dispatch is clearer whole.
    fn tick(&mut self, f: &mut Frame) -> Tick {
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
        let asm = Rc::clone(&f.asm);
        if f.pc >= asm.instructions.len() {
            return Tick::Return(ok(f
                .stack
                .last()
                .cloned()
                .unwrap_or_else(|| f.last_result.clone())));
        }
        let instr = &asm.instructions[f.pc];
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

        macro_rules! try_op {
            ($e:expr) => {
                if let Err(c) = $e {
                    return Tick::Return(c);
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
                if let Err(c) = $e {
                    let cmd_text = instr.source_cmd_text.clone();
                    let line = instr.source_line;
                    let msg = c.result.to_str().to_string();
                    self.log_command_info(&cmd_text, &msg, line);
                    return Tick::Return(c);
                }
            };
        }

        let lvt_name = |slot: i32| -> String {
            lvt.get(usize::try_from(slot).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default()
        };

        match instr.op {
            // -- stack --
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
                    f.stack.push(Value::string(raw));
                } else {
                    match crate::subst::subst_word(&raw, self) {
                        Ok(v) => f.stack.push(v),
                        // A `break`/`continue`/`return` carried out of a `[…]`
                        // substitution propagates with its own code (an enclosing
                        // loop's exception range / a proc boundary handles it).
                        Err(e) => {
                            return Tick::Return(Completion::new(
                                e.code.unwrap_or(Code::Error),
                                Value::string(e.message),
                                Value::empty(),
                            ));
                        }
                    }
                }
            }
            Op::POP => {
                if let Some(v) = f.stack.pop() {
                    f.last_result = v;
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
                let mut s = String::new();
                for p in &parts {
                    s.push_str(&p.to_str());
                }
                f.stack.push(Value::string(s));
            }
            // `START_CMD`/`NOP` are inert.
            Op::NOP | Op::START_CMD => {}

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
                f.catch_ranges.pop();
            }

            // -- variables (stack form, by name) --
            // The `*ScalarStk` opcodes share their C `TEBCresume` case with the
            // general `*Stk` form (the compiler emits them when the name is known
            // to carry no `(index)` part), so they are the same arm here.
            Op::LOAD_STK | Op::LOAD_SCALAR_STK => {
                let name = pop(f).to_str();
                try_op!(self.fire_var_traces(&name, "read"));
                match self.get_var(&name) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}\": no such variable"
                        )));
                    }
                }
            }
            Op::STORE_STK | Op::STORE_SCALAR_STK => {
                let value = pop(f);
                let name = pop(f).to_str();
                try_cmd!(self.set_var(&name, value.clone()));
                f.stack.push(value);
            }
            Op::INCR_STK_IMM | Op::INCR_SCALAR_STK_IMM => {
                let name = pop(f).to_str();
                try_op!(self.incr_var(f, &name, i64::from(imm0(instr))));
            }
            Op::INCR_STK | Op::INCR_SCALAR_STK => {
                let amount = pop(f);
                let name = pop(f).to_str();
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &name, a)),
                    Err(e) => return Tick::Return(crate::command::completion_from_tcl_error(e)),
                }
            }

            // -- variables (LVT form, proc bodies) --
            Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                try_op!(self.fire_var_traces(&name, "read"));
                match self.get_var(&name) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}\": no such variable"
                        )));
                    }
                }
            }
            Op::STORE_SCALAR1 | Op::STORE_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                try_op!(self.set_var(&name, value.clone()));
                f.stack.push(value);
            }
            Op::INCR_SCALAR1 => {
                let name = lvt_name(imm0(instr));
                let amount = pop(f);
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &name, a)),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::INCR_SCALAR1_IMM => {
                let name = lvt_name(imm0(instr));
                try_op!(self.incr_var(f, &name, i64::from(imm_at(instr, 1))));
            }
            Op::INCR_ARRAY_STK_IMM => {
                // Stack: [name, key]; operand is the immediate increment.
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                try_op!(self.incr_var(f, &format!("{name}({key})"), i64::from(imm0(instr))));
            }
            Op::INCR_ARRAY_STK => {
                // Stack: [name, key, amount].
                let amount = pop(f);
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &format!("{name}({key})"), a)),
                    Err(e) => return Tick::Return(crate::command::completion_from_tcl_error(e)),
                }
            }
            Op::INCR_ARRAY1 => {
                // Operand is the array's slot; stack: [key, amount].
                let name = lvt_name(imm0(instr));
                let amount = pop(f);
                let key = pop(f).to_str();
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &format!("{name}({key})"), a)),
                    Err(e) => return Tick::Return(crate::command::completion_from_tcl_error(e)),
                }
            }
            Op::INCR_ARRAY1_IMM => {
                // Operands [slot, increment]; the element key is on the stack.
                let name = lvt_name(imm0(instr));
                let key = pop(f).to_str();
                try_op!(self.incr_var(f, &format!("{name}({key})"), i64::from(imm_at(instr, 1))));
            }
            Op::EXIST_SCALAR => {
                // The slot name may be an `arr(key)` element reference baked
                // into the LVT (the codegen names the slot `a(x)`), so resolve
                // it element-aware.
                let name = lvt_name(imm0(instr));
                // `info exists` fires read traces (a trace may create the
                // variable); a trace error does not abort the existence check.
                let _ = self.fire_var_traces(&name, "read");
                f.stack.push(Value::bool(self.exists_var(&name)));
            }
            Op::EXIST_STK => {
                let name = pop(f).to_str();
                let _ = self.fire_var_traces(&name, "read");
                f.stack.push(Value::bool(self.exists_var(&name)));
            }
            // The array-element existence tests fire the element's read traces
            // first (C `INST_EXIST_ARRAY`: a trace may create it) and never
            // error, exactly like their scalar siblings above.
            Op::EXIST_ARRAY => {
                let name = lvt_name(imm0(instr));
                let key = pop(f).to_str();
                let full = format!("{name}({key})");
                let _ = self.fire_var_traces(&full, "read");
                f.stack.push(Value::bool(self.exists_var(&full)));
            }
            Op::EXIST_ARRAY_STK => {
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                let full = format!("{name}({key})");
                let _ = self.fire_var_traces(&full, "read");
                f.stack.push(Value::bool(self.exists_var(&full)));
            }

            // -- arrays --
            Op::LOAD_ARRAY_STK => {
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                try_op!(self.fire_var_traces(&format!("{name}({key})"), "read"));
                match self.get_array_elem(&name, &key) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}({key})\": no such element in array"
                        )));
                    }
                }
            }
            Op::STORE_ARRAY_STK => {
                let value = pop(f);
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                if let Err(e) = self.set_array_elem(&name, &key, value.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(value);
            }
            Op::LOAD_ARRAY1 | Op::LOAD_ARRAY4 => {
                let name = lvt_name(imm0(instr));
                let key = pop(f).to_str();
                try_op!(self.fire_var_traces(&format!("{name}({key})"), "read"));
                match self.get_array_elem(&name, &key) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}({key})\": no such element in array"
                        )));
                    }
                }
            }
            Op::STORE_ARRAY1 | Op::STORE_ARRAY4 => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f).to_str();
                if let Err(e) = self.set_array_elem(&name, &key, value.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(value);
            }
            Op::ARRAY_EXISTS_IMM => {
                let name = lvt_name(imm0(instr));
                f.stack.push(Value::bool(self.array_is(&name)));
            }
            Op::ARRAY_EXISTS_STK => {
                // The stack form resolves an arbitrary (possibly qualified) name,
                // so it honours the namespace-variable fallback `array_is` skips.
                let name = pop(f).to_str();
                f.stack.push(Value::bool(self.var_is_array(&name)));
            }
            // `array set`'s materialising half (C `INST_ARRAY_MAKE_*`): make the
            // variable an empty array when undefined, no-op when it already is
            // one, and error on a scalar or an array element.
            Op::ARRAY_MAKE_IMM => {
                let name = lvt_name(imm0(instr));
                try_op!(self.ensure_array(&name));
            }
            Op::ARRAY_MAKE_STK => {
                let name = pop(f).to_str();
                // The element-reference test comes from the one owner rather
                // than a local re-spelling of its predicate (issue #1458).
                if tcl_syntax::naming::split_element_ref(&name).is_some() {
                    return Tick::Return(err(format!(
                        "can't array set \"{name}\": variable isn't array"
                    )));
                }
                try_op!(self.ensure_array(&name));
            }

            // -- lists (inline opcodes) --
            Op::LIST => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("list: stack underflow"));
                }
                let items = f.stack.split_off(f.stack.len() - n);
                f.stack.push(Value::list(items));
            }
            Op::LIST_LENGTH => {
                let l = pop(f);
                match l.as_list() {
                    Ok(items) => f.stack.push(Value::int(ilen(items.len()))),
                    Err(e) => return Tick::Return(err(e.message)),
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
                match tcl_cmd_core::list::lindex(self, &l, std::slice::from_ref(&idx)) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => return Tick::Return(err(e.into_message())),
                }
            }
            Op::LIST_INDEX_IMM => {
                let l = pop(f);
                let items = match l.as_list() {
                    Ok(i) => i,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let i = imm_index(imm0(instr), items.len());
                f.stack.push(get_at(&items, i));
            }
            Op::LIST_RANGE_IMM => {
                let l = pop(f);
                let items = match l.as_list() {
                    Ok(i) => i,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let lo = imm_index(imm0(instr), items.len()).max(0);
                let hi = imm_index(imm_at(instr, 1), items.len());
                f.stack.push(slice(&items, lo, hi));
            }
            Op::LIST_IN | Op::LIST_NOT_IN => {
                let list = pop(f);
                let needle = pop(f);
                let items = match list.as_list() {
                    Ok(i) => i,
                    Err(e) => return Tick::Return(err(e.message)),
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
                let name = pop(f).to_str();
                try_op!(self.unset_one(&name, complain));
            }
            Op::UNSET_SCALAR => {
                // Operands [flags, slot]; flags bit 0 ⇒ complain when absent.
                let complain = imm0(instr) != 0;
                let name = lvt_name(imm_at(instr, 1));
                try_op!(self.unset_one(&name, complain));
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
                let name = lvt_name(imm_at(instr, 1));
                let key = pop(f).to_str();
                try_op!(self.unset_array_elem_checked(&name, &key, complain));
            }
            Op::UNSET_ARRAY_STK => {
                // Operand = flags (non-zero ⇒ C's `TCL_LEAVE_ERR_MSG`: complain
                // when the element is absent); the key is on TOS over the array
                // name, matching C's `part2Ptr = OBJ_AT_TOS` /
                // `part1Ptr = OBJ_UNDER_TOS` (`tclExecute.c:3866-3872`).
                let complain = imm0(instr) != 0;
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                try_op!(self.unset_array_elem_checked(&name, &key, complain));
            }

            // -- append / lappend (LVT scalar + array forms) --
            Op::APPEND_SCALAR1 | Op::APPEND_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                let v = pop(f);
                let mut s = self
                    .get_var(&name)
                    .map(|x| x.to_str().to_string())
                    .unwrap_or_default();
                s.push_str(&v.to_str());
                let nv = Value::string(s);
                try_op!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }
            Op::APPEND_ARRAY1 | Op::APPEND_ARRAY4 => {
                let name = lvt_name(imm0(instr));
                let v = pop(f);
                let key = pop(f).to_str();
                let mut s = self
                    .get_array_elem(&name, &key)
                    .map(|x| x.to_str().to_string())
                    .unwrap_or_default();
                s.push_str(&v.to_str());
                let nv = Value::string(s);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }
            Op::LAPPEND_SCALAR1 | Op::LAPPEND_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                let v = pop(f);
                let mut items = match self.get_var(&name) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.push(v);
                let nv = Value::list(items);
                try_op!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }
            Op::LAPPEND_ARRAY1 | Op::LAPPEND_ARRAY4 => {
                let name = lvt_name(imm0(instr));
                let v = pop(f);
                let key = pop(f).to_str();
                let mut items = match self.get_array_elem(&name, &key) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.push(v);
                let nv = Value::list(items);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }
            Op::LAPPEND_LIST => {
                // Append each element of the popped list to the scalar var's list.
                let name = lvt_name(imm0(instr));
                let add = pop(f);
                let add_items = match add.as_list() {
                    Ok(l) => l,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let mut items = match self.get_var(&name) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.extend(add_items.iter().cloned());
                let nv = Value::list(items);
                try_op!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }

            // -- append / lappend (stack form, by name) --
            // `set_var`/`get_var` resolve an `a(k)` name to the element, exactly
            // as C's `TclObjLookupVarEx(part1, NULL)` parses the name.
            Op::APPEND_STK => {
                let v = pop(f);
                let name = pop(f).to_str();
                let mut s = self
                    .get_var(&name)
                    .map(|x| x.to_str().to_string())
                    .unwrap_or_default();
                s.push_str(&v.to_str());
                let nv = Value::string(s);
                try_cmd!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }
            Op::LAPPEND_STK => {
                let v = pop(f);
                let name = pop(f).to_str();
                let mut items = match self.get_var(&name) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.push(v);
                let nv = Value::list(items);
                try_cmd!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }
            Op::APPEND_ARRAY_STK => {
                let v = pop(f);
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                let mut s = self
                    .get_array_elem(&name, &key)
                    .map(|x| x.to_str().to_string())
                    .unwrap_or_default();
                s.push_str(&v.to_str());
                let nv = Value::string(s);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }
            Op::LAPPEND_ARRAY_STK => {
                let v = pop(f);
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                let mut items = match self.get_array_elem(&name, &key) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.push(v);
                let nv = Value::list(items);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }
            // The `lappendList*` family appends *each* element of the popped list
            // (`lappend v {*}$list`); an unparsable list errors before any write.
            Op::LAPPEND_LIST_STK => {
                let add = pop(f);
                let name = pop(f).to_str();
                let add_items = match add.as_list() {
                    Ok(l) => l,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let mut items = match self.get_var(&name) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.extend(add_items.iter().cloned());
                let nv = Value::list(items);
                try_cmd!(self.set_var(&name, nv.clone()));
                f.stack.push(nv);
            }
            Op::LAPPEND_LIST_ARRAY => {
                let name = lvt_name(imm0(instr));
                let add = pop(f);
                let key = pop(f).to_str();
                let add_items = match add.as_list() {
                    Ok(l) => l,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let mut items = match self.get_array_elem(&name, &key) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.extend(add_items.iter().cloned());
                let nv = Value::list(items);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }
            Op::LAPPEND_LIST_ARRAY_STK => {
                let add = pop(f);
                let key = pop(f).to_str();
                let name = pop(f).to_str();
                let add_items = match add.as_list() {
                    Ok(l) => l,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let mut items = match self.get_array_elem(&name, &key) {
                    Some(cur) => match cur.as_list() {
                        Ok(l) => (*l).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => Vec::new(),
                };
                items.extend(add_items.iter().cloned());
                let nv = Value::list(items);
                if let Err(e) = self.set_array_elem(&name, &key, nv.clone()) {
                    return Tick::Return(e);
                }
                f.stack.push(nv);
            }

            // -- const (TIP 677) --
            // `constImm`/`constStk` reuse the `const` command's core so the
            // silent re-`const` no-op and the three error messages
            // (`can't make constant "n": …`) stay in one place.
            Op::CONST_IMM => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let c = crate::command::cmd_const(self, &[Value::string(name), value]);
                if !c.code.is_ok() {
                    return Tick::Return(c);
                }
            }
            Op::CONST_STK => {
                let value = pop(f);
                let name = pop(f);
                let c = crate::command::cmd_const(self, &[name, value]);
                if !c.code.is_ok() {
                    return Tick::Return(c);
                }
            }

            // -- global / upvar links. The namespace ("::") or level reference
            // is left on the stack for the next link op to reuse; a trailing
            // `pop` discards it (matching C Tcl's nsupvar/upvar codegen). --
            Op::NSUPVAR => {
                let local = lvt_name(imm0(instr));
                let var = pop(f).to_str();
                let ns = f
                    .stack
                    .last()
                    .map(|v| v.to_str().to_string())
                    .unwrap_or_default();
                let target = if ns == "::" || ns.is_empty() {
                    var.to_string()
                } else {
                    format!("{}::{}", ns.trim_end_matches("::"), var)
                };
                // `global`/namespace links resolve in the global frame.
                self.add_link(&local, 0, &target);
            }
            Op::UPVAR => {
                let local = lvt_name(imm0(instr));
                let other = pop(f).to_str();
                let spec = f
                    .stack
                    .last()
                    .map(|v| v.to_str().to_string())
                    .unwrap_or_default();
                let target_level = if let Some(abs) = spec.strip_prefix('#') {
                    abs.parse::<usize>().unwrap_or(0)
                } else if !spec.is_empty() && spec.bytes().all(|b| b.is_ascii_digit()) {
                    self.current_level()
                        .saturating_sub(spec.parse::<usize>().unwrap_or(1))
                } else {
                    self.current_level().saturating_sub(1)
                };
                if let Err(error) = self.link_upvar(target_level, &other, &local) {
                    return Tick::Return(crate::command::upvar_link_error(error, &other, &local));
                }
            }
            // `variable` (C `INST_VARIABLE`): link the local slot to the
            // namespace variable named by the popped name. The link machinery is
            // the `variable` command's (`cmd_variable`) — namespace variables
            // live in the global frame under their canonical qualified name.
            Op::VARIABLE => {
                let local = lvt_name(imm0(instr));
                let var = pop(f).to_str();
                let target = self.qualify_name(&var);
                self.add_link(&local, 0, &target);
            }

            // -- concat (stack form): Tcl-concat the top N values. --
            Op::CONCAT_STK => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                let take = f.stack.len().saturating_sub(n);
                let vals: Vec<Value> = f.stack.split_off(take);
                // Backslash-aware trim per element (C `Tcl_ConcatObj`), shared
                // with the `concat` builtin so `concat "a\ " b` keeps the
                // escaped trailing space.
                let joined = vals
                    .iter()
                    .map(Value::to_str)
                    .filter_map(|s| {
                        let t = tcl_cmd_core::list::trim_concat_element(&s);
                        (!t.is_empty()).then(|| t.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                f.stack.push(Value::string(joined));
            }
            Op::LIST_CONCAT => {
                let b = pop(f);
                let a = pop(f);
                match (a.as_list(), b.as_list()) {
                    (Ok(ai), Ok(bi)) => {
                        let mut v = (*ai).clone();
                        v.extend(bi.iter().cloned());
                        f.stack.push(Value::list(v));
                    }
                    (Err(e), _) | (_, Err(e)) => return Tick::Return(err(e.message)),
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
                            Err(e) => return Tick::Return(err(e.message)),
                        };
                        let n = items.len();
                        let nlen = isize::try_from(n).unwrap_or(isize::MAX);
                        if mode == 1 {
                            // lreplace: list first last ?elem ...?
                            let first = match vals.get(1) {
                                Some(v) => match checked_index_value(v, n) {
                                    Ok(i) => i.max(0),
                                    Err(c) => return Tick::Return(c),
                                },
                                None => 0,
                            };
                            let last = match vals.get(2) {
                                Some(v) => match checked_index_value(v, n) {
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
                                Some(v) => match checked_index_value(v, n + 1) {
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
                for (gi, lv) in raw.iter().enumerate() {
                    let items = match lv.as_list() {
                        Ok(x) => (*x).clone(),
                        Err(e) => return Tick::Return(err(e.message)),
                    };
                    let nv = groups[gi].len().max(1);
                    iter_max = iter_max.max(items.len().div_ceil(nv));
                    value_lists.push(items);
                }
                f.foreach_stack.push(ForeachState {
                    var_groups: groups,
                    lists: value_lists,
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
                let (binds, body, more) = {
                    match f.foreach_stack.last_mut() {
                        None => (Vec::new(), 0usize, false),
                        Some(st) if st.iter_num >= st.iter_max => (Vec::new(), 0, false),
                        Some(st) => {
                            let it = st.iter_num;
                            let mut binds: Vec<(String, Value)> = Vec::new();
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
                    for (name, v) in binds {
                        try_op!(self.set_var(&name, v));
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
                    st.accum.push(v);
                }
            }
            Op::FOREACH_END => {
                let st = f.foreach_stack.pop();
                // A collecting loop (`lmap`) yields `list(accum)`; a plain
                // `foreach` yields nothing (its `""` result is pushed by the
                // loop-end block).
                if let Some(st) = st
                    && st.collect
                {
                    f.stack.push(Value::list(st.accum));
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
                let ps = match self.dict_pairs(&dict) {
                    Ok(p) => p
                        .into_iter()
                        .map(|(k, v)| (Value::string(k), v))
                        .collect::<Vec<(Value, Value)>>(),
                    Err(c) => return Tick::Return(c),
                };
                let (k, v) = ps
                    .first()
                    .cloned()
                    .unwrap_or_else(|| (Value::empty(), Value::empty()));
                let done = ps.is_empty();
                // An empty dict skips straight to the loop exit (`jumpTrue`), so
                // `dictNext` never runs to reclaim the slot — don't store state
                // for it. Drop any stale iterator left in the slot.
                if done {
                    f.dict_iters.remove(&slot);
                } else {
                    f.dict_iters
                        .insert(slot, DictIterState { pairs: ps, pos: 0 });
                }
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
            }
            Op::DICT_NEXT => {
                let slot = imm0(instr);
                let (k, v, done) = match f.dict_iters.get_mut(&slot) {
                    Some(st) => {
                        st.pos += 1;
                        match st.pairs.get(st.pos) {
                            Some((k, v)) => (k.clone(), v.clone(), false),
                            None => (Value::empty(), Value::empty(), true),
                        }
                    }
                    None => (Value::empty(), Value::empty(), true),
                };
                // The loop exits once exhausted, so free the (key, value) vector
                // now rather than retaining it until the frame returns.
                if done {
                    f.dict_iters.remove(&slot);
                }
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
            }
            Op::DICT_UPDATE_START => {
                // Prologue of compiled `dict update`: read the dict var and copy
                // each keyed value into the matching target local (unsetting it
                // when the key is absent). The key list stays on the stack for
                // the paired `dictUpdateEnd`.
                let dict_name = lvt_name(imm0(instr));
                let vars = instr.dict_vars.clone().unwrap_or_default();
                let keys: Vec<String> = match f.stack.last() {
                    Some(v) => match v.as_list() {
                        Ok(l) => l.iter().map(|e| e.to_str().to_string()).collect(),
                        Err(e) => return Tick::Return(err(e.message)),
                    },
                    None => return Tick::Return(err("dictUpdateStart: stack underflow")),
                };
                let Some(dict) = self.get_var(&dict_name) else {
                    return Tick::Return(err(format!(
                        "can't read \"{dict_name}\": no such variable"
                    )));
                };
                let ps = match self.dict_pairs(&dict) {
                    Ok(p) => p,
                    Err(c) => return Tick::Return(c),
                };
                for (i, key) in keys.iter().enumerate() {
                    let Some(var) = vars.get(i) else { break };
                    match ps.iter().find(|(k, _)| k == key) {
                        Some((_, val)) => try_op!(self.set_var(var, val.clone())),
                        None => {
                            let _ = self.unset_one(var, false);
                        }
                    }
                }
            }
            Op::DICT_UPDATE_END => {
                // Epilogue of compiled `dict update`: pop the key list and write
                // each target local back into the dict var under its key (or
                // remove the key when the local was unset).
                let dict_name = lvt_name(imm0(instr));
                let vars = instr.dict_vars.clone().unwrap_or_default();
                let keys: Vec<String> = match pop(f).as_list() {
                    Ok(l) => l.iter().map(|e| e.to_str().to_string()).collect(),
                    Err(e) => return Tick::Return(err(e.message)),
                };
                let cur = self.get_var(&dict_name).unwrap_or_else(Value::empty);
                let mut ps = match self.dict_pairs(&cur) {
                    Ok(p) => p,
                    Err(c) => return Tick::Return(c),
                };
                for (i, key) in keys.iter().enumerate() {
                    let Some(var) = vars.get(i) else { break };
                    match self.get_var(var) {
                        Some(val) => {
                            if let Some(slot) = ps.iter_mut().find(|(k, _)| k == key) {
                                slot.1 = val;
                            } else {
                                ps.push((key.clone(), val));
                            }
                        }
                        None => ps.retain(|(k, _)| k != key),
                    }
                }
                try_op!(self.set_var(&dict_name, dict_from_pairs(&ps)));
            }
            Op::DICT_EXPAND => {
                // Prologue of compiled `dict with`: expand every key of the dict
                // (top-of-stack path selects a sub-dict; the inline form always
                // passes an empty path = the whole dict) into a same-named local,
                // pushing the snapshot dict as the recombine state.
                let _path = pop(f);
                let dict = pop(f);
                let ps = match self.dict_pairs(&dict) {
                    Ok(p) => p,
                    Err(c) => return Tick::Return(c),
                };
                for (k, v) in &ps {
                    try_op!(self.set_var(k, v.clone()));
                }
                f.stack.push(dict);
            }
            Op::DICT_RECOMBINE_IMM | Op::DICT_RECOMBINE_STK => {
                // Epilogue of compiled `dict with`: write each snapshot key's
                // local back into the dict var (removing keys whose local was
                // unset). The `Stk` form takes the variable name from the stack,
                // deepest of `varName path state` (C `INST_DICT_RECOMBINE_STK`
                // pops the state first, then reads `OBJ_UNDER_TOS`/`OBJ_AT_TOS`).
                let state = pop(f);
                let _path = pop(f);
                let dict_name = if instr.op == Op::DICT_RECOMBINE_STK {
                    pop(f).to_str().to_string()
                } else {
                    lvt_name(imm0(instr))
                };
                let keys: Vec<String> = match self.dict_pairs(&state) {
                    Ok(p) => p.into_iter().map(|(k, _)| k).collect(),
                    Err(c) => return Tick::Return(c),
                };
                let cur = self.get_var(&dict_name).unwrap_or_else(Value::empty);
                let mut ps = match self.dict_pairs(&cur) {
                    Ok(p) => p,
                    Err(c) => return Tick::Return(c),
                };
                for key in &keys {
                    match self.get_var(key) {
                        Some(val) => {
                            if let Some(slot) = ps.iter_mut().find(|(k, _)| k == key) {
                                slot.1 = val;
                            } else {
                                ps.push((key.clone(), val));
                            }
                        }
                        None => ps.retain(|(k, _)| k != key),
                    }
                }
                try_op!(self.set_var(&dict_name, dict_from_pairs(&ps)));
            }

            // -- dict validation: consumes the (dup'd) TOS, validates even length --
            Op::VERIFY_DICT => {
                let top = pop(f);
                match top.as_list() {
                    Ok(items) if items.len() % 2 == 0 => {}
                    Ok(_) => {
                        return Tick::Return(err("missing value to go with key"));
                    }
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }

            // -- arithmetic / bitwise / shift --
            Op::ADD => try_op!(bin(f, BinOp::Add)),
            Op::SUB => try_op!(bin(f, BinOp::Sub)),
            Op::MULT => try_op!(bin(f, BinOp::Mul)),
            Op::DIV => try_op!(bin(f, BinOp::Div)),
            Op::MOD => try_op!(bin(f, BinOp::Mod)),
            Op::EXPON => try_op!(bin(f, BinOp::Pow)),
            Op::LSHIFT => try_op!(bin(f, BinOp::LShift)),
            Op::RSHIFT => try_op!(bin(f, BinOp::RShift)),
            Op::BITAND => try_op!(bin(f, BinOp::BitAnd)),
            Op::BITOR => try_op!(bin(f, BinOp::BitOr)),
            Op::BITXOR => try_op!(bin(f, BinOp::BitXor)),
            Op::LAND => try_op!(land_lor(f, true)),
            Op::LOR => try_op!(land_lor(f, false)),

            // -- comparisons --
            Op::EQ => try_op!(cmp(f, BinOp::Eq)),
            Op::NEQ => try_op!(cmp(f, BinOp::Ne)),
            Op::LT => try_op!(cmp(f, BinOp::Lt)),
            Op::GT => try_op!(cmp(f, BinOp::Gt)),
            Op::LE => try_op!(cmp(f, BinOp::Le)),
            Op::GE => try_op!(cmp(f, BinOp::Ge)),
            Op::STR_EQ => try_op!(cmp(f, BinOp::StrEq)),
            Op::STR_NEQ => try_op!(cmp(f, BinOp::StrNe)),
            Op::STR_LT => try_op!(cmp(f, BinOp::StrLt)),
            Op::STR_GT => try_op!(cmp(f, BinOp::StrGt)),
            Op::STR_LE => try_op!(cmp(f, BinOp::StrLe)),
            Op::STR_GE => try_op!(cmp(f, BinOp::StrGe)),
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

            // -- iRules dialect operators --
            // The F5 word operators (`contains`/`starts_with`/`ends_with`/
            // `equals`/`matches`/`matches_glob`/`matches_regex`/`and`/`or`/
            // `not`), which
            // `Op::from_binop`/`from_unaryop` emit for an iRules expression.
            // They have no C Tcl counterpart; the semantics live in `expr` next
            // to the standard operators, and reuse the same glob matcher, ARE
            // engine and string comparison the equivalent commands do. Operands
            // arrive left-then-right (subject below, needle/pattern on top).
            Op::IRULE_CONTAINS => try_op!(irule(f, BinOp::Contains)),
            Op::IRULE_STARTS_WITH => try_op!(irule(f, BinOp::StartsWith)),
            Op::IRULE_ENDS_WITH => try_op!(irule(f, BinOp::EndsWith)),
            Op::IRULE_EQUALS => try_op!(irule(f, BinOp::StrEquals)),
            Op::IRULE_MATCHES_GLOB => try_op!(irule(f, BinOp::MatchesGlob)),
            Op::IRULE_MATCHES_REGEX => try_op!(irule(f, BinOp::MatchesRegex)),
            Op::IRULE_MATCHES => try_op!(irule(f, BinOp::Matches)),
            Op::IRULE_WORD_AND => try_op!(irule(f, BinOp::WordAnd)),
            Op::IRULE_WORD_OR => try_op!(irule(f, BinOp::WordOr)),
            Op::IRULE_WORD_NOT => try_op!(un(f, UnaryOp::WordNot)),

            // -- string ops (inline; char-based, mirroring the reference VM) --
            Op::STR_LEN => {
                let s = pop(f).to_str();
                f.stack.push(Value::int(ilen(string_char_len(
                    &s,
                    self.runtime_version(),
                ))));
            }
            // C `INST_STR_INDEX` (`tclExecute.c:5336-5380`): the index goes
            // through `TclGetIntForIndexM` and a *syntactically* bad spec is
            // `goto gotError` (`bad index "foo": …`) — only the separate
            // `index < 0 || index >= slength` test yields the empty string. The
            // old `resolve_index(...).and_then(...)` chain collapsed both into
            // the empty string, so a garbage index looked like a miss.
            Op::STR_INDEX => {
                let idx = pop(f);
                let s = pop(f).to_str();
                let chars: Vec<char> = s.chars().collect();
                let i = match checked_index_value(&idx, chars.len()) {
                    Ok(i) => i,
                    Err(c) => return Tick::Return(c),
                };
                f.stack.push(
                    usize::try_from(i)
                        .ok()
                        .and_then(|i| chars.get(i))
                        .map_or_else(Value::empty, |c| Value::string(c.to_string())),
                );
            }
            // C `INST_STR_RANGE` (`tclExecute.c:5382-5406`): *both* indices go
            // through `TclGetIntForIndexM` and either failure is `goto gotError`
            // (lines 5388-5397); the surviving out-of-range handling is
            // `Tcl_GetRange`'s clamp plus the `toIdx == TCL_INDEX_NONE` → empty
            // short-circuit, which `char_range` already models. Defaulting a bad
            // `first` to 0 and a bad `last` to -1 turned a typo into a
            // plausible-looking substring.
            Op::STR_RANGE => {
                let to = pop(f);
                let from = pop(f);
                let s = pop(f).to_str();
                let chars: Vec<char> = s.chars().collect();
                let lo = match checked_index_value(&from, chars.len()) {
                    Ok(i) => i,
                    Err(c) => return Tick::Return(c),
                };
                let hi = match checked_index_value(&to, chars.len()) {
                    Ok(i) => i,
                    Err(c) => return Tick::Return(c),
                };
                f.stack.push(char_range(&chars, lo, hi));
            }
            Op::STR_RANGE_IMM => {
                let s = pop(f).to_str();
                let chars: Vec<char> = s.chars().collect();
                let lo = imm_index(imm0(instr), chars.len());
                let hi = imm_index(imm_at(instr, 1), chars.len());
                f.stack.push(char_range(&chars, lo, hi));
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
                let s = pop(f).to_str();
                f.stack
                    .push(Value::string(s.chars().rev().collect::<String>()));
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
                let s = pop(f).to_str();
                let n = count.as_int().unwrap_or(0).max(0);
                let times = usize::try_from(n).unwrap_or(0);
                // Charged before the allocation, not after: this is the one
                // opcode that can ask for gigabytes while the `commands` and
                // `time` budgets are still nearly full.
                let wanted = (s.len() as u64).saturating_mul(times as u64);
                if let Some(refusal) = self.charge_allocation(wanted) {
                    return Tick::Return(refusal);
                }
                f.stack.push(Value::string(s.repeat(times)));
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

            // -- numeric/boolean coercion checks (expr result validation) --
            Op::TRY_CVT_TO_NUMERIC => {
                // Canonical normalisation (C Tcl `INST_TRY_CVT_TO_NUMERIC`): a
                // numeric result's string rep is regenerated from the number
                // (`expr {1e3}` → `1000.0`, not `1e3`), a non-numeric value
                // (`expr {1 ? "big" : "x"}`) passes through, and a bare `NaN` is
                // the domain error.
                let v = pop(f);
                match crate::expr::cvt_to_numeric(v) {
                    Ok(nv) => f.stack.push(nv),
                    Err(e) => return Tick::Return(err(e.message)),
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

            // -- unary --
            Op::UMINUS => try_op!(un(f, UnaryOp::Neg)),
            Op::UPLUS => try_op!(un(f, UnaryOp::Pos)),
            Op::BITNOT => try_op!(un(f, UnaryOp::BitNot)),
            Op::NOT | Op::LNOT => try_op!(un(f, UnaryOp::Not)),

            // -- control flow --
            Op::JUMP1 | Op::JUMP4 => {
                if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                    f.pc = idx;
                }
            }
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4 => {
                let c = pop(f);
                match c.as_bool() {
                    Ok(true) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(false) => {}
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::JUMP_FALSE1 | Op::JUMP_FALSE4 => {
                let c = pop(f);
                match c.as_bool() {
                    Ok(false) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(true) => {}
                    Err(e) => return Tick::Return(err(e.message)),
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

            // -- break / continue --
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

            // -- return --
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
                let c = crate::command::cmd_return(
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
                );
                if c.code == Code::Ok {
                    f.stack.push(c.result);
                } else {
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
                let c =
                    crate::command::cmd_return(self, &[Value::string("-options"), opts, result]);
                if c.code == Code::Ok {
                    f.stack.push(c.result);
                } else {
                    return Tick::Return(c);
                }
            }

            // -- command dispatch / expr --
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < argc || argc == 0 {
                    return Tick::Return(err("invoke: stack underflow"));
                }
                let words = f.stack.split_off(f.stack.len() - argc);
                match self.dispatch_words(f, &words) {
                    Ok(Some(call)) => return call,
                    Ok(None) => {}
                    Err(c) => {
                        let cmd_text = instr.source_cmd_text.clone();
                        let msg = c.result.to_str().to_string();
                        let line = instr.source_line;
                        self.log_command_info(&cmd_text, &msg, line);
                        return Tick::Return(c);
                    }
                }
            }

            // -- {*} argument expansion --
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
                match list_v.as_list() {
                    Ok(items) => f.stack.extend(items.iter().cloned()),
                    Err(e) => return Tick::Return(err(e.message)),
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
                let words = f.stack.split_off(marker);
                if words.is_empty() {
                    return Tick::Return(err("invalid command name \"\""));
                }
                match self.dispatch_words(f, &words) {
                    Ok(Some(call)) => return call,
                    Ok(None) => {}
                    Err(c) => {
                        let cmd_text = instr.source_cmd_text.clone();
                        let msg = c.result.to_str().to_string();
                        let line = instr.source_line;
                        self.log_command_info(&cmd_text, &msg, line);
                        return Tick::Return(c);
                    }
                }
            }
            // Ensemble-rewrite invoke (C Tcl `INST_INVOKE_REPLACE`): operands are
            // `(objc, opnd)` — `objc` words sit on the stack with the resolved
            // implementation word on top. The first `opnd` original words (e.g.
            // `string equal`) are replaced by the popped implementation
            // (`::tcl::string::equal`); the effective command is
            // `impl + words[opnd..]`. (The ensemble error-message rewrite C does
            // via `TclInitRewriteEnsemble` is omitted — only the dispatch
            // matters for execution.)
            Op::INVOKE_REPLACE => {
                let objc = usize::try_from(imm0(instr)).unwrap_or(0);
                let opnd = usize::try_from(imm_at(instr, 1)).unwrap_or(0);
                let repl = pop(f);
                if f.stack.len() < objc {
                    return Tick::Return(err("invokeReplace: stack underflow"));
                }
                let words = f.stack.split_off(f.stack.len() - objc);
                let mut argv = Vec::with_capacity(objc.saturating_sub(opnd) + 1);
                argv.push(repl);
                if opnd < words.len() {
                    argv.extend_from_slice(&words[opnd..]);
                }
                match self.dispatch_words(f, &argv) {
                    Ok(Some(call)) => return call,
                    Ok(None) => {}
                    Err(c) => {
                        let cmd_text = instr.source_cmd_text.clone();
                        let msg = c.result.to_str().to_string();
                        self.log_command_info(&cmd_text, &msg, instr.source_line);
                        return Tick::Return(c);
                    }
                }
            }
            Op::EXPR_STK => {
                let s = pop(f).to_str();
                match self.eval_expr(&s) {
                    Ok(v) => f.stack.push(v),
                    // Through `completion_from_tcl_error`, not `err`: a syntax
                    // error in the expression carries a `TCL PARSE EXPR …`
                    // `-errorcode` that a bare message would drop.
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(e));
                    }
                }
            }

            // -- dicts (LVT form, proc bodies) --
            // `dict set var k1 ?k2 …? value` — operands [Imm(N), Imm(slot)];
            // stack holds the N keys then the value. Writes the variable and
            // leaves the new dict on the stack (the codegen POPs it).
            Op::DICT_SET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                let name = lvt_name(imm_at(instr, 1));
                let value = pop(f);
                if f.stack.len() < nkeys || nkeys == 0 {
                    return Tick::Return(err("dictSet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let cur = self.get_var(&name).unwrap_or_else(Value::empty);
                match self.dict_set_path(&cur, &keys, value) {
                    Ok(result) => {
                        try_op!(self.set_var(&name, result.clone()));
                        f.stack.push(result);
                    }
                    Err(c) => return Tick::Return(c),
                }
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
                let cur = self.get_var(&name).unwrap_or_else(Value::empty);
                match self.dict_unset_path(&cur, &keys) {
                    Ok(result) => {
                        try_op!(self.set_var(&name, result.clone()));
                        f.stack.push(result);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            // `dict incr var key ?amount?` — operands [Imm(amount), Imm(slot)];
            // stack holds the key.
            Op::DICT_INCR_IMM => {
                let amount = i64::from(imm0(instr));
                let name = lvt_name(imm_at(instr, 1));
                let key = pop(f).to_str().to_string();
                let cur = self.get_var(&name).unwrap_or_else(Value::empty);
                let inc = Value::int(amount);
                let updated = self.dict_update_single(&cur, &key, |old| {
                    // The same tower addition `incr` uses (`value_ops::int_add`):
                    // a sum past `i64` promotes to `i128` and past that to an
                    // arbitrary-precision bignum rather than erroring, matching
                    // tclsh.
                    crate::value_ops::int_add(old, &inc).map_err(|e| err(e.message()))
                });
                match updated {
                    Ok(result) => {
                        try_op!(self.set_var(&name, result.clone()));
                        f.stack.push(result);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            // `dict append var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_APPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f).to_str().to_string();
                let cur = self.get_var(&name).unwrap_or_else(Value::empty);
                let updated = self.dict_update_single(&cur, &key, |old| {
                    let mut s = old.map(|v| v.to_str().to_string()).unwrap_or_default();
                    s.push_str(&value.to_str());
                    Ok(Value::string(s))
                });
                match updated {
                    Ok(result) => {
                        try_op!(self.set_var(&name, result.clone()));
                        f.stack.push(result);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            // `dict lappend var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_LAPPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f).to_str().to_string();
                let cur = self.get_var(&name).unwrap_or_else(Value::empty);
                let updated = self.dict_update_single(&cur, &key, |old| {
                    let mut items = match old {
                        Some(v) => (*v.as_list().map_err(|e| err(e.message))?).clone(),
                        None => Vec::new(),
                    };
                    items.push(value.clone());
                    Ok(Value::list(items))
                });
                match updated {
                    Ok(result) => {
                        try_op!(self.set_var(&name, result.clone()));
                        f.stack.push(result);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            // `dict get $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves the looked-up value on the stack.
            Op::DICT_GET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictGet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let mut cur = pop(f);
                for k in &keys {
                    let ps = match self.dict_pairs(&cur) {
                        Ok(p) => p,
                        Err(c) => return Tick::Return(c),
                    };
                    let ks = k.to_str();
                    match ps.iter().find(|(pk, _)| pk == &*ks) {
                        Some((_, v)) => cur = v.clone(),
                        None => {
                            return Tick::Return(err(format!(
                                "key \"{ks}\" not known in dictionary"
                            )));
                        }
                    }
                }
                f.stack.push(cur);
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
                let mut cur = pop(f);
                let mut found = true;
                for k in &keys {
                    let ps = match self.dict_pairs(&cur) {
                        Ok(p) => p,
                        Err(c) => return Tick::Return(c),
                    };
                    let ks = k.to_str();
                    if let Some((_, v)) = ps.iter().find(|(pk, _)| pk == &*ks) {
                        cur = v.clone();
                    } else {
                        found = false;
                        break;
                    }
                }
                f.stack.push(if found { cur } else { default });
            }
            // `dict exists $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves a boolean on the stack.
            Op::DICT_EXISTS => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictExists: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let mut cur = pop(f);
                let mut found = true;
                for k in &keys {
                    let Ok(ps) = self.dict_pairs(&cur) else {
                        found = false;
                        break;
                    };
                    let ks = k.to_str();
                    if let Some((_, v)) = ps.iter().find(|(pk, _)| pk == &*ks) {
                        cur = v.clone();
                    } else {
                        found = false;
                        break;
                    }
                }
                f.stack.push(Value::bool(found));
            }

            // Reverse the top `N` stack elements in place (C Tcl `INST_REVERSE`;
            // operand = N). Used to reorder operands an inline emitter pushed in
            // the convenient order.
            Op::REVERSE => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
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
                match tcl_cmd_core::list::lindex_flat(self, list, idxs) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => return Tick::Return(err(e.into_message())),
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
                let bad = |spec: &str| {
                    err(format!(
                        "bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?"
                    ))
                };
                let first_s = first.to_str();
                let last_s = last.to_str();
                let Some(from) = crate::command::resolve_index(&first_s, len) else {
                    return Tick::Return(bad(&first_s));
                };
                let Some(to) = crate::command::resolve_index(&last_s, len) else {
                    return Tick::Return(bad(&last_s));
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
                let path = match index_list.as_list() {
                    Ok(p) => (*p).clone(),
                    Err(e) => return Tick::Return(err(e.message)),
                };
                match lset_descend(&list, &path, value) {
                    Ok(r) => f.stack.push(r),
                    Err(c) => return Tick::Return(c),
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
                match lset_descend(&list, &path, value) {
                    Ok(r) => f.stack.push(r),
                    Err(c) => return Tick::Return(c),
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
            Op::REGEXP => {
                const TCL_REG_NOCASE: i32 = 0o10;
                let nocase = imm0(instr) & TCL_REG_NOCASE != 0;
                let s = pop(f).to_str();
                let pat = pop(f).to_str();
                match crate::cmd_regexp::regexp_matches(&pat, &s, nocase) {
                    Ok(m) => f.stack.push(Value::bool(m)),
                    Err(msg) => return Tick::Return(err(msg)),
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

            // `tailcall cmd ?arg …?` — operand = word count including the
            // `"tailcall"` prefix word the codegen pushes first. Drop that prefix
            // and hand `[cmd, arg, …]` to the trampoline, which runs it in the
            // caller's activation once this proc unwinds (C Tcl `INST_TAILCALL`).
            Op::TAILCALL => {
                let count = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < count || count == 0 {
                    return Tick::Return(err("tailcall: stack underflow"));
                }
                let mut words = f.stack.split_off(f.stack.len() - count);
                // Drop the leading `"tailcall"` literal word.
                words.remove(0);
                return Tick::Tailcall(words);
            }

            // -- termination --
            Op::DONE => {
                return Tick::Return(ok(f
                    .stack
                    .last()
                    .cloned()
                    .unwrap_or_else(|| f.last_result.clone())));
            }

            // The compiled catch epilogue's reads (C `INST_PUSH_RESULT`/
            // `INST_PUSH_RETURN_CODE`/`INST_PUSH_RETURN_OPTS`): after a range
            // absorbed a completion they report *it*; on the fall-through (no
            // catch fired) path they report the current result / an ok options
            // dict, as C's untouched interp state would.
            Op::PUSH_RESULT => {
                let v = Self::innermost_caught(f)
                    .map_or_else(|| f.last_result.clone(), |c| c.result.clone());
                f.stack.push(v);
            }
            Op::PUSH_RETURN_CODE => {
                let code = Self::innermost_caught(f).map_or(0, |c| c.code.as_int());
                f.stack.push(Value::int(code));
            }
            Op::PUSH_RETURN_OPTS => {
                let opts = Self::innermost_caught(f).map_or_else(
                    || crate::command::options_dict(Code::Ok, 0, &[]),
                    |c| c.options.clone(),
                );
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
            Op::EVAL_STK => {
                // Run the script on the *explicit* stack (a transparent script
                // frame) rather than a nested drive, so a `yield` inside it stays
                // yieldable. Its result/`break`/`continue`/error
                // is delivered to this frame by `unwind` exactly as the old inline
                // push/`Tick::Return` did.
                let script = pop(f).to_str().to_string();
                match self.compile_source_cached(&script) {
                    Ok(asm) => {
                        return Tick::PushScript {
                            asm,
                            label: None,
                            cleanup_proc: None,
                        };
                    }
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }

            // -- introspection (C Tcl's "general introspector" instructions) --
            // Each routes through the same core its command form uses, so the
            // compiled and dispatched paths cannot drift.
            Op::CURRENT_NAMESPACE => {
                f.stack.push(tcl_cmd_core::namespace::current(self));
            }
            Op::INFO_LEVEL_NUM => match tcl_cmd_core::info::level(self, None) {
                Ok(v) => f.stack.push(v),
                Err(e) => return Tick::Return(err(e.message())),
            },
            Op::INFO_LEVEL_ARGS => {
                let n = pop(f);
                match tcl_cmd_core::info::level(self, Some(&n)) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => return Tick::Return(err(e.message())),
                }
            }
            // `resolveCmd` never errors — an unresolved name pushes the empty
            // string (C `INST_RESOLVE_COMMAND`); `originCmd` follows the import
            // chain and errors when the name resolves to nothing.
            Op::RESOLVE_CMD => {
                let name = pop(f).to_str();
                let fqn = if self.lookup_command(&name).is_some() {
                    self.resolve_command_fqn(self.current_ns(), &name)
                        .map_or_else(Value::empty, |key| Value::string(format!("::{key}")))
                } else {
                    Value::empty()
                };
                f.stack.push(fqn);
            }
            Op::ORIGIN_CMD => {
                let name = pop(f).to_str();
                match self.resolve_command_fqn(self.current_ns(), &name) {
                    Some(key) if self.lookup_command(&name).is_some() => {
                        let origin = self.command_origin_key(&key);
                        f.stack.push(Value::string(format!("::{origin}")));
                    }
                    _ => {
                        return Tick::Return(err(format!("invalid command name \"{name}\"")));
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
            // TOS is the command list to run in the resuming context; `resume`
            // runs it there and its result lands where the list was.
            Op::YIELD_TO_INVOKE => {
                let words = match pop(f).as_list() {
                    Ok(w) => w,
                    Err(e) => return Tick::Return(err(e.message)),
                };
                if let Err(c) = crate::cmd_coro::request_yieldto(self, &words) {
                    return Tick::Return(c);
                }
                if let Some(req) = self.coro.pending.take() {
                    return Tick::Suspend(req);
                }
            }
            Op::CORO_NAME => {
                f.stack.push(crate::cmd_coro::current_coroutine(self));
            }

            // -- TclOO (C's "start of TclOO support instructions" block) --
            // Each routes through the core its command form uses: `self object`,
            // `info object class`/`namespace`/`isa object`, `next`, `nextto`. The
            // context checks are the opcodes' own, since C's compiled forms report
            // "X may only be called from inside a method" where the command forms
            // report an unresolvable command name.
            Op::TCLOO_SELF => match crate::cmd_oo::current_object_name(self) {
                Some(name) => f.stack.push(name),
                None => {
                    return Tick::Return(err("self may only be called from inside a method"));
                }
            },
            Op::TCLOO_IS_OBJECT => {
                let name = pop(f);
                f.stack
                    .push(Value::bool(crate::cmd_oo::is_object(self, &name)));
            }
            Op::TCLOO_CLASS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => {
                        let v = crate::cmd_oo::object_class_name(self, &key);
                        f.stack.push(v);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_NS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => {
                        let v = crate::cmd_oo::object_namespace_name(self, &key);
                        f.stack.push(v);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_NEXT => try_op!(self.tcloo_next(f, instr, false)),
            Op::TCLOO_NEXT_CLASS => try_op!(self.tcloo_next(f, instr, true)),
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
        if !crate::cmd_oo::in_method(self) {
            return Err(err(format!(
                "{verb} may only be called from inside a method"
            )));
        }
        let args = &words[1..];
        let res = if nextto {
            crate::cmd_oo::cmd_nextto(self, args)
        } else {
            crate::cmd_oo::cmd_next(self, args)
        };
        // `deliver_sync` pushes the result on `OK` and hands the completion back
        // otherwise; the chained method runs on the native stack, so there is
        // never a `Tick` to propagate from here.
        Self::deliver_sync(f, res).map(|_| ())
    }

    /// Dispatch a fully-assembled command word list. Returns `Some(Tick::Call)`
    /// for a proc (the trampoline performs the activation), `None` when a builtin
    /// ran and pushed its result, or `Err` to unwind on a non-`OK` completion or
    /// an unknown command.
    fn dispatch_words(
        &mut self,
        f: &mut Frame,
        words: &[Value],
    ) -> Result<Option<Tick>, Completion<Value>> {
        // Every dispatched command is charged against the `commands` limit
        // before anything runs, so an armed budget bounds the work exactly
        // (issue #1373 finding 1).
        if let Some(exceeded) = self.charge_command() {
            return Err(exceeded);
        }
        // M16.3 fast path: no execution traces registered and no step scope
        // active — dispatch untraced (the common case pays one empty check).
        // Also untraced while a trace callback is itself running (C's
        // `INTERP_TRACE_IN_PROGRESS`) — a callback's own commands are never
        // step-observed, so its `puts`/etc. dispatch here plainly.
        if self.trace_in_progress.get()
            || (self.exec_traces.is_empty() && self.exec_step_scopes.is_empty())
        {
            return self.dispatch_words_inner(f, words, None);
        }
        self.dispatch_words_traced(f, words)
    }

    /// The traced dispatch path (M16.3): fire `enterstep` for every active
    /// step scope and `enter` for the command's own execution traces (a trace
    /// error aborts the command with that error — tclsh-pinned), push the
    /// command's own step scopes, dispatch, then settle the leave side — for
    /// synchronous completions here, for deferred bodies when their frame
    /// unwinds (the context rides on the frame via `pending_exec_leave`).
    fn dispatch_words_traced(
        &mut self,
        f: &mut Frame,
        words: &[Value],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let name = words[0].to_str();
        let key = self
            .resolve_command_fqn(self.current_ns(), &name)
            .unwrap_or_default();
        // The complete current command, list-merged (tclsh-pinned shape:
        // `p one {t w o}`).
        let cmd_string = Value::list(words.to_vec()).to_str().to_string();
        for scope in self.exec_step_scopes.clone() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[
                        Value::string(cmd_string.clone()),
                        Value::string("enterstep"),
                    ],
                );
                if !r.code.is_ok() {
                    return Err(r);
                }
            }
        }
        let key = CommandSidecarKey::visible(key);
        let sidecar = self.active_sidecar(key.clone());
        let own = self.exec_traces.get(&key).cloned().unwrap_or_default();
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self.run_cmd_trace_callback(
                    entry,
                    &[Value::string(cmd_string.clone()), Value::string("enter")],
                );
                if !r.code.is_ok() {
                    return Err(r);
                }
            }
        }
        let mut pushed = 0usize;
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enterstep") || entry.has_op("leavestep") {
                self.exec_step_scopes.push(ExecStepScope {
                    entry: Rc::clone(entry),
                    key: sidecar.clone(),
                });
                pushed += 1;
            }
        }
        let ctx = ExecLeaveCtx {
            cmd_string,
            key: sidecar.clone(),
            step_scopes: pushed,
        };
        match self.dispatch_words_inner(f, words, Some(&sidecar)) {
            Ok(Some(tick)) => {
                match &tick {
                    // The body runs on a pushed frame: the context rides
                    // along and settles when that frame completes.
                    Tick::Call { .. }
                    | Tick::PushScript { .. }
                    | Tick::PushCatch(_)
                    | Tick::PushSubst(_)
                    | Tick::PushEachLoop(_)
                    | Tick::PushTry(_) => self.pending_exec_leave = Some(ctx),
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
        for _ in 0..ctx.step_scopes {
            self.exec_step_scopes.pop();
        }
        if self.exec_traces.is_empty() && self.exec_step_scopes.is_empty() {
            return None;
        }
        let args = |result: Value, op: &str| {
            [
                Value::string(ctx.cmd_string.clone()),
                Value::string(c.code.as_int().to_string()),
                result,
                Value::string(op),
            ]
        };
        let mut replacement = None;
        // Tcl threads a successful leave callback's result to the next
        // callback, while the traced command's own completion remains `c`.
        let mut trace_result = c.result.clone();
        if let Some(key) = ctx.key.key()
            && let Some(entries) = self.exec_traces.get(&key).cloned()
        {
            for e in entries {
                if !ctx.key.is_attached() {
                    break;
                }
                if !self.exec_trace_entry_live(&ctx.key, &e) {
                    continue;
                }
                if e.has_op("leave") {
                    let r = self.run_cmd_trace_callback(&e, &args(trace_result.clone(), "leave"));
                    if !r.code.is_ok() {
                        replacement = Some(r);
                        break;
                    }
                    trace_result = r.result;
                }
            }
        }
        for scope in self
            .exec_step_scopes
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
        {
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
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if res.code.is_ok() {
            f.stack.push(res.result);
            Ok(None)
        } else {
            Err(res)
        }
    }

    /// Settle a synchronously-run native command (a [`BuiltinFn`] or an
    /// embedder's [`NativeCommand`](crate::command::NativeCommand)): drain any
    /// body it deferred to the explicit stack, else deliver its completion.
    fn settle_native_dispatch(
        &mut self,
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        // A `yield`/`yieldto` sets `coro.pending` (after its own boundary
        // check); convert it into a suspend `Tick` that freezes the whole
        // activation stack. The builtin's placeholder result is dropped —
        // the resume value replaces it on the operand stack.
        if let Some(req) = self.coro.pending.take() {
            return Ok(Some(Tick::Suspend(req)));
        }
        // An `eval`/`uplevel`/`apply`-style builtin defers its body to the
        // explicit stack (yieldable): drain it into a `PushScript`, whose
        // frame result replaces this builtin's placeholder (as for yield).
        if let Some((asm, label, cleanup_proc)) = self.pending_eval.take() {
            return Ok(Some(Tick::PushScript {
                asm,
                label,
                cleanup_proc,
            }));
        }
        // A `catch` defers its body the same way, but into a catch frame
        // whose completion the epilogue absorbs (see `Frame::catch`).
        if let Some(req) = self.pending_catch.take() {
            return Ok(Some(Tick::PushCatch(req)));
        }
        // A `subst` defers to a scanner-driven subst frame, whose `[…]`
        // bodies run yieldably as child frames (see `Frame::subst`).
        if let Some(req) = self.pending_subst.take() {
            return Ok(Some(Tick::PushSubst(req)));
        }
        // A `foreach`/`lmap` runtime-fallback loop defers to a
        // scanner-driven each-loop frame, whose iterations run yieldably
        // as child frames (see `Frame::each_loop`, issue #1311).
        if let Some(req) = self.pending_each_loop.take() {
            return Ok(Some(Tick::PushEachLoop(req)));
        }
        // A `try` defers its body (and, from `advance_try`, each
        // subsequent phase) to a try-phase frame (see `Frame::try_ctx`,
        // issue #1311).
        if let Some(req) = self.pending_try.take() {
            return Ok(Some(Tick::PushTry(req)));
        }
        Self::deliver_sync(f, res)
    }

    /// The untraced dispatch body — see [`Self::dispatch_words`].
    fn dispatch_words_inner(
        &mut self,
        f: &mut Frame,
        words: &[Value],
        sidecar_handle: Option<&CommandSidecarHandle>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let name = words[0].to_str();
        match self.lookup_command(&name) {
            Some(Command::Proc(p)) => {
                let sidecar = self
                    .resolve_command_fqn(self.current_ns(), &name)
                    .map(CommandSidecarKey::visible);
                // A traced dispatch re-resolves after its callbacks. Keep the
                // handle only when it still identifies that resolved binding;
                // a callback may have renamed or replaced the original key.
                let sidecar_handle =
                    sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
                let p = self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle);
                Ok(Some(Tick::Call {
                    proc: p,
                    argv: words[1..].to_vec(),
                }))
            }
            Some(Command::Builtin(bf)) => {
                self.set_invoked_name(&name);
                let res = bf(self, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Native(cmd)) => {
                self.set_invoked_name(&name);
                let res = cmd.invoke(self, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Alias(target)) => {
                // Evaluate `target prefix… args…` as one command (see
                // `alias_invoke_script` — brace-safe script quoting so the
                // alias works even when the target is a compiled control
                // command with no runtime builtin).
                //
                // The target resolves in the GLOBAL namespace regardless of the
                // caller's namespace (C's alias handler invokes with
                // TCL_EVAL_INVOKE; tclsh-pinned: an alias to relative `tgt`
                // called from `::ns` dispatches `::tgt` even when `::ns::tgt`
                // exists) — while the caller's *frame* stays current, so an
                // alias to `set` still writes the caller's locals (also
                // pinned). `invoke_alias_words` switches only the namespace.
                let mut argv: Vec<Value> = (*target).clone();
                argv.extend_from_slice(&words[1..]);
                let here = self.cur_interp();
                let res = self.invoke_alias_words(here, &argv);
                Self::deliver_sync(f, res)
            }
            Some(Command::CrossAlias {
                target: target_interp,
                words: target,
            }) => {
                // Cross-interp alias: the target runs in a DIFFERENT interp
                // (parent, child, or sibling).  The engine switches to that
                // interp on the shared native stack (C's
                // `Tcl_EvalObjv(targetInterp, …)`) and runs `target… args…`
                // there at its global frame/namespace, then switches back with
                // the completion — errors propagate catchably.  This works
                // whether this interp is at top level or re-entered deeper on
                // the stack (issue #946 fault 1: no more C-stack-busy).
                let mut argv: Vec<Value> = (*target).clone();
                argv.extend_from_slice(&words[1..]);
                let res = self.invoke_alias_words(target_interp, &argv);
                Self::deliver_sync(f, res)
            }
            Some(Command::ChildInterp(child)) => {
                let res = self.dispatch_child(&name, child, &words[1..]);
                Self::deliver_sync(f, res)
            }
            Some(Command::Ensemble(e)) => {
                let res = self.dispatch_ensemble(&name, &e, &words[1..]);
                Self::deliver_sync(f, res)
            }
            Some(Command::Object(key)) => {
                let res = crate::cmd_oo::oo_dispatch(self, &key, &name, &words[1..]);
                Self::deliver_sync(f, res)
            }
            // Resolution miss fallback chain: a `namespace unknown` handler
            // (current namespace's own, else the global namespace's — TIP
            // 181, not inherited, tclsh-pinned) takes the miss first,
            // invoked as `handler… name arg…` and guarded against a handler
            // whose own head is unresolvable; else Tcl's plain `unknown`
            // proc as `unknown name arg…` (the basis for auto-loading,
            // ensembles, and the iRule command mocks); else a hard error.
            None if &*name != "unknown" => {
                if let Some(handler) = self.ns_unknown_handler() {
                    let mut handler_words = handler;
                    handler_words.extend_from_slice(words);
                    self.with_ns_unknown_guard(|vm| vm.dispatch_words(f, &handler_words))
                } else if self.lookup_command("unknown").is_some() {
                    let mut unknown_words = Vec::with_capacity(words.len() + 1);
                    unknown_words.push(Value::string("unknown"));
                    unknown_words.extend_from_slice(words);
                    self.dispatch_words(f, &unknown_words)
                } else {
                    Err(err(format!("invalid command name \"{name}\"")))
                }
            }
            None => Err(err(format!("invalid command name \"{name}\""))),
        }
    }

    /// Dispatch a fully-resolved command by `name` + `argv` to a completion —
    /// the `Commands::dispatch` engine. Unlike [`dispatch_words`](Self::dispatch_words)
    /// (bytecode-frame-coupled: it pushes the result onto `f.stack` and defers a
    /// proc call to the trampoline as a `Tick::Call`), this is self-contained and
    /// usable outside the bytecode loop: a builtin runs inline, a proc body runs
    /// to completion in a nested `is_proc` activation (so its call-frame and
    /// `return` are handled), and an alias re-evaluates its target prefix.
    pub fn invoke_command(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        if let Some(exceeded) = self.charge_command() {
            return exceeded;
        }
        // M16.3: mirror `dispatch_words`' trace wrapper on this native path —
        // enter/enterstep before, leave/leavestep after (synchronously: the
        // completion is known when the inner call returns). Also untraced
        // while a trace callback is itself running (C's
        // `INTERP_TRACE_IN_PROGRESS`) — see `dispatch_words`.
        if self.trace_in_progress.get()
            || (self.exec_traces.is_empty() && self.exec_step_scopes.is_empty())
        {
            return self.invoke_command_inner(name, argv, None);
        }
        let key = self
            .resolve_command_fqn(self.current_ns(), name)
            .unwrap_or_default();
        let mut words = Vec::with_capacity(argv.len() + 1);
        words.push(Value::string(name));
        words.extend_from_slice(argv);
        let cmd_string = Value::list(words).to_str().to_string();
        for scope in self.exec_step_scopes.clone() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[
                        Value::string(cmd_string.clone()),
                        Value::string("enterstep"),
                    ],
                );
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let key = CommandSidecarKey::visible(key);
        let sidecar = self.active_sidecar(key.clone());
        let own = self.exec_traces.get(&key).cloned().unwrap_or_default();
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self.run_cmd_trace_callback(
                    entry,
                    &[Value::string(cmd_string.clone()), Value::string("enter")],
                );
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let mut pushed = 0usize;
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enterstep") || entry.has_op("leavestep") {
                self.exec_step_scopes.push(ExecStepScope {
                    entry: Rc::clone(entry),
                    key: sidecar.clone(),
                });
                pushed += 1;
            }
        }
        let ctx = ExecLeaveCtx {
            cmd_string,
            key: sidecar.clone(),
            step_scopes: pushed,
        };
        let c = self.invoke_command_inner(name, argv, Some(&sidecar));
        match self.finish_exec_leave(&ctx, &c) {
            Some(replacement) => replacement,
            None => c,
        }
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
        if let Some(exceeded) = self.charge_command() {
            return exceeded;
        }
        if self.trace_in_progress.get()
            || (self.exec_traces.is_empty() && self.exec_step_scopes.is_empty())
        {
            return self.invoke_resolved_command_inner(
                display_name,
                command,
                argv,
                Some(trace_key),
                None,
            );
        }
        let mut words = Vec::with_capacity(argv.len() + 1);
        words.push(Value::string(display_name));
        words.extend_from_slice(argv);
        let cmd_string = Value::list(words).to_str().to_string();
        for scope in self.exec_step_scopes.clone() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[
                        Value::string(cmd_string.clone()),
                        Value::string("enterstep"),
                    ],
                );
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let sidecar = self.active_sidecar(trace_key.clone());
        let own = self
            .exec_traces
            .get(&trace_key)
            .cloned()
            .unwrap_or_default();
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self.run_cmd_trace_callback(
                    entry,
                    &[Value::string(cmd_string.clone()), Value::string("enter")],
                );
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let mut pushed = 0usize;
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enterstep") || entry.has_op("leavestep") {
                self.exec_step_scopes.push(ExecStepScope {
                    entry: Rc::clone(entry),
                    key: sidecar.clone(),
                });
                pushed += 1;
            }
        }
        let ctx = ExecLeaveCtx {
            cmd_string,
            key: sidecar.clone(),
            step_scopes: pushed,
        };
        let Some(live_key) = sidecar.key() else {
            return err("attempt to invoke a deleted command");
        };
        let c = self.invoke_resolved_command_inner(
            display_name,
            command,
            argv,
            Some(live_key),
            Some(&sidecar),
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
        // An `eval`/`uplevel`/`apply`-style builtin deferred its body to
        // `pending_eval`. This call site is on the native Rust stack (no
        // trampoline to push onto), so run the body via a nested drive —
        // a `yield` inside cannot cross it, exactly like every other
        // `invoke_command` re-entry.
        if let Some((asm, label, cleanup_proc)) = self.pending_eval.take() {
            let comp =
                self.run_activation(Frame::new_script(asm, label, self.profile_generation()));
            if let Some(name) = cleanup_proc {
                self.take_command_unchecked(&name);
            }
            return comp;
        }
        // A `catch` deferred its body: run it via a nested drive (a `yield`
        // inside cannot cross this native re-entry), then absorb its
        // completion with the catch epilogue.
        if let Some(req) = self.pending_catch.take() {
            let comp =
                self.run_activation(Frame::new_script(req.asm, None, self.profile_generation()));
            return self.finish_catch(comp, req.resvar.as_ref(), req.optvar.as_ref());
        }
        // A `subst` deferred: run the scanner-driven subst frame via a
        // nested drive (its `[…]` bodies can't yield across this native
        // re-entry), returning its accumulated result.
        if let Some(req) = self.pending_subst.take() {
            return self.run_activation(Frame::new_subst(req, self.profile_generation()));
        }
        // A `foreach`/`lmap` runtime-fallback loop deferred: run the
        // scanner-driven each-loop frame via a nested drive (a `yield` in
        // its body can't cross this native re-entry either).
        if let Some(req) = self.pending_each_loop.take() {
            return self.run_activation(Frame::new_each_loop(req, self.profile_generation()));
        }
        // A `try` deferred its body: run it (and, via `Vm::unwind`'s own
        // `try_ctx` handling, every subsequent phase `advance_try`
        // decides on) via a nested drive — a `yield` anywhere in it
        // can't cross this native re-entry either. `unwind` pushes each
        // next phase onto the same nested `acts`, so one
        // `run_activation` call carries the whole construct through to
        // its final completion.
        if let Some(req) = self.pending_try.take() {
            return self.run_activation(Frame::new_try(req, self.profile_generation()));
        }
        res
    }

    /// The untraced invoke body — see [`Self::invoke_command`].
    fn invoke_command_inner(
        &mut self,
        name: &str,
        argv: &[Value],
        sidecar_handle: Option<&CommandSidecarHandle>,
    ) -> Completion<Value> {
        match self.lookup_command(name) {
            Some(command) => {
                let sidecar = self
                    .resolve_command_fqn(self.current_ns(), name)
                    .map(CommandSidecarKey::visible);
                let sidecar_handle =
                    sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
                self.invoke_resolved_command_inner(name, command, argv, sidecar, sidecar_handle)
            }
            None if name != "unknown" => {
                if let Some(handler) = self.ns_unknown_handler() {
                    let mut handler_words = handler;
                    handler_words.push(Value::string(name));
                    handler_words.extend_from_slice(argv);
                    let head = handler_words[0].to_str().to_string();
                    self.with_ns_unknown_guard(|vm| vm.invoke_command(&head, &handler_words[1..]))
                } else if self.lookup_command("unknown").is_some() {
                    let mut full = Vec::with_capacity(argv.len() + 1);
                    full.push(Value::string(name));
                    full.extend_from_slice(argv);
                    self.invoke_command("unknown", &full)
                } else {
                    err(format!("invalid command name \"{name}\""))
                }
            }
            None => err(format!("invalid command name \"{name}\"")),
        }
    }

    fn invoke_resolved_command_inner(
        &mut self,
        name: &str,
        command: Command,
        argv: &[Value],
        sidecar: Option<CommandSidecarKey>,
        sidecar_handle: Option<&CommandSidecarHandle>,
    ) -> Completion<Value> {
        match command {
            Command::Builtin(bf) => {
                self.set_invoked_name(name);
                let saved = std::mem::replace(&mut self.invoked_sidecar, sidecar);
                let res = bf(self, argv);
                self.invoked_sidecar = saved;
                self.settle_native_invoke(res)
            }
            Command::Native(cmd) => {
                self.set_invoked_name(name);
                let res = cmd.invoke(self, argv);
                self.settle_native_invoke(res)
            }
            Command::Proc(p) => {
                let p = self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle);
                match self.enter_proc(&p, argv) {
                    Ok(()) => self.run_activation(Frame::new(
                        Rc::clone(&p.body),
                        true,
                        p.compiled_profile_generation,
                    )),
                    Err(c) => c,
                }
            }
            Command::Alias(target) => {
                // Target resolves in the GLOBAL namespace, caller's frame kept
                // — see the `dispatch_words` Alias arm for the tclsh pins.
                let mut full: Vec<Value> = (*target).clone();
                full.extend_from_slice(argv);
                let here = self.cur_interp();
                self.invoke_alias_words(here, &full)
            }
            // Cross-interp alias: switch to the target interp and run the words
            // there.  Identical to the bytecode path — a cross-interp alias
            // works from a native re-entry (coroutine resume, `lsort
            // -command`, `invoke_command`), which used to error
            // `cannot invoke parent-interp alias: C stack busy` (issue #946
            // fault 1).
            Command::CrossAlias {
                target: target_interp,
                words: target,
            } => {
                let mut full: Vec<Value> = (*target).clone();
                full.extend_from_slice(argv);
                self.invoke_alias_words(target_interp, &full)
            }
            Command::ChildInterp(child) => self.dispatch_child(name, child, argv),
            Command::Ensemble(e) => self.dispatch_ensemble(name, &e, argv),
            Command::Object(key) => crate::cmd_oo::oo_dispatch(self, &key, name, argv),
            // Miss fallback chain (see `dispatch_words`): `namespace unknown`
            // handler first, then the plain `unknown` proc, then a hard error.
        }
    }

    /// Run a `TclOO` method body: enter the proc activation, link the object's
    /// instance variables into the fresh frame, push the OO call context, run
    /// the body to completion, then pop the context. This is the OO analogue of
    /// [`invoke_command`](Self::invoke_command)'s `Proc` arm plus the
    /// instance-variable linking the runtime's `run_proc` does.
    ///
    /// `link_vars` are `(local, storage-fqn)` pairs: each links a method-local
    /// name to the object namespace's variable of that FQN (namespace variables
    /// live in the global frame keyed by their qualified name, so the link is at
    /// level 0). The `frame` carries the resolved method chain that `self`/`my`/
    /// `next` consult while the body runs.
    pub(crate) fn oo_run_method(
        &mut self,
        proc: &ProcDef,
        argv: &[Value],
        link_vars: &[(String, String)],
        frame: crate::cmd_oo::OoFrame,
    ) -> Completion<Value> {
        if let Err(c) = self.enter_proc(proc, argv) {
            return c;
        }
        for (local, storage) in link_vars {
            self.add_link(local, 0, storage);
        }
        self.oo.call_stack.push(frame);
        let result = self.run_activation(Frame::new(
            Rc::clone(&proc.body),
            true,
            proc.compiled_profile_generation,
        ));
        self.oo.call_stack.pop();
        result
    }

    /// Run a `tailcall` in the place of the proc that issued it. The proc's
    /// activation (and its call-frame + namespace) is popped — it is in tail
    /// position, so it is finished — and `words` (`[cmd, arg, …]`) is dispatched
    /// in the *caller's* activation: a proc tailcall is entered as a fresh
    /// activation at the caller's level (so a tail-recursive loop neither grows
    /// the activation stack nor counts against the recursion limit, the whole
    /// point of `tailcall`); a builtin runs inline and pushes its result. Returns
    /// `Some` when the whole `run` is finished, `None` when a parent resumed.
    /// Mirrors C Tcl's deferred-tailcall NR callback.
    fn run_tailcall(
        &mut self,
        acts: &mut Vec<Frame>,
        words: &[Value],
    ) -> Option<Completion<Value>> {
        let is_proc = acts.last().is_some_and(|fr| fr.is_proc);
        if !is_proc {
            let c = err("tailcall can only be called from a proc, lambda or method");
            return self.unwind(acts, c);
        }
        // Pop the issuing proc in tail position.  Its execution-trace leave
        // contexts (M16.3) transfer to whatever replaces it — C fires a
        // tailcalling proc's leave trace when the tailcalled command finally
        // returns.
        let mut carried = acts
            .pop()
            .map(|mut fr| std::mem::take(&mut fr.exec_leave))
            .unwrap_or_default();
        self.pop_call_frame();
        self.pop_ns();
        if words.is_empty() {
            // `tailcall` with no command is a plain return of "".
            let mut c = ok(Value::empty());
            for ctx in carried.drain(..).rev() {
                if let Some(replacement) = self.finish_exec_leave(&ctx, &c) {
                    c = replacement;
                }
            }
            if !c.code.is_ok() {
                return self.unwind(acts, c);
            }
            return match acts.last_mut() {
                None => Some(c),
                Some(parent) => {
                    parent.stack.push(c.result);
                    None
                }
            };
        }
        let name = words[0].to_str().to_string();
        match acts.last_mut() {
            // The issuing proc was the outermost activation: run the tailcall to
            // completion and let it be the overall result.
            None => {
                let mut c = self.invoke_command(&name, &words[1..]);
                for ctx in carried.drain(..).rev() {
                    if let Some(replacement) = self.finish_exec_leave(&ctx, &c) {
                        c = replacement;
                    }
                }
                Some(c)
            }
            Some(parent) => match self.dispatch_words(parent, words) {
                Ok(Some(Tick::Call { proc, argv })) => match self.enter_proc(&proc, &argv) {
                    Ok(()) => {
                        let mut fr = Frame::new(
                            Rc::clone(&proc.body),
                            true,
                            proc.compiled_profile_generation,
                        );
                        if let Some(ctx) = self.pending_exec_leave.take() {
                            fr.exec_leave.push(ctx);
                        }
                        fr.exec_leave.extend(carried);
                        acts.push(fr);
                        None
                    }
                    Err(c) => {
                        let c = match self.pending_exec_leave.take() {
                            Some(ctx) => self.finish_exec_leave(&ctx, &c).unwrap_or(c),
                            None => c,
                        };
                        self.unwind(acts, c)
                    }
                },
                Ok(_) => {
                    // Synchronous tailcall target: its result is on the
                    // parent's stack; settle the carried contexts with it.
                    let result = acts
                        .last()
                        .and_then(|p| p.stack.last().cloned())
                        .unwrap_or_else(Value::empty);
                    let mut c = ok(result);
                    let mut replaced = false;
                    for ctx in carried.drain(..).rev() {
                        if let Some(replacement) = self.finish_exec_leave(&ctx, &c) {
                            c = replacement;
                            replaced = true;
                        }
                    }
                    if replaced {
                        if let Some(parent) = acts.last_mut() {
                            parent.stack.pop();
                        }
                        return self.unwind(acts, c);
                    }
                    None
                }
                Err(c) => {
                    let mut c = c;
                    for ctx in carried.drain(..).rev() {
                        if let Some(replacement) = self.finish_exec_leave(&ctx, &c) {
                            c = replacement;
                        }
                    }
                    self.unwind(acts, c)
                }
            },
        }
    }

    /// `incr` helper shared by the scalar/stk increment opcodes.
    ///
    /// The current cell is read exactly as before (scalar or `arr(key)`), but the
    /// arithmetic goes through the shared [`tcl_syntax::value::ValueOps::int_add`]
    /// seam — the same one `incr`'s command core uses — so the number-model
    /// behaviour is identical across the compiled and dispatched paths: a sum
    /// past `i64` promotes through the integer tower (`i128`, then an
    /// arbitrary-precision bignum), matching tclsh, rather than wrapping (the
    /// old `wrapping_add` bug) or erroring.
    fn incr_var(
        &mut self,
        f: &mut Frame,
        name: &str,
        amount: i64,
    ) -> Result<(), Completion<Value>> {
        if self.is_constant(name) {
            return Err(err(format!(
                "can't incr \"{name}\": variable is a constant"
            )));
        }
        let cur = self.var_get(name);
        let inc = Value::int(amount);
        let next = tcl_syntax::value::ValueOps::int_add(self, cur.as_ref(), &inc)
            .map_err(|e| err(e.message()))?;
        self.var_set(name, next.clone())?;
        f.stack.push(next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{brace_safe, char_find, imm_index, lset_descend, quote_for_script};
    use crate::interp::Vm;
    use crate::value::Value;
    use tcl_bytecode::INDEX_END;

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
    fn brace_safe_tracks_balance_and_escapes() {
        // No braces / balanced braces are safe to `{…}`-wrap.
        assert!(brace_safe("hello"));
        assert!(brace_safe(""));
        assert!(brace_safe("a {b} c"));
        // Unbalanced in either direction is unsafe.
        assert!(!brace_safe("a {b"));
        assert!(!brace_safe("a }b"));
        // Escaped braces don't count toward depth, so this stays balanced.
        assert!(brace_safe(r"a \{ b"));
        assert!(brace_safe(r"a\{b"));
        // A trailing lone backslash would escape the wrapping `}` — unsafe.
        assert!(!brace_safe("trailing\\"));
    }

    #[test]
    fn quote_for_script_wraps_when_safe_and_falls_back_otherwise() {
        // Brace-safe words are wrapped verbatim.
        assert_eq!(quote_for_script("hello"), "{hello}");
        assert_eq!(quote_for_script("a b c"), "{a b c}");
        assert_eq!(quote_for_script(""), "{}");
        // An unbalanced word cannot be naively brace-wrapped; it must fall back
        // to canonical list-element quoting (delegated to `tcl_syntax::list`).
        let unsafe_word = "a{b";
        assert_ne!(quote_for_script(unsafe_word), format!("{{{unsafe_word}}}"));
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

    /// Regression coverage for issue #996: `lset_descend` recurses once per
    /// index in `lset`'s (possibly nested) index path, with no depth cap
    /// before this fix — it backs the compiled `INST_LSET_LIST`/
    /// `INST_LSET_FLAT` opcodes, so a flat index path is trivially inflated
    /// via `lset listVar {*}[lrepeat 100000 0] v`. Empirically (a
    /// throwaway `zzz_probe_depth lset <depth>` harness, deleted before
    /// this fix landed), unguarded input overflowed the native stack
    /// (SIGABRT) between depth 1800 and 2000 on a 2 MiB thread (`cargo
    /// test`'s per-test default). Rewritten iteratively (no depth cap at
    /// all); 2000 is comfortably past that crash range, and the result is
    /// checked for exact correctness at this depth (descending back down
    /// the same index path lands on the value that was set), not merely
    /// survival.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain
    /// (empirically, SIGABRT between depth 3500 and 4000 on a 2 MiB thread
    /// for construction+drop alone, independent of any operation performed
    /// on the value). That is a separate, genuinely unbounded-depth concern
    /// in `Value`'s representation itself, not in `lset_descend`'s
    /// now-iterative logic — out of scope for this fix.
    #[test]
    fn deeply_nested_lset_survives_and_is_correct() {
        const DEPTH: usize = 2_000;
        let mut v = Value::string("orig");
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
        }
        let path: Vec<Value> = (0..DEPTH).map(|_| Value::int(0)).collect();
        let result = lset_descend(&v, &path, Value::string("new")).expect("lset_descend survives");
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
        // Set an existing element two levels deep.
        let list = Value::list(vec![
            Value::list(vec![n(1), n(2)]),
            Value::list(vec![n(3), n(4)]),
        ]);
        let updated = lset_descend(&list, &[n(1), n(0)], n(99)).unwrap();
        assert_eq!(&*updated.to_str(), "{1 2} {99 4}");

        // An empty path replaces the whole value (`lset x {} v` == `set x v`).
        let replaced = lset_descend(&list, &[], Value::string("whole")).unwrap();
        assert_eq!(&*replaced.to_str(), "whole");

        // `idx == len` appends a fresh slot.
        let flat = Value::list(vec![n(1), n(2)]);
        let appended = lset_descend(&flat, &[n(2)], n(3)).unwrap();
        assert_eq!(&*appended.to_str(), "1 2 3");

        // Out-of-range and non-numeric indices still error.
        assert!(lset_descend(&flat, &[n(5)], n(0)).is_err());
        assert!(lset_descend(&flat, &[Value::string("bogus")], n(0)).is_err());
    }
}
