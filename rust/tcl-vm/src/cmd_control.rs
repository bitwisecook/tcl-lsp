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

//! Control-flow *fallback* commands — `if` / `while` / `for` / `foreach` /
//! `lmap`.
//!
//! The VM compiles these inline for the static, literal forms; this module
//! supplies the **runtime command** forms the codegen falls back to when a
//! construct cannot be inlined — a computed command name (`set z if; $z …`), a
//! dynamic/computed body, or a malformed-grammar barrier. Without them the VM
//! reports `invalid command name "if"` for every such case (the bulk of the
//! `if`/`while` tcltest gap vs. the tree-walking `runtime/rust`).
//!
//! They mirror C's `Tcl_IfObjCmd`/`Tcl_WhileObjCmd`/`Tcl_ForObjCmd` /
//! `EachloopCmd` and `runtime/rust`'s `cmd_control.rs`, evaluating bodies as
//! transparent scripts (`break`/`continue`/`return` propagate) in the current
//! frame via [`Vm::eval_source`], and conditions as Tcl expressions via
//! [`Vm::eval_expr`]. Semantics pinned against tclsh 9.0.

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("if", cmd_if);
    vm.register_stock_builtin("while", cmd_while);
    vm.register_stock_builtin("for", cmd_for);
    vm.register_stock_builtin("foreach", cmd_foreach);
    vm.register_stock_builtin("lmap", cmd_lmap);
}

/// `wrong # args: no script following "<token>" argument` (C's `missingScript`).
fn no_script_following(token: &str) -> Completion<Value> {
    err(format!(
        "wrong # args: no script following \"{token}\" argument"
    ))
}

// if

/// `if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?`.
fn cmd_if(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    vm.pending.control = Some(ControlState::if_command(args.to_vec()));
    ok(Value::empty())
}

// while

/// `while test body`.
fn cmd_while(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() != 2 {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"while test command\"",
        );
    }
    vm.pending.control = Some(ControlState::while_loop(args[0].clone(), args[1].clone()));
    ok(Value::empty())
}

// for

/// `for start test next body`.
fn cmd_for(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() != 4 {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"for start test next command\"",
        );
    }
    vm.pending.control = Some(ControlState::for_loop(args));
    ok(Value::empty())
}

// foreach / lmap

/// `foreach varList list ?varList list ...? body`.
fn cmd_foreach(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    each_loop(vm, args, false)
}

/// `lmap varList list ?varList list ...? body` — `foreach` that collects each
/// (non-`continue`) body result into a list and returns it.
fn cmd_lmap(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    each_loop(vm, args, true)
}

/// Generic loops retain actual argument objects and prepare the body only after
/// the selected native setter sequence reaches an iteration.
fn each_loop(vm: &mut Vm, args: &[Value], collect: bool) -> Completion<Value> {
    use crate::exec::{EachLoopGroup, EachLoopRoot};
    use tcl_runtime_api::native_each_loop::NativeEachLoopKind;
    let kind = if collect {
        NativeEachLoopKind::Lmap
    } else {
        NativeEachLoopKind::Foreach
    };
    if args.len() < 3 || args.len() % 2 != 1 {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!(
                "wrong # args: should be \"{} varList list ?varList list ...? command\"",
                kind.name()
            ),
        );
    }
    let dialect = vm.native_invocation_dialect();
    let Some(protocol) = dialect.native_each_loop_protocol(kind) else {
        return vm.refuse_host_command("native generic each-loop protocol is unavailable".into());
    };
    let Some(strings) = dialect.native_string_protocol() else {
        return vm
            .refuse_host_command("native each-loop List storage protocol is unavailable".into());
    };
    let recipe = protocol.recipe();
    let mut groups = Vec::new();
    let mut lengths = Vec::new();
    let mut empty_variables = false;
    for pair in args[..args.len() - 1].chunks_exact(2) {
        let variable_root = if recipe.copies_headers() {
            match pair[0].native_list_copy(strings) {
                Ok(root) => EachLoopRoot::Header(root),
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            }
        } else {
            EachLoopRoot::Original(pair[0].native_lifetime_lease())
        };
        let variables = match vm.native_object_list_elements_in(variable_root.value(), strings) {
            Ok(items) => items,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        };
        empty_variables |= variables.is_empty();
        if variables.is_empty() && !recipe.live_iterators() {
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::native_each_loop::empty_variables(recipe, kind),
            );
        }
        let value_root = if recipe.copies_headers() {
            match pair[1].native_list_copy(strings) {
                Ok(root) => EachLoopRoot::Header(root),
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            }
        } else {
            EachLoopRoot::Original(pair[1].native_lifetime_lease())
        };
        let values = if recipe.live_iterators() {
            None
        } else {
            match vm.native_object_list_elements_in(value_root.value(), strings) {
                Ok(items) => Some(items),
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            }
        };
        lengths.push((
            variables.len(),
            values.as_ref().map_or(0, |items| items.len()),
        ));
        groups.push(EachLoopGroup {
            variables: variable_root,
            values: value_root,
            variable_items: Some(variables),
            value_items: values,
        });
    }
    if empty_variables {
        return crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::native_each_loop::empty_variables(recipe, kind),
        );
    }
    let jim_empty = if recipe.live_iterators() {
        match vm.native_jim_object_context() {
            Ok(context) => Some(if kind == NativeEachLoopKind::Lmap {
                EachLoopRoot::Original(context.empty_object().native_lifetime_lease())
            } else {
                // Jim's foreach resultObj owns emptyObj; lmap owns a List
                // instead and only borrows emptyObj when padding variables.
                EachLoopRoot::Header(context.empty_object().clone())
            }),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        }
    } else {
        None
    };
    vm.pending.each_loop = Some(crate::exec::EachLoopReq {
        protocol,
        groups,
        cursor: tcl_cmd_core::native_each_loop::EachLoopState::new(recipe, kind, lengths),
        body: args[args.len() - 1].native_lifetime_lease(),
        _arguments: vm
            .native_invocation
            .arguments
            .as_ref()
            .filter(|words| {
                words.len() == args.len() + 1
                    && words[1..]
                        .iter()
                        .zip(args)
                        .all(|(left, right)| left.is_same_object(right))
            })
            .map(crate::NativeListItems::lifetime_view),
        jim_empty,
        invocation: None,
    });
    ok(Value::empty())
}

/// Runtime controls are continuations on the interpreter activation stack.
/// Their scripts keep real Tcl frames and may suspend without native re-entry.
pub(crate) struct ControlState {
    kind: ControlKind,
    phase: ControlPhase,
    finished: Option<Completion<Value>>,
}

enum ControlKind {
    ObjectEval {
        original: Value,
        words: crate::NativeListItems,
        label: &'static str,
        restore: Option<crate::interp::SelectedFrameRestore>,
        report_body_frame: bool,
    },
    If {
        arguments: Vec<Value>,
        cursor: usize,
        clause: &'static str,
        tail: bool,
        chosen: Option<Value>,
        condition_body: Option<Value>,
    },
    While {
        condition: Value,
        body: Value,
    },
    For {
        start: Value,
        condition: Value,
        next: Value,
        body: Value,
    },
}

#[derive(Clone, Copy)]
enum ControlPhase {
    ObjectCommand,
    AwaitObjectCommand,
    IfScan,
    Start,
    Condition,
    Body,
    Next,
    AwaitStart,
    AwaitCondition,
    AwaitBody,
    AwaitNext,
    Done,
}

pub(crate) enum ControlStep {
    Invocation(crate::NativeListItems),
    ResumeInvocation,
    Expression(Value),
    Script(Value),
    Complete(Completion<Value>),
}

impl ControlState {
    pub(crate) fn selected_frame_restore(&self) -> Option<&crate::interp::SelectedFrameRestore> {
        match &self.kind {
            ControlKind::ObjectEval { restore, .. } => restore.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn take_selected_frame_restore(
        &mut self,
    ) -> Option<crate::interp::SelectedFrameRestore> {
        match &mut self.kind {
            ControlKind::ObjectEval { restore, .. } => restore.take(),
            _ => None,
        }
    }

    pub(crate) fn object_eval(
        original: Value,
        words: crate::NativeListItems,
        label: &'static str,
        restore: Option<crate::interp::SelectedFrameRestore>,
    ) -> Self {
        Self {
            kind: ControlKind::ObjectEval {
                original,
                words,
                label,
                restore,
                report_body_frame: true,
            },
            phase: ControlPhase::ObjectCommand,
            finished: None,
        }
    }

    pub(crate) fn object_body_eval(original: Value, words: crate::NativeListItems) -> Self {
        let mut state = Self::object_eval(original, words, "foreach", None);
        if let ControlKind::ObjectEval {
            report_body_frame, ..
        } = &mut state.kind
        {
            *report_body_frame = false;
        }
        state
    }

    pub(crate) fn take_object_context(
        &mut self,
    ) -> Option<(&'static str, Option<crate::interp::SelectedFrameRestore>)> {
        match &mut self.kind {
            ControlKind::ObjectEval {
                original,
                label,
                restore,
                report_body_frame,
                ..
            } => {
                // The activation keeps the original container even if a callback
                // replaces its internal representation during dispatch.
                let _ = original;
                report_body_frame.then(|| (*label, restore.take()))
            }
            _ => None,
        }
    }

    fn if_command(arguments: Vec<Value>) -> Self {
        Self {
            kind: ControlKind::If {
                arguments,
                cursor: 0,
                clause: "if",
                tail: false,
                chosen: None,
                condition_body: None,
            },
            phase: ControlPhase::IfScan,
            finished: None,
        }
    }
    fn while_loop(condition: Value, body: Value) -> Self {
        Self {
            kind: ControlKind::While { condition, body },
            phase: ControlPhase::Condition,
            finished: None,
        }
    }
    fn for_loop(arguments: &[Value]) -> Self {
        Self {
            kind: ControlKind::For {
                start: arguments[0].clone(),
                condition: arguments[1].clone(),
                next: arguments[2].clone(),
                body: arguments[3].clone(),
            },
            phase: ControlPhase::Start,
            finished: None,
        }
    }
    fn finish(&mut self, completion: Completion<Value>) {
        self.finished = Some(completion);
        self.phase = ControlPhase::Done;
    }
    pub(crate) fn accept(&mut self, vm: &mut Vm, completion: Completion<Value>) {
        match self.phase {
            ControlPhase::AwaitObjectCommand => self.finish(completion),
            ControlPhase::AwaitCondition => {
                if completion.code != Code::Ok {
                    self.finish(completion);
                    return;
                }
                let value =
                    match crate::expr::native_boolean(vm.numeric_context(), &completion.result) {
                        Ok(value) => value,
                        Err(error) => {
                            self.finish(crate::command::completion_from_tcl_error(vm, error));
                            return;
                        }
                    };
                match &mut self.kind {
                    ControlKind::If {
                        chosen,
                        condition_body,
                        ..
                    } => {
                        if value {
                            *chosen = condition_body.take();
                        }
                        self.phase = ControlPhase::IfScan;
                    }
                    ControlKind::ObjectEval { .. } => unreachable!(),
                    ControlKind::While { .. } | ControlKind::For { .. } => {
                        if value {
                            self.phase = ControlPhase::Body;
                        } else {
                            self.finish(ok(Value::empty()));
                        }
                    }
                }
            }
            ControlPhase::AwaitStart => {
                if completion.code == Code::Ok {
                    self.phase = ControlPhase::Condition;
                } else {
                    self.finish(completion);
                }
            }
            ControlPhase::AwaitNext => match completion.code {
                Code::Ok => self.phase = ControlPhase::Condition,
                Code::Break => self.finish(ok(Value::empty())),
                _ => self.finish(completion),
            },
            ControlPhase::AwaitBody => {
                let name = match self.kind {
                    ControlKind::If { .. } => {
                        self.finish(completion);
                        return;
                    }
                    ControlKind::ObjectEval { .. } => unreachable!(),
                    ControlKind::While { .. } => "while",
                    ControlKind::For { .. } => "for",
                };
                match completion.code {
                    Code::Ok | Code::Continue => {
                        self.phase = if matches!(self.kind, ControlKind::For { .. }) {
                            ControlPhase::Next
                        } else {
                            ControlPhase::Condition
                        };
                    }
                    Code::Break => self.finish(ok(Value::empty())),
                    Code::Error => {
                        vm.append_body_frame(name);
                        self.finish(completion);
                    }
                    _ => self.finish(completion),
                }
            }
            _ => {
                self.finish(err("invalid native control continuation"));
            }
        }
    }
    pub(crate) fn next(&mut self) -> ControlStep {
        match self.phase {
            ControlPhase::ObjectCommand => {
                let ControlKind::ObjectEval { words, .. } = &self.kind else {
                    unreachable!()
                };
                self.phase = ControlPhase::AwaitObjectCommand;
                ControlStep::Invocation(words.lifetime_view())
            }
            ControlPhase::AwaitObjectCommand => ControlStep::ResumeInvocation,
            ControlPhase::Done => {
                ControlStep::Complete(self.finished.take().unwrap_or_else(|| ok(Value::empty())))
            }
            ControlPhase::Start => {
                let ControlKind::For { start, .. } = &self.kind else {
                    unreachable!()
                };
                self.phase = ControlPhase::AwaitStart;
                ControlStep::Script(start.clone())
            }
            ControlPhase::Condition => {
                let condition = match &self.kind {
                    ControlKind::While { condition, .. } | ControlKind::For { condition, .. } => {
                        condition.clone()
                    }
                    ControlKind::If { .. } | ControlKind::ObjectEval { .. } => unreachable!(),
                };
                self.phase = ControlPhase::AwaitCondition;
                ControlStep::Expression(condition)
            }
            ControlPhase::Body => {
                let body = match &self.kind {
                    ControlKind::While { body, .. } | ControlKind::For { body, .. } => body.clone(),
                    ControlKind::If { chosen, .. } => chosen.clone().unwrap_or_else(Value::empty),
                    ControlKind::ObjectEval { .. } => unreachable!(),
                };
                self.phase = ControlPhase::AwaitBody;
                ControlStep::Script(body)
            }
            ControlPhase::Next => {
                let ControlKind::For { next, .. } = &self.kind else {
                    unreachable!()
                };
                self.phase = ControlPhase::AwaitNext;
                ControlStep::Script(next.clone())
            }
            ControlPhase::IfScan => self.next_if_clause(),
            _ => ControlStep::Complete(err("native control resumed before its child completed")),
        }
    }
    fn next_if_clause(&mut self) -> ControlStep {
        let ControlKind::If {
            arguments,
            cursor,
            clause,
            tail,
            chosen,
            condition_body,
        } = &mut self.kind
        else {
            unreachable!()
        };
        loop {
            if *tail {
                if *cursor >= arguments.len() {
                    break;
                }
                if &*arguments[*cursor].to_str() == "elseif" {
                    *cursor += 1;
                    *clause = "elseif";
                    *tail = false;
                    continue;
                }
                if &*arguments[*cursor].to_str() == "else" {
                    *cursor += 1;
                    if *cursor >= arguments.len() {
                        return ControlStep::Complete(no_script_following("else"));
                    }
                }
                if *cursor + 1 < arguments.len() {
                    return ControlStep::Complete(err(
                        "wrong # args: extra words after \"else\" clause in \"if\" command",
                    ));
                }
                if chosen.is_none() {
                    *chosen = arguments.get(*cursor).cloned();
                }
                break;
            }
            if *cursor >= arguments.len() {
                return ControlStep::Complete(err(format!(
                    "wrong # args: no expression after \"{clause}\" argument"
                )));
            }
            let condition = arguments[*cursor].clone();
            *cursor += 1;
            if *cursor < arguments.len() && &*arguments[*cursor].to_str() == "then" {
                *cursor += 1;
            }
            if *cursor >= arguments.len() {
                return ControlStep::Complete(no_script_following(
                    &arguments[*cursor - 1].to_str(),
                ));
            }
            let body = arguments[*cursor].clone();
            *cursor += 1;
            *tail = true;
            if chosen.is_none() {
                *condition_body = Some(body);
                self.phase = ControlPhase::AwaitCondition;
                return ControlStep::Expression(condition);
            }
        }
        if chosen.is_some() {
            self.phase = ControlPhase::Body;
            self.next()
        } else {
            self.finish(ok(Value::empty()));
            self.next()
        }
    }
}

#[cfg(test)]
mod native_each_loop_tests;
