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

//! Scanner-driven activations that execute no bytecode: `subst`, each-loops, expressions and control commands, and folding a child completion back into them.

use super::completion::logged_command;
use super::instructions::pop;
use super::{
    EachLoopFold, EachLoopRoot, EachLoopState, ExpressionReq, Frame, ScriptNamespace, SubstFold,
    SubstReq, Tick,
};
use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::value::ValueOps;

impl Vm {
    pub(super) fn settle_control_activation(
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
            if ctx.evaluates_word {
                self.log_compiled_catch(acts, &c);
            }
            c = self.finish_catch(
                c,
                ctx.resvar.as_ref(),
                ctx.optvar.as_ref(),
                ctx.ignored_codes,
            );
            // A failed result or options store is the catch command's own
            // error: it is logged as it leaves that command.
            if c.code == Code::Error
                && let Some((cmd, line, enclosing)) = acts.last().and_then(|parent| {
                    parent
                        .asm
                        .instructions
                        .get(parent.pc.saturating_sub(1))
                        .map(logged_command)
                })
            {
                let message = c.result.string_bytes();
                self.log_command_with_enclosing(
                    &cmd,
                    Value::from_native_string_bytes(cmd.clone()),
                    &enclosing,
                    &message,
                    line,
                );
            }
        }
        // A package discovery script completed: the `package require` it
        // belongs to resumes, and either defers its next script (the
        // selected loader after `package unknown`) or completes.
        if let Some(suspended) = act.package.take() {
            c = crate::cmd_package::advance_require(self, *suspended, c);
            if let Some(req) = self.pending.eval.take() {
                let mut next = Frame::new_eval(req);
                next.exec_leave.append(&mut act.exec_leave);
                self.push_frame(acts, next);
                return Err(());
            }
            // The `package require` command is logged as its error leaves it.
            if c.code == Code::Error
                && let Some((cmd, line, enclosing)) = acts.last().and_then(|parent| {
                    parent
                        .asm
                        .instructions
                        .get(parent.pc.saturating_sub(1))
                        .map(logged_command)
                })
            {
                let message = c.result.string_bytes();
                self.log_command_with_enclosing(
                    &cmd,
                    Value::from_native_string_bytes(cmd.clone()),
                    &enclosing,
                    &message,
                    line,
                );
            }
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

    pub(super) fn fold_expression_result(
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
            let key = match self.native_name_operand_bytes_checked(&c.result) {
                Ok(key) => key,
                Err(refusal) => {
                    return Err(crate::command::completion_from_cmd_error(self, refusal));
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
                // TEBC holds a substitution's result object by its own
                // reference before the next command resets the result.
                .resume(c.result.into_native_reference());
            return Ok(());
        }
        if c.code == Code::Error {
            self.settle_expression_substitution_error();
        }
        Err(c)
    }

    /// An error leaving an expression's command substitution. From 8.5 the
    /// expression ends as `Tcl_EvalObjEx` does, clearing `ERR_ALREADY_LOGGED`,
    /// so each command enclosing the expression logs its own level. 8.4's
    /// `Tcl_ExprObj` runs it without that clear, so an error the substitution
    /// logged stays logged through every enclosing command up to a procedure
    /// boundary
    /// ([`tcl_dialect::TclVersion::keeps_expression_substitution_errors_logged`]).
    /// Measured against tclsh 8.4.20, 8.5.19 and 8.6.16 on
    /// `set w [list a [expr $e]]` with `e` holding `[p] + 1`.
    pub(crate) fn settle_expression_substitution_error(&mut self) {
        if !self
            .runtime_version()
            .keeps_expression_substitution_errors_logged()
        {
            self.clear_error_logged();
        } else if self.command_error_is_logged() {
            self.mark_error_logged_through_frames();
        }
    }

    /// Fold a subst `[…]` bracket body's completion into the parent subst frame's
    /// scan, per C's per-bracket `subst` rules (subst-8.x/10.x): `Ok`/`Return`/any
    /// other code appends the value and `continue` drops it (both `Resume` the
    /// scan — the caller re-ticks the subst frame); a `break` finalises the result
    /// with the output accumulated so far and an error propagates (both `Unwind`,
    /// dropping the subst frame). The scan state is cleared on `Unwind` so the
    /// dropped frame delivers its `ok`/error result normally.
    pub(super) fn fold_subst_bracket(
        &mut self,
        parent: &mut Frame,
        mut c: Completion<Value>,
    ) -> SubstFold {
        let state = parent.subst.as_mut().expect("subst state present");
        if let Some(base) = state.pending_array.take()
            && c.code == Code::Ok
        {
            let key = match self.native_name_operand_bytes_checked(&c.result) {
                Ok(key) => key,
                Err(refusal) => {
                    parent.subst = None;
                    return SubstFold::Unwind(crate::command::completion_from_cmd_error(
                        self, refusal,
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
                let v = match self.native_name_operand_bytes_checked(&c.result) {
                    Ok(bytes) => bytes,
                    Err(refusal) => {
                        parent.subst = None;
                        return SubstFold::Unwind(crate::command::completion_from_cmd_error(
                            self, refusal,
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

    /// One scan step of a subst activation: append the next literal / `$…` run and
    /// either finish (`Return` with the accumulated output), pause for a top-level
    /// `[…]` (compile it and push a yieldable child script frame), or fail. The
    /// bracket's completion is folded back into the scan state by the subst rules
    /// in [`Vm::unwind`].
    pub(super) fn tick_subst(&mut self, f: &mut Frame) -> Tick {
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
    pub(super) fn tick_each_loop(&mut self, f: &mut Frame) -> Tick {
        use tcl_cmd_core::native_each_loop::EachLoopAction;
        use tcl_runtime_api::native_each_loop::NativeEachLoopKind;
        let Some(strings) = self.object_protocol() else {
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
            tcl_native_model::native_eval_object::EvalObjectPurpose::ControlBody,
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
    pub(super) fn tick_expression(&mut self, frame: &mut Frame) -> Tick {
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
                .resume(completion.result.into_native_reference());
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
            match crate::expr::normalize_result_for_vm(self, &value, tcl_native_model::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi) {
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
            let qualified = tcl_registry::mathfunc::qualified_name(function);
            let name = tcl_syntax::naming::unroot_rooted_key(&qualified).unwrap_or(&qualified);
            // TIP 232 uses ordinary namespace-relative command resolution.
            // Unknown math functions bypass the user unknown handler.
            if self.lookup_command(name).is_none() {
                return Err(Box::new(Tick::Return(
                    crate::command::completion_from_tcl_error(
                        self,
                        if self.runtime_version().has_lookup_error_code("COMMAND") {
                            crate::error::TclError::with_error_code(
                                format!("invalid command name \"{name}\""),
                                format!("TCL LOOKUP COMMAND {name}"),
                            )
                        } else {
                            crate::error::TclError::new(format!("invalid command name \"{name}\""))
                        },
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

    pub(super) fn tick_control(&mut self, frame: &mut Frame) -> Tick {
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
                    match self.prepare_original_expression_value(&expression) {
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
    pub(super) fn fold_each_loop(
        parent: &mut Frame,
        completion: Completion<Value>,
    ) -> EachLoopFold {
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
}
