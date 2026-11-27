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

//! Activation records: building a `Frame`, the trampoline that drives the activation stack, parked and stale activations, and procedure entry.

use super::instructions::frame_indexes;
use super::{
    ActivationErrorLog, BytecodeEntry, CatchCtx, CatchReq, EachLoopReq, EachLoopState,
    EnteredCommand, EvalReq, ExpressionReq, Frame, PolicyAtStamp, SubstReq, next_frame_serial,
};
use crate::value::Value;
use std::collections::HashMap;
use tcl_bytecode::{FunctionAsm, Op};

mod procedure_entry;
mod staleness;
mod suspension;
mod trampoline;

impl Frame {
    /// Keep what the current instruction consumed before it failed.
    pub(super) fn record_failed_operands(&mut self, operands: Vec<Value>) {
        self.failed_operands = Some((
            self.pc.saturating_sub(1),
            crate::NativeListItems::invocation_view(std::rc::Rc::new(operands)),
        ));
    }

    /// Keep the words the current invoke handed to a call that has not yet
    /// completed; a call that completes without error drops them.
    pub(super) fn record_invoked_words(&mut self, words: &crate::NativeListItems) {
        self.failed_operands = Some((self.pc.saturating_sub(1), words.clone()));
    }

    /// The operands the instruction at `index` consumed before it failed.
    pub(super) fn failed_operands_at(&self, index: usize) -> Option<&[Value]> {
        self.failed_operands
            .as_ref()
            .filter(|(at, _)| *at == index)
            .map(|(_, operands)| operands.as_slice())
    }

    pub(super) fn close_native_script(&mut self) {
        if self.jim_script.is_some() || self.jim_script_entry.is_some() {
            self.jim_evaluation.invocation = Value::empty();
            drop(self.jim_script.take());
            drop(self.jim_script_entry.take());
        }
    }

    fn jim_evaluation_script_location(
        has_jim_script: bool,
        source_location: Option<&tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Option<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>> {
        (!has_jim_script).then(|| tcl_runtime_api::jim_error_stack::JimScriptLocation {
            file: Value::string(source_location.map_or("", |location| location.file.as_str())),
            line: source_location.map_or(1, |location| location.line),
        })
    }

    #[allow(clippy::too_many_lines)] // one frame construction
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
            // Upstream's per-unit literal binding is not read by this
            // activation yet: PUSH reads `push_verbatim` and the literal pool.
            literal_values: _,
            manifest,
        } = unit;
        let (off2idx, foreach_pairs) = frame_indexes(&asm);
        let compiler_generation = (compiler != crate::compiled::CompilerProvenance::NativeScript)
            .then(|| compiler.generation());
        let jim_evaluation_script =
            Self::jim_evaluation_script_location(jim_script.is_some(), source_location.as_ref());
        Self {
            jim_script_entry: jim_script,
            jim_script: None,
            jim_evaluation: crate::interp::OriginalJimEvaluationFrame {
                evaluation_frame_entered: false,
                procedure_level: 0,
                command: None,
                is_procedure: false,
                script: jim_evaluation_script,
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
            policy_current_at: std::cell::Cell::new(None),
            namespace_current_at: std::cell::Cell::new(None),
            serial: next_frame_serial(),
            command_epoch,
            compiler_generation,
            manifest,
            off2idx,
            foreach_pairs,
            pc: 0,
            stack: Vec::new(),
            tailcall_owners: Vec::new(),
            original_invocation: None,
            dispatched_invocation: None,
            procedure_invocation: None,
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
            failed_operands: None,
            native_bytecode_entry: BytecodeEntry::Pending,
            deferred_dispatch: None,
            last_options: Value::empty(),
            is_proc,
            oo_call_exit: None,
            original_target: None,
            procedure_call_exit: None,
            is_script: false,
            script_command: None,
            settles_as_top_level: false,
            release_units: crate::release_units::ReleaseUnits::new(is_proc),
            replay_namespace_restore: None,
            selected_frame_restore: None,
            body_label: None,
            error_log: ActivationErrorLog::Own,
            body_invocation: None,
            package: None,
            tailcall_invocation: None,
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
        current_policy: &PolicyAtStamp<'_>,
    ) -> Option<&'static str> {
        if self.each_loop.is_some() {
            None
        } else if self.profile_generation != current_profile {
            Some("cannot continue bytecode after dialect profile changed")
        } else if let Some(message) = self.policy_stale_message(current_policy) {
            Some(message)
        } else if self
            .compiler_generation
            .is_some_and(|generation| generation != current_compiler)
        {
            Some("cannot continue bytecode after compile service changed")
        } else {
            None
        }
    }

    /// The frame's policy checks, skipped while the current policy keeps the
    /// stamp at which they last passed.
    fn policy_stale_message(&self, current: &PolicyAtStamp<'_>) -> Option<&'static str> {
        if self.direct_source_operands.is_none() && self.native_policy.is_none() {
            return None;
        }
        let stamp = current.stamp;
        if self.policy_current_at.get() == Some(stamp) {
            return None;
        }
        let policy = current.policy();
        let message = if self
            .direct_source_operands
            .as_ref()
            .is_some_and(|direct| !direct.is_current(&policy))
        {
            Some("cannot continue direct source after native invocation policy changed")
        } else if self.native_policy.is_some_and(|native| native != policy) {
            Some("cannot continue bytecode after native invocation policy changed")
        } else {
            None
        };
        if message.is_none() {
            self.policy_current_at.set(Some(stamp));
        }
        message
    }

    /// Build a bytecode frame from the generations owned by its compiled
    /// script. Keeping this extraction here prevents individual deferred
    /// consumers from accidentally substituting the VM's current generations.
    fn from_compiled_unit(unit: crate::compiled::CompiledUnit) -> Self {
        Self::new(unit, false)
    }

    /// Settle this coroutine base activation's completion as at
    /// `numLevels == 0` ([`Vm::settle_level_zero`]).
    pub(crate) fn settle_as_top_level(&mut self) {
        self.settles_as_top_level = true;
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
    pub(super) fn new_eval(req: EvalReq) -> Self {
        let mut frame = Self::new_script(req.script, req.label);
        frame.fatal_tail = req.fatal_tail.or(frame.fatal_tail);
        frame.cleanup_proc = req.cleanup_proc;
        frame.selected_frame_restore = req.selected_frame_restore;
        frame.package = req.package;
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
            evaluates_word: req.evaluates_word,
        }));
        f
    }

    /// A **subst** activation: a scanner-driven frame (empty placeholder asm — it
    /// never executes bytecode) carrying the resumable scan state. `tick` runs the
    /// scanner instead of the bytecode dispatch when `subst` is set.
    pub(crate) fn new_subst(req: SubstReq, placeholder: crate::compiled::CompiledUnit) -> Self {
        if let Some(unit) = req.compiled {
            let mut frame = Self::new_script(unit, None);
            if req.control == crate::subst::SubstitutionControl::Command {
                frame.error_log = ActivationErrorLog::InvokingCommand;
            }
            return frame;
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
        frame.native_bytecode_entry = BytecodeEntry::Entered;
        frame
    }

    pub(super) fn new_expression(
        req: ExpressionReq,
        placeholder: crate::compiled::CompiledUnit,
    ) -> Self {
        let mut frame = Self::new(placeholder, false);
        frame.expression = Some(Box::new(req));
        // Like C's `expr` returning to the command that ran it: that command
        // logs its level unless the expression's error is still logged.
        frame.error_log = ActivationErrorLog::InvokingCommand;
        frame
    }

    pub(super) fn new_control(
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
    pub(super) fn take_entered_command(&mut self) -> Option<EnteredCommand> {
        let index = self
            .entered_commands
            .iter()
            .rposition(|entered| entered.continuation == self.pc)?;
        Some(self.entered_commands.remove(index))
    }
}

/// Pair each `FOREACH_START` with its `FOREACH_STEP` by nesting order
/// (`foreach_start … body … foreach_step` nests like balanced brackets).
pub(super) fn pair_foreach(asm: &FunctionAsm) -> HashMap<usize, usize> {
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
