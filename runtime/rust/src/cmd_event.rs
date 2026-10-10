// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object event callbacks and independently selected global wait subjects.

use crate::interp::{new_string, Code, Interp};
use crate::obj::{self, TclObj};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tcl_cmd_core::event::EventKind;
use tcl_registry::native_event::{NativeEventProtocol, NativeEventWait};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
use tcl_syntax::value::{ValueError, ValueOps};

/// Pending events retain actual owning references to their script objects.
pub(crate) type EventQueue = tcl_cmd_core::event::EventQueue<obj::Owned>;
/// Register the selected after, vwait and update handlers.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"after", after_cmd);
    interp.register_builtin(b"vwait", vwait_cmd);
    interp.register_builtin(b"update", update_cmd);
}
fn selected(interp: &mut Interp) -> Result<NativeEventProtocol, Code> {
    interp
        .native_invocation_dialect()
        .native_event_protocol()
        .ok_or_else(|| {
            failure(
                interp,
                ValueError::CommandProtocolUnavailable("native event purpose"),
            )
        })
}
fn failure(interp: &mut Interp, error: ValueError) -> Code {
    interp.report_cmd_error(error.into())
}
fn empty() -> *mut TclObj {
    new_string(b"")
}
fn finish(interp: &mut Interp, value: Result<*mut TclObj, Code>) -> Code {
    match value {
        Ok(value) => {
            unsafe { interp.set_obj_result(value) };
            Code::Ok
        }
        Err(code) => code,
    }
}
fn script(
    interp: &mut Interp,
    p: NativeEventProtocol,
    args: &[*mut TclObj],
) -> Result<obj::Owned, Code> {
    if p.retains_single_script() && args.len() == 1 {
        return Ok(obj::Owned::retain(args[0]));
    }
    tcl_cmd_core::list::concat_selected(interp, args)
        .map(obj::Owned::fresh)
        .map_err(|e| interp.report_cmd_error(e))
}
fn numeric(
    interp: &mut Interp,
    p: NativeEventProtocol,
    original: *mut TclObj,
) -> Result<Duration, ValueError> {
    let snapshot = obj::native_object_snapshot(original)?;
    if let Some(ms) = p.cached_number_ms(&snapshot) {
        return Ok(Duration::from_millis(ms));
    }
    if p.number_kind() == tcl_syntax::scalar_getter::NativeScalarGetterKind::Double {
        let value = interp.as_double(&original)?;
        return Duration::try_from_secs_f64(value.max(0.0) / 1000.0)
            .map_err(|_| ValueError::CommandProtocolUnavailable("host timer capacity"));
    }
    let value = crate::typed_value::native_scalar_getter_with_environment(
        original,
        interp.native_invocation_dialect(),
        p.number_kind(),
        interp.host().numeric_environment(),
    )?;
    let tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(ms) = value else {
        return Err(ValueError::ScalarNumericInputUnavailable);
    };
    Ok(Duration::from_millis(u64::try_from(ms).unwrap_or(0)))
}
fn option(
    interp: &mut Interp,
    p: NativeEventProtocol,
    first: *mut TclObj,
) -> Result<Option<usize>, ValueError> {
    let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
        p.after_options(),
    );
    if p.after_options_exact() {
        interp
            .native_jim_enum_from_original(
                first,
                &table,
                tcl_registry::native_jim_enum::NativeJimEnumFlags::options(true),
                Some(b"argument"),
            )
            .map(|r| r.ok())
    } else {
        interp
            .native_index_from_original(first, &table, false, "argument")
            .map(|r| r.ok())
    }
}
fn after_miss(interp: &mut Interp, p: NativeEventProtocol, first: *mut TclObj) -> Code {
    if p.after_options_exact() {
        return match interp.native_static_string_option_index(
            first,
            p.after_options(),
            true,
            "argument",
        ) {
            Err(e) => interp.report_cmd_error(e),
            Ok(_) => unreachable!("prior enum miss"),
        };
    }
    let bytes = match interp.native_string_bytes(&first) {
        Ok(b) => b,
        Err(e) => return failure(interp, e),
    };
    interp.error_with_code(
        &p.after_miss_message(&bytes).expect("C after miss"),
        &p.after_miss_code(&bytes).expect("C after code"),
    )
}
fn after_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let result = after(interp, &argv[1..]);
    finish(interp, result)
}
fn after(interp: &mut Interp, args: &[*mut TclObj]) -> Result<*mut TclObj, Code> {
    let Some(&first) = args.first() else {
        return Err(
            interp.wrong_arguments_message(b"wrong # args: should be \"after option ?arg ...?\"")
        );
    };
    let p = selected(interp)?;
    let object = obj::native_object_snapshot(first).map_err(|e| failure(interp, e))?;
    let first_byte = if p.number_kind() == tcl_syntax::scalar_getter::NativeScalarGetterKind::Int
        && !matches!(
            object.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::Numeric(_)
        ) {
        interp
            .native_string_bytes(&first)
            .map_err(|e| failure(interp, e))?
            .first()
            .copied()
    } else {
        None
    };
    let numeric_first = p.number_before_options(&object, first_byte);
    let mut delay = None;
    let member = if numeric_first {
        match numeric(interp, p, first) {
            Ok(ms) => {
                delay = Some(ms);
                None
            }
            Err(e) if e.native_access_refusal().is_some() || p.reports_number_error() => {
                return Err(failure(interp, e));
            }
            Err(_) => option(interp, p, first).map_err(|e| failure(interp, e))?,
        }
    } else {
        match option(interp, p, first).map_err(|e| failure(interp, e))? {
            Some(i) => Some(i),
            None => match numeric(interp, p, first) {
                Ok(ms) => {
                    delay = Some(ms);
                    None
                }
                Err(e) if e.native_access_refusal().is_some() => return Err(failure(interp, e)),
                Err(_) => return Err(after_miss(interp, p, first)),
            },
        }
    };
    if let Some(ms) = delay {
        if args.len() == 1 {
            std::thread::sleep(ms);
            return Ok(empty());
        }
        return schedule(interp, p, &args[1..], ms, EventKind::Timer);
    }
    let Some(member) = member else {
        return Err(after_miss(interp, p, first));
    };
    match p.after_options()[member] {
        "idle" => {
            if args.len() < 2 {
                return Err(interp.wrong_arguments_message(
                    b"wrong # args: should be \"after idle script ?script ...?\"",
                ));
            }
            schedule(interp, p, &args[1..], Duration::ZERO, EventKind::Idle)
        }
        "cancel" => cancel(interp, p, &args[1..]),
        "info" => info(interp, p, &args[1..]),
        _ => unreachable!("selected after table"),
    }
}
fn schedule(
    interp: &mut Interp,
    p: NativeEventProtocol,
    args: &[*mut TclObj],
    delay: Duration,
    kind: EventKind,
) -> Result<*mut TclObj, Code> {
    if Instant::now().checked_add(delay).is_none() {
        return Err(failure(
            interp,
            ValueError::CommandProtocolUnavailable("host timer deadline capacity"),
        ));
    }
    let original = script(interp, p, args)?;
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
    let id = interp
        .events_mut()
        .push(p.first_id(), delay, actual, reported, original);
    Ok(new_string(format!("after#{id}").as_bytes()))
}
fn after_id(p: NativeEventProtocol, bytes: &[u8]) -> Result<Option<u64>, ValueError> {
    p.after_id(
        bytes,
        tcl_runtime_api::native_hash_abi::supported_backend_array_search_abi(),
    )
}
fn cancel(
    interp: &mut Interp,
    p: NativeEventProtocol,
    args: &[*mut TclObj],
) -> Result<*mut TclObj, Code> {
    let Some(&first) = args.first() else {
        return Err(
            interp.wrong_arguments_message(b"wrong # args: should be \"after cancel id|command\"")
        );
    };
    let mut id = None;
    if p.cancel_id_first() {
        let bytes = interp
            .native_string_bytes(&first)
            .map_err(|e| failure(interp, e))?;
        id = after_id(p, &bytes)
            .map_err(|error| failure(interp, error))?
            .filter(|id| *id > 0);
    }
    if id.is_none() {
        let original = script(interp, p, args)?;
        let bytes = interp
            .native_string_bytes(&original.as_ptr())
            .map_err(|e| failure(interp, e))?;
        let ids = interp.events_mut().ids(p.deadline_order());
        for candidate in ids {
            let owned = interp
                .events_mut()
                .script(candidate)
                .map(|(s, _)| s.clone());
            let Some(owned) = owned else { continue };
            let other = interp
                .native_string_bytes(&owned.as_ptr())
                .map_err(|e| failure(interp, e))?;
            if bytes == other {
                id = Some(candidate);
                break;
            }
        }
        if id.is_none() && !p.cancel_id_first() {
            id = after_id(p, &bytes).map_err(|error| failure(interp, error))?;
        }
    }
    if let Some(id) = id {
        let remaining = interp.events_mut().remaining(id);
        let cancelled = interp.events_mut().cancel(id);
        if p.cancel_id_first() && cancelled.is_some() {
            if let Some(remaining) = remaining {
                return Ok(interp.new_int(
                    i64::try_from(remaining.as_micros()).expect("selected Jim event deadline"),
                ));
            }
        }
    }
    Ok(empty())
}
fn info(
    interp: &mut Interp,
    p: NativeEventProtocol,
    args: &[*mut TclObj],
) -> Result<*mut TclObj, Code> {
    if args.is_empty() {
        let values = interp
            .events_mut()
            .ids(p.deadline_order())
            .into_iter()
            .map(|i| obj::Owned::fresh(new_string(format!("after#{i}").as_bytes())))
            .collect::<Vec<_>>();
        return Ok(interp.new_list(values.iter().map(obj::Owned::as_ptr).collect()));
    }
    let [original] = args else {
        return Err(interp.wrong_arguments_message(b"wrong # args: should be \"after info ?id?\""));
    };
    let bytes = interp
        .native_string_bytes(original)
        .map_err(|e| failure(interp, e))?;
    if let Some(id) = after_id(p, &bytes).map_err(|error| failure(interp, error))? {
        let script = interp.events_mut().script(id).map(|(s, k)| (s.clone(), k));
        if let Some((script, kind)) = script {
            let kind = obj::Owned::fresh(new_string(if kind == EventKind::Idle {
                b"idle"
            } else {
                b"timer"
            }));
            return Ok(interp.new_list(vec![script.as_ptr(), kind.as_ptr()]));
        }
    }
    let reported = tcl_core_types::c_string_extent(&bytes);
    let mut message = b"event \"".to_vec();
    message.extend_from_slice(reported);
    message.extend_from_slice(b"\" doesn't exist");
    let mut detail = tcl_cmd_core::CmdError::new_bytes(message).into_byte_details();
    if let Some(code) = p.missing_event_code(&bytes) {
        detail.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(code);
    }
    Err(interp.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(detail)))
}
struct WaitFlag(Rc<Cell<bool>>);
impl NativeVariableObserver<Interp> for WaitFlag {
    type Error = tcl_cmd_core::CmdError;
    fn observe(&self, _: &mut Interp, _: NativeVariableTraceAccess<'_>) -> Result<(), Self::Error> {
        self.0.set(true);
        Ok(())
    }
}
fn vwait_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Implementation contract: naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
    let p = match selected(interp) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let form = match p.vwait().form(argv.len() - 1, |index| {
        interp.native_string_bytes(&argv[index + 1])
    }) {
        Ok(form) => form,
        Err(error) => return failure(interp, error),
    };
    match form {
        tcl_registry::native_vwait::NativeVwaitForm::Basic => {}
        tcl_registry::native_vwait::NativeVwaitForm::Extended => {
            return failure(
                interp,
                ValueError::CommandProtocolUnavailable("extended native vwait grammar"),
            );
        }
        tcl_registry::native_vwait::NativeVwaitForm::WrongArity => {
            return interp.wrong_arguments_message(p.vwait().wrong_arguments().as_bytes());
        }
    }
    let original = &argv[1];
    if p.wait() == NativeEventWait::GlobalValueComparison {
        return jim_wait(interp, *original);
    }
    let flag = Rc::new(Cell::new(false));
    let token = match interp.with_event_global_frame(|i| {
        i.add_native_variable_observer(
            *original,
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
    let mut completion = Code::Ok;
    while !flag.get() {
        if interp.events_mut().is_empty() {
            completion = failure(
                interp,
                ValueError::CommandProtocolUnavailable("host event-source inventory"),
            );
            break;
        }
        process_one(interp);
        if interp.native_access_refusal().is_some() {
            completion = Code::Error;
            break;
        }
    }
    interp.remove_native_variable_observer(&token);
    if completion == Code::Ok {
        interp.set_result_bytes(b"");
    }
    completion
}
fn jim_wait(interp: &mut Interp, original: *mut TclObj) -> Code {
    let before = match interp.original_global_event_value(original) {
        Ok(value) => value.map(obj::Owned::retain),
        Err(code) => return code,
    };
    while !interp.events_mut().is_empty() {
        process_one(interp);
        if interp.native_access_refusal().is_some() {
            return Code::Error;
        }
        let current = match interp.original_global_event_value(original) {
            Ok(value) => value.map(obj::Owned::retain),
            Err(code) if interp.native_access_refusal().is_some() => return code,
            Err(_) => None,
        };
        match (&before, current) {
            (None, None) => {}
            (Some(old), Some(new)) => {
                let a = match interp.native_string_bytes(&old.as_ptr()) {
                    Ok(b) => b,
                    Err(e) => return failure(interp, e),
                };
                let b = match interp.native_string_bytes(&new.as_ptr()) {
                    Ok(b) => b,
                    Err(e) => return failure(interp, e),
                };
                if a != b {
                    break;
                }
            }
            _ => break,
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}
fn update_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let p = match selected(interp) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let idle = match &argv[1..] {
        [] => false,
        [option] if p.after_options_exact() => {
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
                    "idletasks",
                ]);
            match interp.native_jim_enum_from_original(
                *option,
                &table,
                tcl_registry::native_jim_enum::NativeJimEnumFlags(
                    tcl_registry::native_jim_enum::NativeJimEnumFlags::ABBREVIATE,
                ),
                None,
            ) {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => {
                    return interp.wrong_arguments_message(
                        b"wrong # args: should be \"update ?idletasks?\"",
                    );
                }
                Err(error) => return failure(interp, error),
            }
        }
        [option] => {
            match interp.native_static_option_index(*option, &[b"idletasks"], false, "option") {
                Ok(_) => true,
                Err(e) => return interp.report_cmd_error(e),
            }
        }
        _ => {
            return interp
                .wrong_arguments_message(b"wrong # args: should be \"update ?idletasks?\"");
        }
    };
    interp.process_bg_errors();
    while service_ready(interp, idle && !p.idle_update_runs_timers()) {
        if interp.native_access_refusal().is_some() {
            return Code::Error;
        }
    }
    interp.process_bg_errors();
    if p.clears_update_result() {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}
fn service_ready(interp: &mut Interp, idle_only: bool) -> bool {
    let turn = interp.events_mut().begin_turn(Instant::now(), idle_only);
    let Some(turn) = turn else { return false };
    loop {
        let script = interp.events_mut().pop_turn(turn);
        let Some(script) = script else { break };
        run_event(interp, script.as_ptr());
        if interp.native_access_refusal().is_some() {
            break;
        }
    }
    true
}
fn process_one(interp: &mut Interp) {
    if service_ready(interp, false) {
        return;
    }
    let deadline = interp.events_mut().earliest_deadline();
    if let Some(deadline) = deadline {
        std::thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
        service_ready(interp, false);
    }
}
fn run_event(interp: &mut Interp, script: *mut TclObj) {
    let code = interp.eval_original_event(script);
    let p = match selected(interp) {
        Ok(p) => p,
        Err(_) => return,
    };
    if interp.native_access_refusal().is_none() && p.reports_callback_code(code.as_int()) {
        let message = obj::Owned::retain(interp.get_obj_result());
        let options = obj::Owned::fresh(crate::cmd_error::completion_options(interp, code));
        interp.report_bg_error_original(message, options);
        interp.process_bg_errors();
    }
}

#[cfg(test)]
mod native_original_tests;

#[cfg(test)]
mod tests {
    use crate::interp::{Code, Interp};

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    /// `update`'s option and `after`'s subcommand word both resolve through
    /// the one `Tcl_GetIndexFromObj` matcher. `update`'s is an ordinary
    /// one-entry table, so a miss is always `bad`. `after`'s scan is
    /// *silent* in C (`Tcl_GetIndexFromObj` with a NULL interp), so a miss
    /// falls through to the integer parse and `after` composes its own
    /// sentence — including for the ambiguous `i`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   update {}  -> bad option "": must be idletasks
    ///   update x   -> bad option "x": must be idletasks
    ///   update i   -> {}     (abbreviation accepted)
    ///   after in   -> {}     (info)      ;  after ca 1 -> {}   (cancel)
    ///   after i    -> bad argument "i": must be cancel, idle, info, or an integer
    ///   after {}   -> bad argument "": must be cancel, idle, info, or an integer
    #[test]
    fn update_and_after_words_resolve_like_tcl_get_index_from_obj() {
        let mut i = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        assert_eq!(i.eval_str(b"update {}"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad option \"\": must be idletasks");
        assert_eq!(i.eval_str(b"update x"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad option \"x\": must be idletasks");
        assert_eq!(i.eval_str(b"update a b"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"update ?idletasks?\""
        );
        // A unique prefix resolves.
        ok(&mut i, b"update i");
        // `after`'s silent scan: `in` and `ca` resolve; `i` is ambiguous and so
        // reaches the integer parse, where `after` words its own miss.
        assert_eq!(ok(&mut i, b"after in"), b"");
        assert_eq!(ok(&mut i, b"after ca 1"), b"");
        assert_eq!(i.eval_str(b"after i"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"bad argument \"i\": must be cancel, idle, info, or an integer"
        );
        assert_eq!(i.eval_str(b"after {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"bad argument \"\": must be cancel, idle, info, or an integer"
        );
    }
}
