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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NativeCommand;
    use tcl_runtime_api::Code;

    fn vm() -> Vm {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::default(),
        ));
        vm.install_irules_timer_simulation();
        vm
    }

    fn eval(vm: &mut Vm, source: &str) -> Completion<Value> {
        let completion = vm.try_eval_source(source).expect("actual engine execution");
        assert_eq!(completion.code, Code::Ok, "{:?}", completion.result);
        completion
    }

    fn root_context(vm: &mut Vm) {
        assert!(cmd_rule_loaded(vm, &[Value::empty()]).is_ok());
        assert!(cmd_context_enter(vm, &[Value::string("RULE_INIT"), Value::empty()]).is_ok());
    }

    #[test]
    fn ordinary_tcl_after_keeps_its_global_callback_frame() {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::default(),
        ));
        eval(
            &mut vm,
            "proc schedule {} {set local 1; after 0 {set observed [info exists local]}}; schedule; update",
        );
        assert_eq!(
            vm.get_var("observed").unwrap().string_bytes().as_ref(),
            b"0"
        );
        assert!(!vm.authored_timers.installed);
    }

    #[test]
    fn retained_frame_rejects_reused_levels_foreign_vms_and_deleted_interpreters() {
        let mut original = vm();
        let mut foreign = vm();
        original.push_call_frame(None, Vec::new());
        let capability = original.retain_current_activation();
        assert!(matches!(
            foreign.eval_retained_activation(&capability, "set changed 1"),
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ForeignRuntime)
        ));
        original.pop_call_frame();
        original.push_call_frame(None, Vec::new());
        assert!(matches!(
            original.eval_retained_activation(&capability, "set changed 1"),
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame)
        ));
        assert!(original.get_var("changed").is_none());
        original.pop_call_frame();
        let name = original.create_child(Some("worker".into()), false);
        let child = original.child_id(&name).unwrap();
        let child_activation = original.in_interp(child, |vm| vm.retain_current_activation());
        assert!(original.delete_interp(child));
        assert!(matches!(
            original.eval_retained_activation(&child_activation, "set changed 1"),
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredInterpreter)
        ));
    }

    #[test]
    fn scheduled_connection_uses_original_renamed_coroutine_and_expires_on_deletion() {
        let mut vm = vm();
        eval(
            &mut vm,
            "::tmm::_timer_rule_loaded {}; ::tmm::_timer_context_enter HTTP_REQUEST {}; proc flow {} {::tmm::_timer_anchor; set local 10; after 5 {incr local}; yield; yield}; coroutine connection flow; rename connection moved",
        );
        let reports = vm.advance_irules_time(5);
        assert_eq!(reports.len(), 1);
        let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
            panic!("{:?}", reports[0]);
        };
        assert_eq!(completion.result.string_bytes().as_ref(), b"11");
        eval(
            &mut vm,
            "coroprobe moved after 5 {set local REPLACED}; rename moved {}; coroutine moved flow",
        );
        let reports = vm.advance_irules_time(5);
        assert!(reports.iter().any(|report| matches!(
            report.outcome,
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame)
        )));
        assert_eq!(
            eval(&mut vm, "coroprobe moved set local")
                .result
                .string_bytes()
                .as_ref(),
            b"11"
        );
    }

    #[test]
    fn timer_cancellation_reentrancy_and_rule_replacement_are_deterministic() {
        let mut vm = vm();
        root_context(&mut vm);
        eval(
            &mut vm,
            "set order {}; after 0 {lappend order first; after 0 {lappend order nested}}; after 0 {lappend order second}; after 2 -periodic {lappend order periodic; after cancel -current}",
        );
        assert_eq!(vm.advance_irules_time(0).len(), 2);
        assert_eq!(
            vm.get_var("order").unwrap().string_bytes().as_ref(),
            b"first second"
        );
        assert_eq!(vm.advance_irules_time(2).len(), 2);
        assert_eq!(
            vm.get_var("order").unwrap().string_bytes().as_ref(),
            b"first second nested periodic"
        );
        assert!(vm.advance_irules_time(100).is_empty());
        eval(
            &mut vm,
            "after 0 {set stale MUST_NOT_RUN}; ::tmm::_timer_rule_loaded {}",
        );
        let reports = vm.advance_irules_time(0);
        assert!(matches!(
            reports[0].outcome,
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredRule)
        ));
        assert!(vm.get_var("stale").is_none());
    }

    struct EchoOriginal;
    impl NativeCommand for EchoOriginal {
        fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            ok(args.first().cloned().unwrap_or_else(Value::empty))
        }
    }

    #[test]
    fn retained_callback_and_timer_execute_the_original_non_unicode_source() {
        let mut vm = vm();
        vm.register_native_command("echo_original", Rc::new(EchoOriginal));
        let source = Value::from_native_string_bytes(b"echo_original \xff\0tail".as_slice());
        let activation = vm.retain_current_activation();
        let ScheduledCallbackOutcome::Guest(completion) =
            vm.eval_retained_activation_value(&activation, &source)
        else {
            panic!("original byte callback must execute");
        };
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0tail");
        root_context(&mut vm);
        assert!(cmd_after(&mut vm, &[Value::int(0), source.clone()]).is_ok());
        assert_eq!(
            vm.authored_timers.queue[0].script.native_object_identity(),
            source.native_object_identity()
        );
        let reports = vm.advance_irules_time(0);
        let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
            panic!("{:?}", reports[0]);
        };
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0tail");
    }

    #[test]
    fn retained_activation_policy_includes_the_authored_name_provider() {
        let mut vm = vm();
        let source = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        vm.set_dialect_profile(source);
        assert!(vm.set_native_engine_profile(host));
        let activation = vm.retain_current_activation();
        let policy = vm.native_compiler_policy();
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert_ne!(vm.native_compiler_policy(), policy);
        assert!(matches!(
            vm.eval_retained_activation(&activation, "MUST_NOT_RUN"),
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ChangedPolicy)
        ));
    }

    #[test]
    fn retained_host_source_requires_the_original_explicit_activation() {
        let mut vm = vm();
        let user = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        vm.set_dialect_profile(user);
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_native_engine_profile(host));
        vm.active_native_profile = Some(host);
        let activation = vm.retain_current_activation();
        assert_eq!(activation.source_policy, host.cache_key());
        vm.active_native_profile = None;
        assert!(matches!(
            vm.eval_retained_activation(&activation, "list {*}{A B}"),
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ChangedPolicy)
        ));
        vm.active_native_profile = Some(host);
        let ScheduledCallbackOutcome::Guest(completion) =
            vm.eval_retained_activation(&activation, "list {*}{A B}")
        else {
            panic!("original host activation must remain usable");
        };
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(completion.result.string_bytes().as_ref(), b"A B");
        vm.active_native_profile = None;
        assert_eq!(vm.dialect_profile().cache_key(), user.cache_key());
    }

    struct Abrupt(Completion<Value>);
    impl NativeCommand for Abrupt {
        fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            self.0.clone()
        }
    }

    #[test]
    fn scheduled_guest_options_and_host_refusal_remain_separate() {
        let mut vm = vm();
        root_context(&mut vm);
        let result = Value::from_string_bytes(b"raw \xff\0result".as_slice());
        let options = crate::command::options_dict(
            Code::Error,
            3,
            &[
                (
                    "-errorcode",
                    Value::from_string_bytes(b"RAW \xfe\0code".as_slice()),
                ),
                ("-custom", Value::string("retained")),
            ],
        );
        vm.register_native_command(
            "abrupt",
            Rc::new(Abrupt(Completion::new(
                Code::Other(7),
                result.clone(),
                options.clone(),
            ))),
        );
        eval(&mut vm, "after 0 abrupt");
        let reports = vm.advance_irules_time(0);
        let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
            panic!("{:?}", reports[0]);
        };
        assert_eq!(completion.code, Code::Other(7));
        assert_eq!(completion.result.string_bytes(), result.string_bytes());
        assert_eq!(completion.options.string_bytes(), options.string_bytes());
        vm.register("refuse", |vm, _| {
            vm.refuse_host_command("reached callback backend refusal".into())
        });
        eval(
            &mut vm,
            "after 0 {set before 1; catch refuse result; set continued 1}; after 0 {set later 1}",
        );
        let reports = vm.advance_irules_time(0);
        assert_eq!(reports.len(), 1);
        assert!(matches!(
            reports[0].outcome,
            ScheduledCallbackOutcome::Host(_)
        ));
        assert_eq!(vm.get_var("before").unwrap().string_bytes().as_ref(), b"1");
        assert!(vm.get_var("continued").is_none());
        assert!(vm.get_var("later").is_none());
    }

    #[test]
    fn cancellation_cannot_remove_another_workers_timer() {
        let mut vm = vm();
        let one = vm.create_child(Some("one".into()), false);
        let two = vm.create_child(Some("two".into()), false);
        let one = vm.child_id(&one).unwrap();
        let two = vm.child_id(&two).unwrap();
        let id = vm.in_interp(one, |worker| {
            root_context(worker);
            eval(worker, "after 0 {set own 1}").result
        });
        vm.in_interp(two, |worker| {
            root_context(worker);
            assert!(cmd_after(worker, &[Value::string("cancel"), id]).is_ok());
        });
        assert_eq!(vm.advance_irules_time(0).len(), 1);
        assert_eq!(
            vm.in_interp(one, |worker| worker.get_var("own").unwrap())
                .string_bytes()
                .as_ref(),
            b"1"
        );
        assert!(vm.in_interp(two, |worker| worker.get_var("own")).is_none());
    }
}
