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

//! Explicit simulator scheduling over retained physical activations.

use std::collections::HashSet;
use std::rc::{Rc, Weak};

use tcl_runtime_api::Completion;
use tcl_runtime_api::retained_activation::{
    RetainedActivationRefusal, ScheduledCallbackOutcome, ScheduledCallbackReport,
};

use crate::Value;
use crate::compiled::NativeCompilerPolicy;
use crate::frame::ActivationIdentity;
use crate::interp::{CommandSidecarHandle, InterpId, Vm, err, ok};

/// A concrete activation minted by this VM. Its frame lifetime is borrowed;
/// retaining this value cannot keep a deleted connection or interpreter alive.
#[derive(Clone, Debug)]
pub struct RetainedActivation {
    owner: u64,
    interpreter: InterpId,
    frame: Weak<ActivationIdentity>,
    epoch: Weak<ActivationIdentity>,
    serial: u64,
    coroutine: Option<CommandSidecarHandle>,
    source_policy: tcl_dialect::DialectProfileKey,
    invocation_policy: NativeCompilerPolicy,
}

#[derive(Clone)]
pub(crate) struct TimerContext {
    event: Value,
    rule: Value,
    pub(crate) root: Option<bool>,
    pub(crate) static_context: Option<tcl_runtime_api::authored_tmm::TmmStaticExecutionContext>,
    pub(crate) scope: Option<Weak<ActivationIdentity>>,
}

struct Timer {
    id: u64,
    deadline: u64,
    periodic: Option<u64>,
    script: Value,
    activation: RetainedActivation,
    rule_epoch: Weak<ActivationIdentity>,
    context: TimerContext,
}

#[derive(Default)]
pub(crate) struct AuthoredTimers {
    pub(crate) installed: bool,
    now: u64,
    next_id: u64,
    queue: Vec<Timer>,
    running: Vec<(InterpId, u64)>,
    cancelled: HashSet<u64>,
}

impl Vm {
    /// Install the authored iRules timer simulator in this VM and future
    /// children. This grants no vendor-native callback or compiler contract.
    pub fn install_irules_timer_simulation(&mut self) {
        self.authored_timers.installed = true;
        register_provider(self);
    }

    /// Capture the current physical call frame for an explicit host callback.
    /// The receipt does not infer an event's connection frame from its name.
    #[must_use]
    pub fn retain_current_activation(&self) -> RetainedActivation {
        self.retain_activation(false)
    }

    fn retain_activation(&self, root: bool) -> RetainedActivation {
        let frame = self.retained_frame_identity(root);
        RetainedActivation {
            owner: self.owner_nonce,
            interpreter: self.cur_interp(),
            frame: Rc::downgrade(&frame),
            epoch: Rc::downgrade(&self.timer_epoch),
            serial: frame.serial,
            coroutine: if root {
                None
            } else {
                crate::cmd_coro::current_activation_handle(self)
            },
            source_policy: self.source_profile().cache_key(),
            invocation_policy: self.native_compiler_policy(),
        }
    }

    /// Execute in the original retained frame, preserving complete guest
    /// options and engine refusal. A reused frame level is never sufficient.
    pub fn eval_retained_activation(
        &mut self,
        activation: &RetainedActivation,
        source: &str,
    ) -> ScheduledCallbackOutcome<Value> {
        self.eval_retained_activation_value(
            activation,
            &Value::from_native_string_bytes(source.as_bytes()),
        )
    }

    /// Execute the original source object in its retained physical activation.
    /// Byte materialisation uses that interpreter's selected native issuer;
    /// source location, guest options and host refusal remain intact.
    pub fn eval_retained_activation_value(
        &mut self,
        activation: &RetainedActivation,
        source: &Value,
    ) -> ScheduledCallbackOutcome<Value> {
        if activation.owner != self.owner_nonce {
            return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ForeignRuntime);
        }
        if !self.interp_alive(activation.interpreter) {
            return ScheduledCallbackOutcome::Activation(
                RetainedActivationRefusal::RetiredInterpreter,
            );
        }
        if activation.epoch.upgrade().is_none() {
            return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame);
        }
        let Some(frame) = activation
            .frame
            .upgrade()
            .filter(|frame| frame.serial == activation.serial)
        else {
            return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame);
        };
        self.in_interp(activation.interpreter, |vm| {
            if vm.source_profile().cache_key() != activation.source_policy
                || vm.native_compiler_policy() != activation.invocation_policy
            {
                return ScheduledCallbackOutcome::Activation(
                    RetainedActivationRefusal::ChangedPolicy,
                );
            }
            if let Some(level) = vm.retained_frame_level(&frame) {
                return execution_outcome(vm.eval_retained_frame(level, source));
            }
            let Some(coroutine) = &activation.coroutine else {
                return ScheduledCallbackOutcome::Activation(
                    RetainedActivationRefusal::RetiredFrame,
                );
            };
            crate::cmd_coro::eval_retained_activation(vm, coroutine, &frame, source)
        })
    }

    /// Advance the simulator's logical clock and drain timers due at this
    /// instant in deadline/registration order. Reentrant callbacks may cancel
    /// or register timers. Host refusal stops this drain immediately.
    pub fn advance_irules_time(
        &mut self,
        milliseconds: u64,
    ) -> Vec<ScheduledCallbackReport<Value>> {
        self.authored_timers.now = self.authored_timers.now.saturating_add(milliseconds);
        let horizon = self.authored_timers.now;
        let cutoff = self.authored_timers.next_id;
        let mut reports = Vec::new();
        loop {
            let next = self
                .authored_timers
                .queue
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.deadline <= horizon && timer.id <= cutoff)
                .min_by_key(|(_, timer)| (timer.deadline, timer.id))
                .map(|(index, _)| index);
            let Some(index) = next else {
                break;
            };
            let timer = self.authored_timers.queue.remove(index);
            self.authored_timers
                .running
                .push((timer.activation.interpreter, timer.id));
            let mut context = timer.context.clone();
            context.scope = None;
            let interpreter = timer.activation.interpreter;
            let outcome = if !self.interp_alive(interpreter) {
                ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredInterpreter)
            } else if timer.rule_epoch.upgrade().is_none() {
                ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredRule)
            } else {
                self.in_interp(interpreter, |vm| {
                    let saved = vm.timer_contexts.len();
                    // Metadata follows the authenticated activation; it is not a frame proof.
                    vm.timer_contexts.push(context.clone());
                    let result = vm.with_timer_metadata(&context.event, &context.rule, |vm| {
                        vm.eval_retained_activation_value(&timer.activation, &timer.script)
                    });
                    vm.timer_contexts.truncate(saved);
                    result
                })
            };
            self.authored_timers.running.pop();
            let stopped = matches!(outcome, ScheduledCallbackOutcome::Host(_));
            let repeat = timer
                .periodic
                .filter(|_| matches!(outcome, ScheduledCallbackOutcome::Guest(_)));
            let cancelled = self.authored_timers.cancelled.remove(&timer.id);
            reports.push(ScheduledCallbackReport {
                id: timer.id,
                event: context.event,
                rule: context.rule,
                outcome,
            });
            if let Some(period) = repeat.filter(|_| !cancelled) {
                // Missed intervals coalesce; one periodic dispatch per advance.
                if let Some(deadline) = horizon.checked_add(period) {
                    self.authored_timers.queue.push(Timer { deadline, ..timer });
                }
            }
            if stopped {
                break;
            }
        }
        reports
    }
}

pub(crate) fn execution_outcome(
    result: Result<Completion<Value>, tcl_runtime_api::NativeExecutionError>,
) -> ScheduledCallbackOutcome<Value> {
    match result {
        Ok(completion) => ScheduledCallbackOutcome::Guest(completion),
        Err(refusal) => ScheduledCallbackOutcome::Host(refusal),
    }
}

pub(crate) fn register_provider(vm: &mut Vm) {
    vm.register("after", cmd_after);
    vm.register("::tmm::_timer_anchor", cmd_anchor);
    vm.register("::tmm::_timer_context_enter", cmd_context_enter);
    vm.register("::tmm::_timer_context_leave", cmd_context_leave);
    vm.register("::tmm::_timer_retire", cmd_retire);
    vm.register("::tmm::_timer_rule_loaded", cmd_rule_loaded);
}

fn cmd_anchor(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("wrong # args: should be \"::tmm::_timer_anchor\"");
    }
    if crate::cmd_coro::current_activation_handle(vm).is_none() {
        return vm.refuse_host_command(
            "a connection timer anchor requires an actual coroutine activation".into(),
        );
    }
    vm.timer_anchor = Some(vm.retain_current_activation());
    ok(Value::empty())
}

fn cmd_context_enter(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [event, rule] = args else {
        return err("timer context requires event and rule");
    };
    let event_text = match event.try_to_str() {
        Ok(text) => text,
        Err(refusal) => return vm.refuse_host_command(refusal.to_string()),
    };
    let root = match tcl_registry::events::EventRegistry::build().variable_frame(&event_text) {
        tcl_registry::events::EventVariableFrame::InitialisationNamespace => Some(true),
        tcl_registry::events::EventVariableFrame::Connection => Some(false),
        tcl_registry::events::EventVariableFrame::Unknown => None,
    };
    vm.timer_contexts.retain(|context| {
        context
            .scope
            .as_ref()
            .is_none_or(|scope| scope.strong_count() != 0)
    });
    let frame = vm.retained_frame_identity(false);
    vm.timer_contexts.push(TimerContext {
        event: event.clone(),
        rule: rule.clone(),
        root,
        static_context: root.map(|initialisation| {
            if initialisation {
                tcl_runtime_api::authored_tmm::TmmStaticExecutionContext::InitialisationBroadcast
            } else {
                tcl_runtime_api::authored_tmm::TmmStaticExecutionContext::ExecutingWorker
            }
        }),
        scope: Some(Rc::downgrade(&frame)),
    });
    ok(Value::empty())
}

fn cmd_rule_loaded(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [identity] = args else {
        return err("rule timer ownership requires one identity");
    };
    vm.timer_rules.insert(
        identity.string_bytes().to_vec(),
        Rc::new(ActivationIdentity::new()),
    );
    ok(Value::empty())
}

fn cmd_retire(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("timer retirement takes no arguments");
    }
    vm.timer_epoch = Rc::new(ActivationIdentity::new());
    vm.timer_anchor = None;
    vm.timer_contexts.clear();
    vm.timer_rules.clear();
    ok(Value::empty())
}

fn cmd_context_leave(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("timer context leave takes no arguments");
    }
    vm.timer_contexts.pop();
    ok(Value::empty())
}

fn timer_id(value: &Value) -> Option<u64> {
    let bytes = value.string_bytes();
    let tail = bytes.strip_prefix(b"after#")?;
    std::str::from_utf8(tail).ok()?.parse().ok()
}

fn cancel_authored_timers(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [_, targets @ ..] = args else {
        unreachable!()
    };
    if targets.is_empty() {
        return err("after cancel requires a timer identifier or -current");
    }
    let interpreter = vm.cur_interp();
    for target in targets {
        let id = if target.string_bytes().as_ref() == b"-current" {
            vm.authored_timers
                .running
                .last()
                .filter(|(owner, _)| *owner == interpreter)
                .map(|(_, id)| *id)
        } else {
            timer_id(target)
        };
        if let Some(id) = id {
            vm.authored_timers
                .queue
                .retain(|timer| timer.id != id || timer.activation.interpreter != interpreter);
            if vm.authored_timers.running.contains(&(interpreter, id)) {
                vm.authored_timers.cancelled.insert(id);
            }
        }
    }
    ok(Value::empty())
}

fn authored_timer_info(vm: &Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [_] => ok(Value::list(
            vm.authored_timers
                .queue
                .iter()
                .filter(|timer| timer.activation.interpreter == vm.cur_interp())
                .map(|timer| Value::string(format!("after#{}", timer.id)))
                .collect(),
        )),
        [_, id] => match timer_id(id).and_then(|id| {
            vm.authored_timers
                .queue
                .iter()
                .find(|timer| timer.id == id && timer.activation.interpreter == vm.cur_interp())
        }) {
            Some(timer) => ok(Value::list(vec![
                timer.script.clone(),
                Value::string("timer"),
            ])),
            None => err("event does not exist"),
        },
        _ => err("after info takes at most one timer identifier"),
    }
}

fn cmd_after(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(first) = args.first() else {
        return err("after requires a delay or subcommand");
    };
    let first_bytes = first.string_bytes();
    if first_bytes.as_ref() == b"cancel" {
        return cancel_authored_timers(vm, args);
    }
    if first_bytes.as_ref() == b"info" {
        return authored_timer_info(vm, args);
    }
    let delay = first;
    let (periodic, script) = if args
        .get(1)
        .is_some_and(|value| value.string_bytes().as_ref() == b"-periodic")
    {
        (true, &args[2..])
    } else {
        (false, &args[1..])
    };
    if script.is_empty() {
        return vm.refuse_host_command(
            "bare simulator delay requires a retained resumable continuation".into(),
        );
    }
    let milliseconds = match tcl_syntax::value::ValueOps::as_int(vm, delay) {
        Ok(value) => match u64::try_from(value) {
            Ok(value) if !periodic || value != 0 => value,
            _ => return err("timer delay must be nonnegative and periodic delay must be positive"),
        },
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    vm.timer_contexts.retain(|context| {
        context
            .scope
            .as_ref()
            .is_none_or(|scope| scope.strong_count() != 0)
    });
    let Some(mut context) = vm.timer_contexts.last().cloned() else {
        return vm
            .refuse_host_command("iRules timers require an explicit active event context".into());
    };
    // Root-frame timer execution is not a reached RULE_INIT handler.
    context.static_context =
        Some(tcl_runtime_api::authored_tmm::TmmStaticExecutionContext::ExecutingWorker);
    if let Some(rule) = vm.get_var("::itest::_current_rule") {
        context.rule = rule;
    }
    let Some(rule_epoch) = vm
        .timer_rules
        .get(context.rule.string_bytes().as_ref())
        .map(Rc::downgrade)
    else {
        return vm.refuse_host_command(
            "iRules timers require the original loaded rule ownership".into(),
        );
    };
    let Some(root) = context.root else {
        return vm.refuse_host_command("the event has no authored simulator callback frame".into());
    };
    let activation = if root {
        vm.retain_activation(true)
    } else {
        let Some(anchor) = vm.timer_anchor.clone() else {
            return vm.refuse_host_command(
                "iRules timers require the actual connection activation anchor".into(),
            );
        };
        // The anchor supplies only the physical connection. The reached
        // scheduling call supplies the original user invocation policy.
        RetainedActivation {
            invocation_policy: vm.native_compiler_policy(),
            epoch: Rc::downgrade(&vm.timer_epoch),
            ..anchor
        }
    };
    let script = if let [original] = script {
        original.clone()
    } else {
        match tcl_cmd_core::list::concat_selected(vm, script) {
            Ok(script) => script,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    };
    let Some(id) = vm.authored_timers.next_id.checked_add(1) else {
        return err("timer identity space exhausted");
    };
    let Some(deadline) = vm.authored_timers.now.checked_add(milliseconds) else {
        return err("timer deadline exceeds logical clock");
    };
    vm.authored_timers.next_id = id;
    vm.authored_timers.queue.push(Timer {
        id,
        deadline,
        periodic: periodic.then_some(milliseconds),
        script,
        activation,
        rule_epoch,
        context,
    });
    ok(Value::string(format!("after#{id}")))
}
