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

//! Dispatching commands: invoking instructions, word-list dispatch, execution traces, command invocation from the host and from commands, and `tailcall`.

use super::completion::{invoke_inner_context, logged_command};
use super::instructions::{imm_at, imm0, pop, take_words};
use super::{EnteredCommand, ExecLeaveCtx, Frame, OriginalExecutionOwner, Tick};
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_runtime_api::{Code, Completion};

mod command_invocation;
mod execution_traces;
mod tailcall;
mod word_dispatch;

impl Vm {
    /// Execute one command-invoking instruction (`f.pc` is already past it):
    /// `INVOKE_*` and the compiled `next`/`nextto` forms. Its frame is on the
    /// native stack for the whole invoked command, so it holds only these arms.
    #[inline(never)]
    pub(super) fn tick_invoke(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        macro_rules! try_op {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => return Tick::Return(c),
                }
            };
        }
        match instr.op {
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => return self.tick_invoke_stack(f, instr),

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
                                let (cmd_text, line, enclosing) = logged_command(instr);
                                let msg = c.result.string_bytes();
                                self.log_command_with_enclosing(
                                    &cmd_text,
                                    invoke_inner_context(self, instr, &words, &cmd_text),
                                    &enclosing,
                                    &msg,
                                    line,
                                );
                            }
                            return Tick::Return(c);
                        }
                    }
                }
            }
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
                            let (cmd_text, line, enclosing) = logged_command(instr);
                            let msg = c.result.string_bytes();
                            self.log_command_with_enclosing(
                                &cmd_text,
                                invoke_inner_context(self, instr, &rewritten, &cmd_text),
                                &enclosing,
                                &msg,
                                line,
                            );
                        }
                        return Tick::Return(c);
                    }
                }
            }
            Op::TCLOO_NEXT | Op::TCLOO_NEXT4 => try_op!(self.tcloo_next(f, instr, false)),
            Op::TCLOO_NEXT_CLASS | Op::TCLOO_NEXT_CLASS4 => {
                try_op!(self.tcloo_next(f, instr, true));
            }
            Op::TCLOO_NEXT_LIST => try_op!(self.tcloo_next_list(f, false)),
            Op::TCLOO_NEXT_CLASS_LIST => try_op!(self.tcloo_next_list(f, true)),
            _ => return self.tick_instruction(f, asm, instr),
        }
        Tick::Continue
    }

    fn tick_invoke_stack(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        let argc = usize::try_from(imm0(instr)).unwrap_or(0);
        if f.stack.len() < argc || argc == 0 {
            return Tick::Return(err("invoke: stack underflow"));
        }
        let words = crate::NativeListItems::invocation_view(Rc::new(
            f.stack.split_off(f.stack.len() - argc),
        ));
        let entered = f.take_entered_command();
        // How the release compiles this command where the VM runs it
        // (C's compiled `catch`/`try`/`for`/`switch`/`dict`): only in
        // a procedure body where its form requires one, and only where
        // the words it binds name the frame's compiled locals.
        let release_compiled = instr.release_compiled.filter(|compiled| {
            (f.is_proc || !compiled.in_procedure_only)
                && self.frame_compiled_locals_named(compiled.frame_local_words, &words)
        });
        self.pending.release_compiled = release_compiled;
        // A top-level statement's own command runs with no other
        // command active; a command nested in its words does not.
        self.pending.top_level_invocation = self.pending.bottom_activation
            && f.script_command
                .and_then(|start| f.asm.instructions.get(start))
                .and_then(|start| start.script_command.as_ref())
                .is_none_or(|command| command.text == instr.source_cmd_text);
        let dispatched = self.dispatch_words_with_entered(f, &words, entered.as_ref());
        self.pending.top_level_invocation = false;
        self.pending.release_compiled = None;
        match dispatched {
            Ok(Some(call)) => {
                f.record_invoked_words(&words);
                return call;
            }
            Ok(None) => {}
            Err(c) => {
                if c.code == Code::Error {
                    let (cmd_text, line, enclosing) = logged_command(instr);
                    let msg = c.result.string_bytes();
                    self.log_command_with_enclosing(
                        &cmd_text,
                        invoke_inner_context(self, instr, &words, &cmd_text),
                        &enclosing,
                        &msg,
                        line,
                    );
                }
                return Tick::Return(c);
            }
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
                .native_name_operand_bytes_checked(&words[0])
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
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
        let protocol = self.actual_native_string_protocol().ok_or_else(|| {
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
            .native_name_operand_bytes_checked(&words[0])
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
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
    pub(super) fn dispatch_words(
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
        self.dispatch_words_selected(f, words, entered, &[])
    }

    fn dispatch_words_selected(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original_scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return Err(crate::command::completion_from_cmd_error(self, error)),
        };
        let reached = self.dispatch_words_selected_in_original_scope(f, words, entered, usage);
        self.finish_original_dispatch(&original_scope, reached)
    }

    fn dispatch_words_selected_in_original_scope(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let context = self.current_namespace_execution_context();
        self.dispatch_words_entry_in_context(f, words, entered, &context, usage, false)
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
        let context =
            crate::interp::namespace_context::NamespaceExecutionContext::Resolved(context);
        self.dispatch_words_entry_in_context(f, words, entered, &context, usage, ordinary)
    }

    fn dispatch_words_entry_in_context(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        ordinary: bool,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original_scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return Err(crate::command::completion_from_cmd_error(self, error)),
        };
        let reached = self.dispatch_words_entry_at_in_original_scope(
            (&mut *f, words),
            entered,
            context,
            usage,
            ordinary,
            &original_scope,
        );
        self.finish_original_dispatch(&original_scope, reached)
    }

    fn dispatch_words_entry_at_in_original_scope(
        &mut self,
        invocation: (&mut Frame, &crate::NativeListItems),
        entered: Option<&EnteredCommand>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        ordinary: bool,
        original_scope: &NativeOperationScope,
    ) -> Result<Option<Tick>, Completion<Value>> {
        use tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent as Event;
        let (f, words) = invocation;
        if let Some(deleted) = self.deleted_interpreter_refusal() {
            return Err(deleted);
        }
        if ordinary {
            self.reset_native_ensemble_rewrite(Event::BeforeOrdinaryLookup)?;
        }
        let resolved = match (entered, words.first()) {
            (None, Some(head)) => Some(crate::interp::ResolvedHead {
                key: self
                    .resolve_original_command_key_in_context(context, head)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?,
                stamp: self.command_lookup_stamp(),
            }),
            _ => None,
        };
        let selected = entered.is_some()
            || resolved
                .as_ref()
                .is_some_and(|resolved| resolved.key.is_some());
        if ordinary && selected {
            self.reset_native_ensemble_rewrite(Event::AfterSuccessfulOrdinaryLookup)?;
        }
        if let Some(exceeded) = self.charge_command() {
            return Err(exceeded);
        }
        if !self.exec_traces_can_fire() {
            let ctx = ExecLeaveCtx {
                original_scopes: vec![OriginalExecutionOwner::Entered(original_scope.clone())],
                traces: false,
                jim_commands: Vec::new(),
                cmd_string: None,
                leave_owner: None,
                step_scopes: 0,
            };
            let reached = self.dispatch_words_inner(
                (&mut *f, words),
                None,
                entered,
                resolved.as_ref(),
                context,
                usage,
                original_scope,
            );
            return self.settle_traced_word_dispatch(f, ctx, reached);
        }
        self.dispatch_words_traced(f, words, entered, context, usage, original_scope)
    }

    pub(super) fn tick_original_invocation(&mut self, frame: &mut Frame, original: Value) -> Tick {
        let Some(protocol) = self.actual_native_string_protocol() else {
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
                return Tick::Return(match self.native_name_operand_bytes_checked(namespace) {
                    Ok(bytes) => self.namespace_lookup_error_bytes(&bytes),
                    Err(error) => crate::command::completion_from_cmd_error(self, error),
                });
            }
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error(self, error));
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
}
