// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native Switch primitives and paired entered-procedure controls.
use crate::{Value, Vm};
use tcl_cmd_core::{CmdError, switch::NativeCompiledSwitchObjects};
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueError;
impl NativeCompiledSwitchObjects for Vm {
    fn compiled_binary_bytes(
        &mut self,
        value: &Value,
        version: TclVersion,
    ) -> Result<std::rc::Rc<[u8]>, CmdError> {
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| {
                protocol.tcl_version() == Some(version) && version < TclVersion::V9_0
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiled C8 binary conversion",
            ))?;
        value.check_native_header()?;
        value
            .as_native_byte_array(
                tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                    version.string_character_model(),
                ),
                protocol,
            )
            .map_err(Into::into)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_core_types::Code;
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn registered_switch_instructions_match_56_native_controls() {
        let mut compared = 0;
        for row in include_str!("../../tcl-registry/tests/data/registered-switch56.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let profile = tcl_dialect::DialectProfile::find(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("native version"),
            })
            .unwrap();
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let body = Value::new_native_string_bytes(bytes(fields[2]));
            assert_eq!(
                vm.invoke_command(
                    "proc",
                    &[
                        Value::new_native_string_bytes(b"p".as_slice()),
                        Value::new_native_string_bytes(b"x".as_slice()),
                        body
                    ]
                )
                .code,
                Code::Ok
            );
            let result = vm.invoke_command("p", &[Value::new_native_string_bytes(b"A".as_slice())]);
            assert_eq!(
                result.code,
                Code::from_int(fields[3].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                result
            );
            assert_eq!(
                result
                    .result
                    .native_string_bytes(
                        vm.native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap()
                    )
                    .unwrap()
                    .as_ref(),
                bytes(fields[4]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
    #[test]
    fn registered_switch_instructions_match_56_native_branch_controls() {
        let mut compared = 0;
        for row in
            include_str!("../../tcl-registry/tests/data/registered-switch-branches56.tsv").lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let profile = tcl_dialect::DialectProfile::find(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("native version"),
            })
            .unwrap();
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            assert_eq!(
                vm.invoke_command(
                    "proc",
                    &[
                        Value::new_native_string_bytes(b"p".as_slice()),
                        Value::new_native_string_bytes(b"x".as_slice()),
                        Value::new_native_string_bytes(bytes(fields[2]))
                    ]
                )
                .code,
                Code::Ok
            );
            let argument = Value::from_native_byte_array(
                Rc::from(bytes(fields[3])),
                vm.native_invocation_dialect(),
            )
            .unwrap();
            let result = vm.invoke_command("p", &[argument]);
            assert_eq!(
                result.code,
                Code::from_int(fields[4].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                result
            );
            assert_eq!(
                result
                    .result
                    .native_string_bytes(
                        vm.native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap()
                    )
                    .unwrap()
                    .as_ref(),
                bytes(fields[5]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
}
