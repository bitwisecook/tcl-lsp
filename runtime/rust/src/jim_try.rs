// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected Jim catch/try ordering over original runtime objects.

use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use tcl_cmd_core::{
    return_options::{self, ReturnOptionsOps},
    CmdError,
};
use tcl_syntax::value::ValueError;

const USAGE: &[u8] = b"try ?options? script ?on|trap code varlist script ...? ?finally script?";
fn switches(
    interp: &mut Interp,
    argv: &[Owned],
) -> Result<tcl_registry::catch_invocation::JimCapturePrefix, CmdError> {
    let headers = argv.iter().map(Owned::as_ptr).collect::<Vec<_>>();
    let originals = crate::interp::native_arguments::OriginalArguments::enter(interp, &headers)?;
    let selected = tcl_registry::catch_invocation::select_native_jim_capture_prefix(
        argv.len(),
        interp.native_invocation_dialect(),
        |index| {
            originals.check_original_for(interp, index, argv[index].as_ptr())?;
            let reached = originals.bytes(index)?;
            originals.check_original_for(interp, index, argv[index].as_ptr())?;
            Ok::<_, CmdError>(reached.to_vec())
        },
    );
    originals.check_for(interp)?;
    match selected? {
        tcl_registry::catch_invocation::JimCapturePrefixSelection::Valid(prefix) => Ok(prefix),
        tcl_registry::catch_invocation::JimCapturePrefixSelection::Invalid => {
            Err(CmdError::wrong_args_bytes(USAGE))
        }
        tcl_registry::catch_invocation::JimCapturePrefixSelection::Unknown => {
            Err(ValueError::CommandProtocolUnavailable("original Jim try prefix").into())
        }
    }
}

fn trap_matches(
    interp: &Interp,
    ops: &mut crate::return_options::NativeReturnOps,
    pattern: &Owned,
    error_code: &crate::interp::OriginalJimErrorCodeCapture,
) -> Result<bool, CmdError> {
    let Some(original_code) = error_code.original(interp)? else {
        return Ok(false);
    };
    let pattern = ops.list(pattern)?;
    let source =
        crate::interp::native_arguments::OriginalArguments::enter(interp, &[original_code])?;
    let source_current = || {
        error_code.original(interp)?;
        source.check_original_for(interp, 0, original_code)
    };
    let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
    let code = crate::list::list_elements_native_using(
        original_code,
        protocol,
        &source_current,
        &|error| source.retain_refusal(error.into()),
    )?;
    let backing = crate::list::native_list_backing(original_code).ok_or(
        ValueError::CommandProtocolUnavailable("original Jim trap error-code List"),
    )?;
    let originals = crate::interp::native_arguments::OriginalArguments::enter(interp, &code)?;
    let current = || {
        source_current()?;
        originals.check_for(interp)?;
        backing
            .check_original_header(original_code)
            .map_err(|error| originals.retain_refusal(error.into()))
    };
    current()?;
    if pattern.len() > code.len() {
        return Ok(false);
    }
    let dialect = interp.native_invocation_dialect();
    let representation = dialect
        .string_length_representation()
        .ok_or(ValueError::CharacterModelUnavailable)?;
    for (index, left) in pattern.iter().take(code.len()).enumerate() {
        let left_bytes = ops.bytes(left)?;
        current()?;
        let right_bytes = originals.bytes_using(index, &current)?;
        let right = originals.original(index)?;
        let counts = (
            obj::native_character_count(left.as_ptr(), protocol, representation)?,
            obj::native_character_count(right, protocol, representation)?,
        );
        current()?;
        if tcl_syntax::raw_string::RawString::from_bytes(left_bytes)
            .jim084_compare(
                counts.0,
                &tcl_syntax::raw_string::RawString::from_bytes(right_bytes.to_vec()),
                counts.1,
                false,
            )
            .map_err(ValueError::from)?
            != std::cmp::Ordering::Equal
        {
            return Ok(false);
        }
    }
    Ok(true)
}

struct Handler {
    vars: Vec<Owned>,
    script: Owned,
}

fn clauses(
    interp: &mut Interp,
    ops: &mut crate::return_options::NativeReturnOps,
    rest: &[Owned],
    body_code: Code,
    error_code: &crate::interp::OriginalJimErrorCodeCapture,
) -> Result<(Option<Handler>, Option<Owned>), CmdError> {
    let mut selected = None;
    let mut finally = None;
    let mut at = 0;
    let (_, protocol) = crate::return_options::NativeReturnOps::selected(interp)?;
    while at < rest.len() {
        let dialect = interp.native_invocation_dialect();
        let table = dialect.native_jim_try_handler_table().ok_or(
            ValueError::CommandProtocolUnavailable("original Jim try handler table"),
        )?;
        let index = interp
            .native_jim_enum_from_original(
                rest[at].as_ptr(),
                &table,
                tcl_registry::native_jim_enum::NativeJimEnumFlags(1),
                Some(b"handler"),
            )?
            .map_err(|message| {
                let mut details = CmdError::new_bytes(message.expect("ERRMSG handler lookup"))
                    .into_byte_details();
                details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Unchanged;
                CmdError::from_byte_details(details)
            })?;
        let kind =
            dialect
                .native_jim_try_handler(index)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim try handler ordinal",
                ))?;
        match kind {
            tcl_registry::NativeTryClauseArgument::Finally => {
                if at + 2 != rest.len() {
                    return Err(CmdError::wrong_args_bytes(USAGE));
                }
                finally = Some(rest[at + 1].clone());
                at += 2;
            }
            tcl_registry::NativeTryClauseArgument::On
            | tcl_registry::NativeTryClauseArgument::Trap => {
                if at + 4 > rest.len() {
                    return Err(CmdError::wrong_args_bytes(USAGE));
                }
                let matches = if kind == tcl_registry::NativeTryClauseArgument::Trap {
                    trap_matches(interp, ops, &rest[at + 1], error_code)?
                } else {
                    let mut matched = false;
                    for original in ops.list(&rest[at + 1])? {
                        match return_options::parse_completion_code(ops, protocol, &original) {
                            Ok(code) if i64::from(code) == body_code.as_int() => {
                                matched = true;
                                break;
                            }
                            Ok(_) => {}
                            Err(error) if error.native_execution_refusal().is_some() => {
                                return Err(error);
                            }
                            Err(_) => return Err(CmdError::wrong_args_bytes(USAGE)),
                        }
                    }
                    matched
                };
                if matches && selected.is_none() {
                    selected = Some(Handler {
                        vars: ops.list(&rest[at + 2])?,
                        script: rest[at + 3].clone(),
                    });
                }
                at += 4;
            }
        }
    }
    Ok((selected, finally))
}

enum HandlerValue<'a> {
    Result,
    Options(&'a crate::interp::OriginalJimErrorCodeCapture, Code),
}

fn bind(interp: &mut Interp, name: &Owned, value: HandlerValue<'_>) -> bool {
    let original =
        match crate::interp::native_arguments::OriginalArguments::enter(interp, &[name.as_ptr()]) {
            Ok(original) => original,
            Err(error) => {
                interp.report_cmd_error(error);
                return false;
            }
        };
    let bytes = match original.bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => {
            interp.report_cmd_error(error);
            return false;
        }
    };
    if bytes.is_empty() {
        return true;
    }
    if let Err(error) = original.check_original_for(interp, 0, name.as_ptr()) {
        interp.report_cmd_error(error);
        return false;
    }
    let options = match value {
        HandlerValue::Result => None,
        HandlerValue::Options(captured, code) => {
            match crate::cmd_error::jim_options_object_with_captured_error_code(
                interp, captured, code,
            ) {
                Ok(options) => Some(options),
                Err(error) => {
                    interp.refuse_native_execution(error);
                    return false;
                }
            }
        }
    };
    if let Err(error) = original.check_original_for(interp, 0, name.as_ptr()) {
        interp.report_cmd_error(error);
        return false;
    }
    let value = options
        .as_ref()
        .map_or_else(|| interp.get_obj_result(), Owned::as_ptr);
    let assigned = interp.assign_original_named_variable(name.as_ptr(), value);
    if let Err(error) = original.check_original_for(interp, 0, name.as_ptr()) {
        interp.report_cmd_error(error);
        return false;
    }
    assigned.is_ok()
}

pub(crate) fn command(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let _ = interp.reset_original_jim_catch_error_code();
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    let original: Vec<_> = argv[1..]
        .iter()
        .map(|&value| Owned::retain(value))
        .collect();
    let (mut ops, _) = match crate::return_options::NativeReturnOps::selected(interp) {
        Ok(selected) => selected,
        Err(error) => return interp.report_cmd_error(error),
    };
    let prefix = match switches(interp, &original) {
        Ok(selected) => selected,
        Err(error) => return interp.report_cmd_error(error),
    };
    let Some(body) = original.get(prefix.script_at) else {
        return interp.wrong_args(USAGE);
    };
    let mut code = interp.eval_control_body(body.as_ptr());
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    interp.reset_jim_error_capture();
    let body_error_code = match interp.capture_original_jim_error_code() {
        Ok(captured) => captured,
        Err(error) => return interp.report_cmd_error(error),
    };
    let (handler, finally) = match clauses(
        interp,
        &mut ops,
        &original[prefix.script_at + 1..],
        code,
        &body_error_code,
    ) {
        Ok(clauses) => clauses,
        Err(error) => return interp.report_cmd_error(error),
    };
    if prefix.ignores(code.as_int()) {
        if let Some(finally) = finally {
            let _ = interp.eval_control_body(finally.as_ptr());
        }
        return if interp.host_refusal_pending() {
            Code::Error
        } else {
            code
        };
    }
    if let Some(handler) = handler {
        let result_bound = handler
            .vars
            .first()
            .is_none_or(|name| bind(interp, name, HandlerValue::Result));
        let options_bound = if result_bound {
            if let Some(name) = handler.vars.get(1) {
                bind(interp, name, HandlerValue::Options(&body_error_code, code))
            } else {
                true
            }
        } else {
            false
        };
        if interp.host_refusal_pending() {
            return Code::Error;
        }
        if options_bound {
            code = interp.eval_control_body(handler.script.as_ptr());
        }
    }
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    if let Some(finally) = finally {
        let previous = Owned::retain(interp.get_obj_result());
        let final_code = interp.eval_control_body(finally.as_ptr());
        if interp.host_refusal_pending() {
            return Code::Error;
        }
        if final_code == Code::Ok {
            interp.set_result(previous.as_ptr());
        } else {
            code = final_code;
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_finally_uses_live_private_state_and_original_result() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let cases: &[(&[u8], &[u8])] = &[
            (b"set code [catch {try -noerror {error BODY} finally {set done FINAL}} result options]; list $code $result [dict get $options -code] [dict get $options -level]", b"1 FINAL 1 0"),
            (b"set code [catch {try -noerror {error BODY} finally {return -code ok -level 3 FINAL}} result options]; list $code $result [dict get $options -code] [dict get $options -level]", b"1 FINAL 1 3"),
            (b"set code [catch {try {return -level 2 ORIGINAL} finally {set done FINAL}} result options]; list $code $result [dict get $options -code] [dict get $options -level]", b"2 ORIGINAL 0 2"),
            (b"set code [catch {try {error BODY} finally {error FINAL TRACE}} result options]; list $code $result [dict get $options -code] [dict get $options -level] [dict exists $options -during] [dict get $options -errorinfo]", b"1 FINAL 1 0 0 TRACE"),
            (b"set a SCALAR; set code [catch {try {error BODY} on error {a(k) options} {set reached YES} finally {set cleanup YES}} result options]; list $code $result [dict get $options -code] [dict get $options -level] $cleanup", b"1 {can't set \"a(k)\": variable isn't array} 1 0 YES"),
            (b"set code [catch {try {return -level 2 ORIGINAL} finally {return -level 0 FINAL}} result options]; list $code $result [dict get $options -code] [dict get $options -level]", b"2 ORIGINAL 0 0"),
        ];
        for (source, expected) in cases {
            crate::counters::reset();
            {
                let mut interp = Interp::new();
                interp.set_dialect_profile(profile);
                assert_eq!(interp.eval_str(source), Code::Ok, "{:?}", source);
                assert_eq!(interp.result_bytes(), *expected, "{:?}", source);
            }
            assert_eq!(crate::counters::finalize(), 0);
        }
    }

    #[test]
    fn explicit_stacktrace_and_error_code_keep_original_objects() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let trace = Owned::fresh(crate::list::new_list_obj(&[]));
        let code = Owned::fresh(crate::list::new_list_obj(&[]));
        interp.adopt_jim_stacktrace(trace.clone()).unwrap();
        interp.var_set(b"::errorCode", code.as_ptr()).unwrap();
        let receipt = interp.jim_return_receipt().unwrap();
        assert_eq!(receipt.stack_trace.as_ptr(), trace.as_ptr());
        assert_eq!(receipt.error_code.unwrap().as_ptr(), code.as_ptr());
    }
}
