// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected Jim catch/try ordering over original runtime objects.

use crate::{
    interp::{new_string, obj_bytes, Code, Interp},
    obj::{self, Owned, TclObj},
};
use tcl_cmd_core::{
    return_options::{self, ReturnOptionsOps},
    CmdError,
};
use tcl_syntax::value::ValueError;

const USAGE: &[u8] = b"try ?options? script ?on|trap code varlist script ...? ?finally script?";
const CODES: &[&[u8]] = &[
    b"ok",
    b"error",
    b"return",
    b"break",
    b"continue",
    b"signal",
    b"exit",
    b"eval",
];

fn switches(
    interp: &mut Interp,
    ops: &mut crate::return_options::NativeReturnOps,
    argv: &[Owned],
) -> Result<(usize, u64), CmdError> {
    let protocol = interp
        .native_invocation_dialect()
        .native_scalar_getter_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable("Jim try switches"))?;
    let mut at = 0;
    let mut ignored = (1_u64 << 5) | (1_u64 << 6) | (1_u64 << 7);
    while at + 1 < argv.len() {
        let bytes = ops.bytes(&argv[at])?;
        let word = tcl_core_types::c_string_extent(&bytes);
        if word == b"--" {
            at += 1;
            break;
        }
        if !word.starts_with(b"-") {
            break;
        }
        let (ignore, name) = word
            .strip_prefix(b"-no")
            .map_or((false, &word[1..]), |name| (true, name));
        let decimal = protocol
            .jim_decimal_wide_probe(name)
            .ok_or(ValueError::CommandProtocolUnavailable("Jim try switches"))?;
        let code = match decimal {
            Ok(code) if code >= 0 => code,
            _ => CODES
                .iter()
                .position(|code| *code == name)
                .map_or(-1, |code| code as i64),
        };
        if code < 0 {
            return Err(CmdError::wrong_args_bytes(USAGE));
        }
        let code = u32::try_from(code).ok().filter(|code| *code < 64).ok_or(
            ValueError::CommandProtocolUnavailable("Jim try ignore-mask width"),
        )?;
        if ignore {
            ignored |= 1_u64 << code;
        } else {
            ignored &= !(1_u64 << code);
        }
        at += 1;
    }
    Ok((at, ignored))
}

fn trap_matches(
    interp: &Interp,
    ops: &mut crate::return_options::NativeReturnOps,
    pattern: &Owned,
    error_code: Option<&Owned>,
) -> Result<bool, CmdError> {
    let Some(error_code) = error_code else {
        return Ok(false);
    };
    let pattern = ops.list(pattern)?;
    let code = ops.list(error_code)?;
    if pattern.len() > code.len() {
        return Ok(false);
    }
    let dialect = interp.native_invocation_dialect();
    let representation = dialect
        .string_length_representation()
        .ok_or(ValueError::CharacterModelUnavailable)?;
    let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
    for (left, right) in pattern.iter().zip(&code) {
        let left_bytes = ops.bytes(left)?;
        let right_bytes = ops.bytes(right)?;
        let counts = (
            obj::native_character_count(left.as_ptr(), protocol, representation)?,
            obj::native_character_count(right.as_ptr(), protocol, representation)?,
        );
        if tcl_syntax::raw_string::RawString::from_bytes(left_bytes)
            .jim084_compare(
                counts.0,
                &tcl_syntax::raw_string::RawString::from_bytes(right_bytes),
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
    error_code: Option<&Owned>,
) -> Result<(Option<Handler>, Option<Owned>), CmdError> {
    let mut selected = None;
    let mut finally = None;
    let mut at = 0;
    let (_, protocol) = crate::return_options::NativeReturnOps::selected(interp)?;
    while at < rest.len() {
        const HANDLERS: &[&str] = &["on", "trap", "finally"];
        let table =
            tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(HANDLERS);
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
        let name = HANDLERS[index].as_bytes();
        match name {
            b"finally" => {
                if at + 2 != rest.len() {
                    return Err(CmdError::wrong_args_bytes(USAGE));
                }
                finally = Some(rest[at + 1].clone());
                at += 2;
            }
            b"on" | b"trap" => {
                if at + 4 > rest.len() {
                    return Err(CmdError::wrong_args_bytes(USAGE));
                }
                let matches = if name == b"trap" {
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
            _ => {
                let mut message = b"bad handler \"".to_vec();
                message.extend_from_slice(name);
                message.extend_from_slice(b"\": must be on, trap or finally");
                let mut details = CmdError::new_bytes(message).into_byte_details();
                details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Unchanged;
                return Err(CmdError::from_byte_details(details));
            }
        }
    }
    Ok((selected, finally))
}

fn bind(interp: &mut Interp, name: &Owned, value: &Owned) -> bool {
    let name = obj_bytes(name.as_ptr());
    if name.is_empty() {
        return true;
    }
    match interp.var_set_named(&name, value.as_ptr()) {
        Ok(()) => true,
        Err(error) => {
            crate::builtins::var_error(interp, &name, error);
            false
        }
    }
}

pub(crate) fn command(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let none = Owned::fresh(new_string(b"NONE"));
    let _ = interp.var_set(b"::errorCode", none.as_ptr());
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
    let (body_at, ignored) = match switches(interp, &mut ops, &original) {
        Ok(selected) => selected,
        Err(error) => return interp.report_cmd_error(error),
    };
    let Some(body) = original.get(body_at) else {
        return interp.wrong_args(USAGE);
    };
    let mut code = interp.eval_control_body(body.as_ptr());
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    interp.reset_jim_error_capture();
    let body_error_code = interp.jim_return_receipt().error_code;
    let (handler, finally) = match clauses(
        interp,
        &mut ops,
        &original[body_at + 1..],
        code,
        body_error_code.as_ref(),
    ) {
        Ok(clauses) => clauses,
        Err(error) => return interp.report_cmd_error(error),
    };
    let ignored = u32::try_from(code.as_int())
        .ok()
        .is_some_and(|code| code < 64 && ignored & (1_u64 << code) != 0);
    if ignored {
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
        let result = Owned::retain(interp.get_obj_result());
        let result_bound = handler
            .vars
            .first()
            .is_none_or(|name| bind(interp, name, &result));
        let options_bound = if result_bound {
            if let Some(name) = handler.vars.get(1) {
                let mut receipt = interp.jim_return_receipt();
                receipt.error_code = body_error_code;
                let options = match crate::cmd_error::jim_options_object(interp, receipt, code) {
                    Ok(options) => options,
                    Err(error) => return interp.refuse_native_execution(error),
                };
                bind(interp, name, &options)
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
        interp.adopt_jim_stacktrace(trace.clone());
        interp.var_set(b"::errorCode", code.as_ptr()).unwrap();
        let receipt = interp.jim_return_receipt();
        assert_eq!(receipt.stack_trace.as_ptr(), trace.as_ptr());
        assert_eq!(receipt.error_code.unwrap().as_ptr(), code.as_ptr());
    }
}
