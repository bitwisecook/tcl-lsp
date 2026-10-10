// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event storage retains original script objects and selected global wait subjects.

use crate::interp::{Vm, ok};
use crate::value::Value;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tcl_cmd_core::event::EventKind;
use tcl_registry::native_event::{NativeEventProtocol, NativeEventWait};
use tcl_runtime_api::Completion;
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
use tcl_syntax::value::{ValueError, ValueOps};

pub(crate) type EventQueue = tcl_cmd_core::event::EventQueue<Value>;
pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("after", cmd_after);
    vm.register_stock_builtin("vwait", cmd_vwait);
    vm.register_stock_builtin("update", cmd_update);
}
fn selected(vm: &mut Vm) -> Result<NativeEventProtocol, Completion<Value>> {
    vm.actual_native_invocation_dialect()
        .native_event_protocol()
        .ok_or_else(|| {
            failure(
                vm,
                ValueError::CommandProtocolUnavailable("native event purpose"),
            )
        })
}
fn failure(vm: &mut Vm, error: ValueError) -> Completion<Value> {
    crate::command::completion_from_cmd_error(vm, error.into())
}
fn wrong(vm: &mut Vm, usage: &str) -> Completion<Value> {
    crate::command::native_wrong_arguments_message(vm, usage)
}
fn script(vm: &mut Vm, p: NativeEventProtocol, args: &[Value]) -> Result<Value, Completion<Value>> {
    if p.retains_single_script() && args.len() == 1 {
        return Ok(args[0].clone().into_native_reference());
    }
    tcl_cmd_core::list::concat_selected(vm, args)
        .map(Value::into_native_reference)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))
}
fn numeric(vm: &mut Vm, p: NativeEventProtocol, original: &Value) -> Result<Duration, ValueError> {
    if let Some(ms) = p.cached_number_ms(&original.native_object_snapshot()) {
        return Ok(Duration::from_millis(ms));
    }
    if p.number_kind() == tcl_syntax::scalar_getter::NativeScalarGetterKind::Double {
        let value = vm.as_double(original)?;
        return Duration::try_from_secs_f64(value.max(0.0) / 1000.0)
            .map_err(|_| ValueError::CommandProtocolUnavailable("host timer capacity"));
    }
    let value = original.native_scalar_getter_with_environment(
        vm.actual_native_invocation_dialect(),
        p.number_kind(),
        vm.host().numeric_environment(),
    )?;
    let tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(ms) = value else {
        return Err(ValueError::ScalarNumericInputUnavailable);
    };
    Ok(Duration::from_millis(u64::try_from(ms).unwrap_or(0)))
}
fn option(vm: &mut Vm, p: NativeEventProtocol, first: &Value) -> Result<Option<usize>, ValueError> {
    let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
        p.after_options(),
    );
    if p.after_options_exact() {
        vm.native_jim_enum_from_original(
            first,
            &table,
            tcl_registry::native_jim_enum::NativeJimEnumFlags::options(true),
            Some(b"argument"),
        )
        .map(Result::ok)
    } else {
        vm.native_index_from_original(first, &table, false, "argument")
            .map(Result::ok)
    }
}
fn after_miss(vm: &mut Vm, p: NativeEventProtocol, first: &Value) -> Completion<Value> {
    if p.after_options_exact() {
        return match vm.native_static_option_index(first, p.after_options(), true, "argument") {
            Err(e) => crate::command::completion_from_cmd_error(vm, e),
            Ok(_) => unreachable!("prior enum miss"),
        };
    }
    let bytes = match vm.native_string_bytes(first) {
        Ok(b) => b,
        Err(e) => return failure(vm, e),
    };
    let mut details =
        tcl_cmd_core::CmdError::new_bytes(p.after_miss_message(&bytes).expect("C after miss"))
            .into_byte_details();
    details.error_code =
        tcl_cmd_core::CmdErrorCodeUpdate::Set(p.after_miss_code(&bytes).expect("C after code"));
    crate::command::completion_from_cmd_error(
        vm,
        tcl_cmd_core::CmdError::from_byte_details(details),
    )
}
fn cmd_after(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(first) = args.first() else {
        return wrong(vm, "wrong # args: should be \"after option ?arg ...?\"");
    };
    let p = match selected(vm) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let object = first.native_object_snapshot();
    let first_byte = if p.number_kind() == tcl_syntax::scalar_getter::NativeScalarGetterKind::Int
        && !matches!(
            object.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::Numeric(_)
        ) {
        match vm.native_string_bytes(first) {
            Ok(b) => b.first().copied(),
            Err(e) => return failure(vm, e),
        }
    } else {
        None
    };
    let numeric_first = p.number_before_options(&object, first_byte);
    let mut delay = None;
    let member = if numeric_first {
        match numeric(vm, p, first) {
            Ok(ms) => {
                delay = Some(ms);
                None
            }
            Err(e) if e.native_access_refusal().is_some() || p.reports_number_error() => {
                return failure(vm, e);
            }
            Err(_) => match option(vm, p, first) {
                Ok(i) => i,
                Err(e) => return failure(vm, e),
            },
        }
    } else {
        match option(vm, p, first) {
            Ok(Some(i)) => Some(i),
            Ok(None) => match numeric(vm, p, first) {
                Ok(ms) => {
                    delay = Some(ms);
                    None
                }
                Err(e) if e.native_access_refusal().is_some() => return failure(vm, e),
                Err(_) => return after_miss(vm, p, first),
            },
            Err(e) => return failure(vm, e),
        }
    };
    if let Some(ms) = delay {
        if args.len() == 1 {
            std::thread::sleep(ms);
            return ok(Value::empty());
        }
        return schedule(vm, p, &args[1..], ms, EventKind::Timer);
    }
    let Some(member) = member else {
        return after_miss(vm, p, first);
    };
    match p.after_options()[member] {
        "idle" => {
            if args.len() < 2 {
                return wrong(
                    vm,
                    "wrong # args: should be \"after idle script ?script ...?\"",
                );
            }
            schedule(vm, p, &args[1..], Duration::ZERO, EventKind::Idle)
        }
        "cancel" => cancel(vm, p, &args[1..]),
        "info" => info(vm, p, &args[1..]),
        _ => unreachable!("selected after table"),
    }
}
fn schedule(
    vm: &mut Vm,
    p: NativeEventProtocol,
    args: &[Value],
    delay: Duration,
    kind: EventKind,
) -> Completion<Value> {
    if Instant::now().checked_add(delay).is_none() {
        return failure(
            vm,
            ValueError::CommandProtocolUnavailable("host timer deadline capacity"),
        );
    }
    let original = match script(vm, p, args) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let reported = if kind == EventKind::Idle || (delay.is_zero() && p.zero_timer_reports_idle()) {
        EventKind::Idle
    } else {
        EventKind::Timer
    };
    let actual = if kind == EventKind::Idle && p.idle_is_timer() {
        EventKind::Timer
    } else {
        kind
    };
    let id = vm
        .events
        .push(p.first_id(), delay, actual, reported, original);
    ok(Value::string(format!("after#{id}")))
}
fn after_id(p: NativeEventProtocol, bytes: &[u8]) -> Result<Option<u64>, ValueError> {
    p.after_id(
        bytes,
        tcl_runtime_api::native_hash_abi::supported_backend_array_search_abi(),
    )
}
fn cancel(vm: &mut Vm, p: NativeEventProtocol, args: &[Value]) -> Completion<Value> {
    let Some(first) = args.first() else {
        return wrong(vm, "wrong # args: should be \"after cancel id|command\"");
    };
    let mut id = None;
    if p.cancel_id_first() {
        let bytes = match vm.native_string_bytes(first) {
            Ok(b) => b,
            Err(e) => return failure(vm, e),
        };
        id = match after_id(p, &bytes) {
            Ok(id) => id.filter(|id| *id > 0),
            Err(error) => return failure(vm, error),
        };
    }
    if id.is_none() {
        let original = match script(vm, p, args) {
            Ok(s) => s,
            Err(c) => return c,
        };
        let bytes = match vm.native_string_bytes(&original) {
            Ok(b) => b,
            Err(e) => return failure(vm, e),
        };
        for candidate in vm.events.ids(p.deadline_order()) {
            let Some((s, _)) = vm.events.script(candidate) else {
                continue;
            };
            let s = s.clone();
            let other = match vm.native_string_bytes(&s) {
                Ok(b) => b,
                Err(e) => return failure(vm, e),
            };
            if bytes == other {
                id = Some(candidate);
                break;
            }
        }
        if id.is_none() && !p.cancel_id_first() {
            id = match after_id(p, &bytes) {
                Ok(id) => id,
                Err(error) => return failure(vm, error),
            };
        }
    }
    if let Some(id) = id {
        let remaining = vm.events.remaining(id);
        let cancelled = vm.events.cancel(id);
        if p.cancel_id_first()
            && cancelled.is_some()
            && let Some(remaining) = remaining
        {
            return ok(Value::int(
                i64::try_from(remaining.as_micros()).expect("selected Jim event deadline"),
            ));
        }
    }
    ok(Value::empty())
}
fn info(vm: &mut Vm, p: NativeEventProtocol, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        let values = vm
            .events
            .ids(p.deadline_order())
            .into_iter()
            .map(|i| Value::string(format!("after#{i}")))
            .collect();
        return ok(vm.new_list(values));
    }
    let [original] = args else {
        return wrong(vm, "wrong # args: should be \"after info ?id?\"");
    };
    let bytes = match vm.native_string_bytes(original) {
        Ok(b) => b,
        Err(e) => return failure(vm, e),
    };
    let id = match after_id(p, &bytes) {
        Ok(id) => id,
        Err(error) => return failure(vm, error),
    };
    if let Some(id) = id
        && let Some((script, kind)) = vm.events.script(id)
    {
        let script = script.clone();
        let kind = Value::string(if kind == EventKind::Idle {
            "idle"
        } else {
            "timer"
        });
        return ok(vm.new_list(vec![script, kind]));
    }
    let reported = tcl_core_types::c_string_extent(&bytes);
    let mut message = b"event \"".to_vec();
    message.extend_from_slice(reported);
    message.extend_from_slice(b"\" doesn't exist");
    let mut detail = tcl_cmd_core::CmdError::new_bytes(message).into_byte_details();
    if let Some(code) = p.missing_event_code(&bytes) {
        detail.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(code);
    }
    crate::command::completion_from_cmd_error(vm, tcl_cmd_core::CmdError::from_byte_details(detail))
}
struct WaitFlag(Rc<Cell<bool>>);
impl NativeVariableObserver<Vm> for WaitFlag {
    type Error = tcl_cmd_core::CmdError;
    fn observe(&self, _: &mut Vm, _: NativeVariableTraceAccess<'_>) -> Result<(), Self::Error> {
        self.0.set(true);
        Ok(())
    }
}
fn cmd_vwait(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Implementation contract: naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
    let p = match selected(vm) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let form = match p
        .vwait()
        .form(args.len(), |index| vm.native_string_bytes(&args[index]))
    {
        Ok(form) => form,
        Err(error) => return failure(vm, error),
    };
    match form {
        tcl_registry::native_vwait::NativeVwaitForm::Basic => {}
        tcl_registry::native_vwait::NativeVwaitForm::Extended => {
            return failure(
                vm,
                ValueError::CommandProtocolUnavailable("extended native vwait grammar"),
            );
        }
        tcl_registry::native_vwait::NativeVwaitForm::WrongArity => {
            return wrong(vm, p.vwait().wrong_arguments());
        }
    }
    let original = &args[0];
    if p.wait() == NativeEventWait::GlobalValueComparison {
        return jim_wait(vm, original);
    }
    let flag = Rc::new(Cell::new(false));
    let token = match vm.with_event_global_frame(|vm| {
        vm.add_native_variable_observer(
            original,
            &[
                NativeVariableTraceOperation::Write,
                NativeVariableTraceOperation::Unset,
            ],
            Rc::new(WaitFlag(flag.clone())),
        )
    }) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let mut completion = None;
    while !flag.get() {
        if vm.events.is_empty() {
            completion = Some(failure(
                vm,
                ValueError::CommandProtocolUnavailable("host event-source inventory"),
            ));
            break;
        }
        process_one(vm);
        if let Some(c) = vm.refused_completion() {
            completion = Some(c);
            break;
        }
    }
    vm.remove_native_variable_observer(&token);
    completion.unwrap_or_else(|| ok(Value::empty()))
}
fn jim_wait(vm: &mut Vm, original: &Value) -> Completion<Value> {
    let before = match vm.original_global_event_value(original) {
        Ok(value) => value.map(Value::into_native_reference),
        Err(error) => return error,
    };
    while !vm.events.is_empty() {
        process_one(vm);
        if let Some(c) = vm.refused_completion() {
            return c;
        }
        let current = match vm.original_global_event_value(original) {
            Ok(value) => value,
            Err(error) if vm.refused_completion().is_some() => return error,
            Err(_) => None,
        };
        match (&before, current) {
            (None, None) => {}
            (Some(old), Some(new)) => {
                let a = match vm.native_string_bytes(old) {
                    Ok(b) => b,
                    Err(e) => return failure(vm, e),
                };
                let b = match vm.native_string_bytes(&new) {
                    Ok(b) => b,
                    Err(e) => return failure(vm, e),
                };
                if a != b {
                    break;
                }
            }
            _ => break,
        }
    }
    ok(Value::empty())
}
fn cmd_update(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let p = match selected(vm) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let idle = match args {
        [] => false,
        [option] if p.after_options_exact() => {
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
                    "idletasks",
                ]);
            match vm.native_jim_enum_from_original(
                option,
                &table,
                tcl_registry::native_jim_enum::NativeJimEnumFlags(
                    tcl_registry::native_jim_enum::NativeJimEnumFlags::ABBREVIATE,
                ),
                None,
            ) {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => return wrong(vm, "wrong # args: should be \"update ?idletasks?\""),
                Err(error) => return failure(vm, error),
            }
        }
        [option] => match vm.native_static_option_index(option, &["idletasks"], false, "option") {
            Ok(_) => true,
            Err(e) => return crate::command::completion_from_cmd_error(vm, e),
        },
        _ => return wrong(vm, "wrong # args: should be \"update ?idletasks?\""),
    };
    while service_ready(vm, idle && !p.idle_update_runs_timers()) {
        if let Some(c) = vm.refused_completion() {
            return c;
        }
    }
    if p.clears_update_result() {
        ok(Value::empty())
    } else {
        match vm.with_native_interp_result(|value| value.clone().into_native_reference()) {
            Ok(value) => ok(value),
            Err(e) => failure(vm, e),
        }
    }
}
fn service_ready(vm: &mut Vm, idle_only: bool) -> bool {
    let Some(turn) = vm.events.begin_turn(Instant::now(), idle_only) else {
        return false;
    };
    while let Some(script) = vm.events.pop_turn(turn) {
        run_event(vm, &script);
        if vm.refused_completion().is_some() {
            break;
        }
    }
    true
}
fn process_one(vm: &mut Vm) {
    if service_ready(vm, false) {
        return;
    }
    if let Some(deadline) = vm.events.earliest_deadline() {
        std::thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
        service_ready(vm, false);
    }
}
fn run_event(vm: &mut Vm, script: &Value) {
    let completion = vm.eval_original_event(script);
    let Ok(p) = selected(vm) else {
        return;
    };
    if vm.refused_completion().is_none() && p.reports_callback_code(completion.code.as_int()) {
        let options = vm
            .completion_options_snapshot(&completion)
            .into_native_reference();
        report_bg_error(vm, completion.result.into_native_reference(), options, p);
    }
}
fn report_bg_error(vm: &mut Vm, message: Value, options: Value, p: NativeEventProtocol) {
    let prefix = if p.after_options_exact() {
        Value::empty()
    } else {
        match vm.bgerror_apply(&[]) {
            Ok(prefix) => prefix,
            Err(_) => return,
        }
    };
    let mut words = match vm.list_elements(&prefix) {
        Ok(words) => words,
        Err(e) => {
            let _ = failure(vm, e);
            return;
        }
    };
    if let Some(head) = words.first() {
        match vm.lookup_original_command_at(
            if p.after_options_exact() {
                vm.current_ns_id()
            } else {
                tcl_core_types::ROOT_NS
            },
            head,
        ) {
            Ok(Some(_)) => {
                let head = words.remove(0);
                words.push(message);
                if !p.after_options_exact() {
                    words.push(options);
                }
                vm.with_event_global_frame(|vm| {
                    let _ = vm.invoke_host_original_object_vector(&head, &words);
                });
                return;
            }
            Err(e) => {
                let _ = failure(vm, e);
                return;
            }
            Ok(None) => {}
        }
    }
    let head = Value::new_native_string_bytes(b"bgerror".as_slice());
    let context = if p.after_options_exact() {
        vm.current_ns_id()
    } else {
        tcl_core_types::ROOT_NS
    };
    match vm.lookup_original_command_at(context, &head) {
        Ok(Some(_)) => {
            if p.after_options_exact() {
                let _ = vm.invoke_host_original_object_vector(&head, &[message]);
            } else {
                vm.with_event_global_frame(|vm| {
                    let _ = vm.invoke_host_original_object_vector(&head, &[message]);
                });
            }
            return;
        }
        Err(error) => {
            let _ = failure(vm, error);
            return;
        }
        Ok(None) => {}
    }
    let bytes = match vm.native_string_bytes(&message) {
        Ok(b) => b,
        Err(e) => {
            let _ = failure(vm, e);
            return;
        }
    };
    let mut info = vm.take_error_info().unwrap_or_else(|| bytes.to_vec());
    info.extend_from_slice(b"\n    (\"after\" script)");
    match tcl_syntax::raw_string::RawString::from_bytes(info.as_slice()).unicode() {
        Ok(text) => vm.report_stderr_text(&text),
        Err(_) => vm.report_stderr_text(&format!("native Tcl error bytes: {info:?}")),
    }
}

#[cfg(test)]
mod native_original_tests;
