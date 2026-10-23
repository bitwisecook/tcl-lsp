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

//! Jim's original existence query over current command and variable owners.
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{Code, Command, Interp};
use crate::obj::TclObj;
use tcl_registry::commands::tcl::NativeJimExistsKind;

pub(crate) fn install(interpreter: &mut Interp) {
    if interpreter
        .native_invocation_dialect()
        .native_jim_exists_protocol()
        .is_some()
    {
        interpreter.register_stock_builtin(b"exists", exists);
    }
}

fn exists(interpreter: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interpreter, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interpreter.report_cmd_error(error),
    };
    let code = exists_original(interpreter, argv, &arguments);
    arguments.finish(code)
}

fn exists_original(
    interpreter: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    let Some(protocol) = interpreter
        .native_invocation_dialect()
        .native_jim_exists_protocol()
    else {
        return interpreter.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim exists command",
            )
            .into(),
        );
    };
    let (kind, original) = match argv {
        [_, name] => (NativeJimExistsKind::Variable, *name),
        [_, option, name] => {
            let selected = interpreter.native_jim_enum_from_original(
                *option,
                &protocol.table(),
                tcl_registry::native_jim_enum::NativeJimEnumFlags::options(false),
                None,
            );
            if let Err(error) = arguments.check() {
                return interpreter.report_cmd_error(error);
            }
            let ordinal = match selected {
                Ok(Ok(ordinal)) => ordinal,
                Ok(Err(message)) => {
                    return interpreter.report_cmd_error(
                        tcl_cmd_core::CmdError::with_error_code_bytes(
                            message.unwrap_or_default(),
                            b"NONE",
                        ),
                    );
                }
                Err(error) => return interpreter.report_cmd_error(error.into()),
            };
            (
                protocol
                    .selected(ordinal)
                    .expect("original exists enum ordinal"),
                *name,
            )
        }
        _ => return interpreter.wrong_args_for_invocation(argv, protocol.usage()),
    };
    let reached = if kind == NativeJimExistsKind::Variable {
        interpreter.exists_original_jim_variable(original)
    } else {
        let current = interpreter.current_ns();
        match interpreter.resolve_original_command_at(current, original) {
            Ok(None) => Ok(false),
            Ok(Some((command, _))) => match kind {
                NativeJimExistsKind::Command => Ok(true),
                NativeJimExistsKind::Procedure => Ok(matches!(command, Command::Proc(_))),
                NativeJimExistsKind::Alias => Ok(matches!(
                    command,
                    Command::Alias { .. } | Command::ParentAlias { .. }
                )),
                NativeJimExistsKind::Channel
                    if matches!(
                        command,
                        Command::Proc(_) | Command::Alias { .. } | Command::ParentAlias { .. }
                    ) =>
                {
                    Ok(false)
                }
                NativeJimExistsKind::Channel => Err(interpreter.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim channel command flag",
                    )
                    .into(),
                )),
                NativeJimExistsKind::Variable => unreachable!("variable read selected separately"),
            },
            Err(error) => Err(interpreter.report_cmd_error(error.into())),
        }
    };
    if let Err(error) = arguments.check() {
        return interpreter.report_cmd_error(error);
    }
    match reached {
        Ok(exists) => {
            if kind == NativeJimExistsKind::Alias {
                if let Err(error) = interpreter.original_import_alias_exists(original, exists) {
                    return interpreter.report_cmd_error(error.into());
                }
            }
            interpreter.set_result(crate::obj::new_boolean_obj(i32::from(exists)));
            Code::Ok
        }
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jim() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn original_jim_exists_matches_command_variable_and_enum_observations() {
        let mut interpreter = jim();
        assert_eq!(
            interpreter.eval_str(include_bytes!(
                "../../../rust/tcl-registry/tests/data/native_jim_exists/exists.tcl"
            )),
            Code::Ok,
            "result={:?}, refusal={:?}",
            interpreter.result_bytes(),
            interpreter.native_execution_refusal(),
        );
        let expected =
            include_bytes!("../../../rust/tcl-registry/tests/data/native_jim_exists/stdout");
        assert_eq!(
            interpreter.result_bytes(),
            expected.strip_suffix(b"\n").unwrap()
        );
        assert!(interpreter.native_execution_refusal().is_none());
    }

    #[test]
    fn original_jim_exists_retains_hot_command_and_variable_objects_and_refuses_unknown_channel_flags(
    ) {
        let mut interpreter = jim();
        assert_eq!(
            interpreter.eval_str(b"set v 1; set name v; exists $name"),
            Code::Ok
        );
        assert_eq!(interpreter.result_bytes(), b"1");
        assert_eq!(interpreter.eval_str(b"unset v; exists $name"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"0");
        assert_eq!(interpreter.eval_str(b"proc q {} {}; set command q; exists -command $command; rename q r; exists -command $command"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"0");
        assert_ne!(interpreter.eval_str(b"exists -channel set"), Code::Ok);
        assert!(interpreter.native_execution_refusal().is_some());
    }
}
