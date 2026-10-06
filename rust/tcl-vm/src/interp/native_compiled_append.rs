// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected append instructions retain their actual headers and receiver cells.

use super::{CapturedVariableUpdate, Local, Value, Vm};
use tcl_runtime_api::{Completion, VariableUpdateResult};
use tcl_syntax::native_string::NativeStringProtocol;

impl Vm {
    pub(crate) fn lappend_instruction_single_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        addition: &Value,
    ) -> Result<Value, Completion<Value>> {
        let captured = self.capture_update_cell(name, key)?;
        self.lappend_instruction_single_captured(name, key, &captured, addition)
    }

    pub(super) fn lappend_instruction_single_captured(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        addition: &Value,
    ) -> Result<Value, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        self.with_variable_operation(&captured.cell, |vm| {
            vm.check_captured_update_with_errors(name, key, captured, true)?;
            let id = captured
                .cell
                .id
                .expect("selected single-element append cell");
            let selected = match vm.initial_value_at_cell(id, captured.explicit_array) {
                Ok(selected) => selected,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(vm, error.into()));
                }
            };
            let value = selected.map_or_else(
                || Value::new_native_string_bytes(&b""[..]),
                |value| {
                    if value.native_object_is_shared() {
                        value.duplicate_native_object_in(protocol)
                    } else {
                        value.clone()
                    }
                },
            );
            // Single-element TclPtrSetVar installs the chosen header before
            // List conversion, including a duplicate left after parse failure.
            let _ = vm.var_arena.replace_state(id, Local::Scalar(value));
            let result = match vm.var_arena.get(id).expect("retained append cell").state() {
                Local::Scalar(value) => value
                    .native_list_append_prepared_elements(std::slice::from_ref(addition), protocol),
                _ => unreachable!("installed append header"),
            };
            result.map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let value = vm.read_resolved_cell(id).expect("installed append value");
            vm.store_captured_update(name, key, captured, value)
        })
    }

    pub(crate) fn lappend_instruction_list_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        input: &Value,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        let members = self
            .native_object_list_elements_in(input, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        let captured = self.capture_update_cell(name, key)?;
        self.lappend_instruction_list_captured(name, key, &captured, input, &members, protocol)
    }

    pub(crate) fn lappend_instruction_compiled_list(
        &mut self,
        slot: usize,
        key: Option<&[u8]>,
        input: &Value,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        // GetElements belongs to the input before selected receiver lookup.
        let members = self
            .native_object_list_elements_in(input, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        let (name, captured) = self.capture_compiled_update(slot, key)?;
        self.lappend_instruction_list_captured(
            name.as_bytes(),
            key,
            &captured,
            input,
            &members,
            protocol,
        )
    }

    fn lappend_instruction_list_captured(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        input: &Value,
        members: &[Value],
        protocol: NativeStringProtocol,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        self.with_variable_operation(&captured.cell, |vm| {
            let trace = if let Some(key) = key {
                vm.fire_elem_traces_from_cell_bytes(name, key, "read", Some(captured.cell.clone()))
            } else {
                vm.fire_var_traces_from_cell_bytes(
                    name,
                    "read",
                    None,
                    None,
                    Some(captured.cell.clone()),
                )
            };
            if let Some(refusal) = vm.refused_completion() {
                return Err(refusal);
            }
            let id = captured.cell.id.expect("selected append-list cell");
            let (present, options) = match trace {
                Ok(()) => {
                    let present = vm
                        .initial_value_at_cell(id, captured.explicit_array)
                        .map(|value| value.is_some())
                        .map_err(|error| {
                            crate::command::completion_from_cmd_error(vm, error.into())
                        })?;
                    let options = if present {
                        Value::empty()
                    } else {
                        vm.captured_read_missing(name, key, &captured.cell).options
                    };
                    (present, options)
                }
                Err(error) => {
                    let options = vm.completion_options_snapshot(&error);
                    vm.publish_swallowed_trace_error();
                    (false, options)
                }
            };
            vm.retain_variable_read_error_code(&options);
            let empty =
                tcl_registry::native_instruction_plan::native_list_append_empty_publication(
                    protocol,
                )
                .ok_or_else(|| {
                    crate::command::completion_from_cmd_error(
                        vm,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native List append instruction",
                        )
                        .into(),
                    )
                })?;
            let next = if present {
                let original = match vm.initial_value_at_cell(id, captured.explicit_array) {
                    Ok(original) => original.expect("selected receiver before further callbacks"),
                    Err(error) => {
                        return Err(crate::command::completion_from_cmd_error(vm, error.into()));
                    }
                };
                if members.is_empty() && empty.skip_existing_store {
                    let value = match original.native_object_list_elements(protocol) {
                        Ok(_) => original.clone(),
                        Err(error) => {
                            return Err(crate::command::completion_from_cmd_error(
                                vm,
                                error.into(),
                            ));
                        }
                    };
                    return Ok(vm.variable_update_result(value, &options));
                }
                let next = original.native_list_append_list_elements(members, protocol);
                next.map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
            } else {
                // Native missing receiver adopts this same prepared input.
                if members.is_empty() && empty.fresh_missing_receiver {
                    Value::new_native_string_bytes(&b""[..])
                } else {
                    input.clone()
                }
            };
            let value = vm.store_captured_update(name, key, captured, next)?;
            Ok(vm.variable_update_result(value, &options))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_runtime_api::Code;

    #[test]
    fn c91_empty_list_append_returns_original_array_default_without_store() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
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
        assert_eq!(vm.eval_source("proc watch {n1 n2 op} {lappend ::events $op}; array default set a {A B}; set saved [array default get a]; set events {}; trace add variable a {read write} watch; proc p {} {lappend ::a(missing)}").unwrap().code, Code::Ok);
        let default = vm.get_var_bytes(b"saved").expect("actual retained default");
        let completion = vm.invoke_command("p", &[]);
        assert_eq!(
            completion.code,
            Code::Ok,
            "{:?}",
            completion.result.string_bytes()
        );
        assert_eq!(completion.result.string_bytes().as_ref(), b"A B");
        assert_eq!(
            completion.result.native_object_identity(),
            default.native_object_identity()
        );
        // Pinned C9.1 control: only READ; the missing element remains undefined.
        assert_eq!(
            vm.get_var_bytes(b"events").unwrap().string_bytes().as_ref(),
            b"read"
        );
        let exists = vm.invoke_command(
            "info",
            &[
                Value::new_native_string_bytes(&b"exists"[..]),
                Value::new_native_string_bytes(&b"a(missing)"[..]),
            ],
        );
        assert_eq!(exists.code, Code::Ok);
        assert_eq!(exists.result.string_bytes().as_ref(), b"0");

        let input = Value::native_list_constructor(
            Vec::new(),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        );
        let update = vm
            .lappend_instruction_list_bytes(b"absent", None, &input)
            .unwrap();
        assert_eq!(update.value.string_bytes().as_ref(), b"");
        assert_ne!(
            update.value.native_object_identity(),
            input.native_object_identity()
        );
        assert_eq!(
            update.value.native_object_identity(),
            vm.get_var_bytes(b"absent")
                .unwrap()
                .native_object_identity()
        );
    }

    #[test]
    fn registered_append_instructions_match_43_native_callback_controls() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in
            include_str!("../../../../runtime/rust/tests/data/native_registered_append.tsv").lines()
        {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 7);
            let profile =
                tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
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
            assert_eq!(vm.eval_source("proc watch {n1 n2 op} {lappend ::events $op}; proc arm {} {uplevel 1 {trace add variable r {read write} watch}}; set events {}").unwrap().code, Code::Ok);
            let body = Value::new_native_string_bytes(decode(fields[2]));
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::new_native_string_bytes(&b"p"[..]),
                    Value::new_native_string_bytes(&b"input"[..]),
                    body,
                ],
            );
            assert_eq!(definition.code, Code::Ok);
            let completion =
                vm.invoke_command("p", &[Value::new_native_string_bytes(decode(fields[3]))]);
            assert_eq!(
                completion.code.as_int(),
                fields[4].parse::<i64>().unwrap(),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                completion.result.string_bytes()
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                decode(fields[5]),
                "{}/{}: result",
                fields[0],
                fields[1]
            );
            let events = vm
                .get_var_bytes(b"events")
                .expect("actual callback event variable");
            assert_eq!(
                events.string_bytes().as_ref(),
                decode(fields[6]),
                "{}/{}: observer order",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 43);
    }
}
