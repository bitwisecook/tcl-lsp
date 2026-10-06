// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim switch keeps original argument and current case-list member headers.
use crate::{
    interp::{Vm, ok},
    value::Value,
};
use tcl_cmd_core::{
    CmdError,
    native_jim_switch::{self, Failure, Immediate, NativeJimSwitchObjects, Selection},
};
use tcl_runtime_api::Completion;
use tcl_syntax::{
    native_string::NativeStringProtocol,
    raw_string::{NativeStringAccessError, RawString},
    value::ValueError,
};
static IMMEDIATES: &[&str] = &["default", "-"];
impl NativeJimSwitchObjects for Vm {
    type Value = Value;
    type Callback = Completion<Value>;
    fn switch_bytes(&mut self, value: &Value) -> Result<Vec<u8>, CmdError> {
        value
            .native_string_bytes(NativeStringProtocol::Jim084)
            .map(|bytes| bytes.to_vec())
            .map_err(|error| NativeStringAccessError::Unavailable(error).into())
    }
    fn switch_borrow(&self, value: &Value) -> Value {
        value.native_lifetime_lease().into_value()
    }
    fn switch_list_length(&mut self, value: &Value) -> Result<usize, CmdError> {
        Ok(self
            .native_object_list_elements_in(value, NativeStringProtocol::Jim084)?
            .elements()?
            .len())
    }
    fn switch_list_member(&mut self, value: &Value, index: usize) -> Result<Value, CmdError> {
        let items = self.native_object_list_elements_in(value, NativeStringProtocol::Jim084)?;
        let elements = items.elements()?;
        elements
            .get(index)
            .map(|value| value.native_lifetime_lease().into_value())
            .ok_or_else(|| {
                ValueError::CommandProtocolUnavailable("Jim switch current List member").into()
            })
    }
    fn switch_immediate(&mut self, value: &Value, immediate: Immediate) -> Result<bool, CmdError> {
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            IMMEDIATES,
        );
        self.native_jim_compare_immediate(
            value,
            &table,
            match immediate {
                Immediate::Default => 0,
                Immediate::Dash => 1,
            },
        )
        .map_err(Into::into)
    }
    fn switch_equal(&mut self, subject: &Value, pattern: &Value) -> Result<bool, CmdError> {
        if subject.native_object_identity() == pattern.native_object_identity() {
            return Ok(true);
        }
        Ok(self.switch_bytes(subject)? == self.switch_bytes(pattern)?)
    }
    fn switch_glob(&mut self, pattern: &Value, subject: &Value) -> Result<bool, CmdError> {
        let pattern = RawString::from_bytes(self.switch_bytes(pattern)?);
        let subject = RawString::from_bytes(self.switch_bytes(subject)?);
        subject.jim084_matches(&pattern, false).map_err(Into::into)
    }
    fn switch_callback_code(&self, callback: &Completion<Value>) -> i32 {
        i32::try_from(callback.code.as_int()).expect("native callback completion code")
    }
    fn switch_negative_match(&mut self, code: i32) -> Completion<Value> {
        match self.with_native_interp_result(|value| value.native_lifetime_lease().into_value()) {
            Ok(value) => {
                Completion::new(tcl_runtime_api::Code::from_int(code), value, Value::empty())
            }
            Err(error) => crate::command::completion_from_cmd_error(self, error.into()),
        }
    }
    fn switch_command(
        &mut self,
        command: Option<&Value>,
        pattern: &Value,
        subject: &Value,
        option_end: bool,
    ) -> Result<i64, Completion<Value>> {
        let head;
        let original = if let Some(command) = command {
            command
        } else {
            head =
                Value::new_native_string_bytes(b"regexp".as_slice()).into_native_unowned_lifetime();
            &head
        };
        crate::cmd_regexp::invoke_jim_match_command(
            self, original, pattern, subject, false, option_end,
        )
    }
}
pub(super) fn invoke(
    vm: &mut Vm,
    args: &[Value],
    protocol: tcl_registry::native_jim_switch::NativeJimSwitchProtocol,
) -> Completion<Value> {
    let selection = match native_jim_switch::select(vm, protocol, args) {
        Ok(selection) => selection,
        Err(Failure::Command(error)) => {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        Err(Failure::Callback(completion)) => return completion,
    };
    if let Err(error) = vm.reset_native_jim_result() {
        return crate::command::completion_from_cmd_error(vm, error.into());
    }
    match selection {
        Selection::Empty => {
            match vm.with_native_interp_result(|value| value.native_lifetime_lease().into_value()) {
                Ok(value) => ok(value),
                Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
            }
        }
        Selection::Body(body) => match vm.eval_original_script_value(
            &body,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            None,
        ) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(vm, error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    struct Callback<F>(F);
    impl<F> crate::command::NativeCommand for Callback<F>
    where
        F: Fn(&mut Vm, &[Value]) -> Completion<Value>,
    {
        fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            (self.0)(vm, args)
        }
    }
    fn register_matcher(
        vm: &mut Vm,
        callback: impl Fn(&mut Vm, &[Value]) -> Completion<Value> + 'static,
    ) {
        vm.register_written_command(
            "matcher",
            crate::command::Command::Native(Rc::new(Callback(callback))),
        );
    }
    fn hex(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2));
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn install_case_matcher(
        vm: &mut Vm,
        index: usize,
        list: crate::value::NativeObjectLifetimeLease,
        calls: &Rc<Cell<usize>>,
    ) {
        let count = Rc::clone(calls);
        register_matcher(vm, move |vm, _| {
            count.set(count.get() + 1);
            if index == 30
                && let Err(error) = list.value().native_character_count_with_protocol(NativeStringProtocol::Jim084,tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString) {
                    return crate::command::completion_from_cmd_error(vm,error.into());
                }
            if index == 31 && count.get() == 1 {
                let count = Rc::clone(&count);
                register_matcher(vm, move |_, _| {
                    count.set(count.get() + 1);
                    ok(Value::int(1))
                });
                return ok(Value::int(0));
            }
            if index == 27 {
                return ok(Value::int(0));
            }
            if index == 28 {
                return ok(Value::new_native_string_bytes(b"NONNUMERIC".as_slice()));
            }
            if index == 29 {
                return Completion::new(
                    tcl_runtime_api::Code::Error,
                    Value::new_native_string_bytes(b"CALLBACK_ERROR".as_slice()),
                    Value::empty(),
                );
            }
            ok(Value::int(1))
        });
    }

    #[test]
    fn switch_matches_all_192_original_native_option_and_callback_results() {
        let cases = include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/cases.tsv");
        let mut compared = 0;
        for (profile, rows) in [
            (
                "tcl8.4",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/jim0.84.tsv"),
            ),
        ] {
            for (index, row) in rows
                .lines()
                .filter(|line| line.starts_with("RESULT\t"))
                .enumerate()
            {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields[1].parse::<usize>().unwrap(), index);
                let case: Vec<_> = cases.lines().nth(index).unwrap().split('\t').collect();
                let profile_axis =
                    tcl_registry::model::ingress::resolve_environment(profile).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile_axis);
                let original: Vec<_> = case[1..]
                    .iter()
                    .map(|word| Value::new_native_string_bytes(hex(word)))
                    .collect();
                let list = original.last().unwrap().native_lifetime_lease();
                let calls = Rc::new(Cell::new(0));
                install_case_matcher(&mut vm, index, list, &calls);
                let args: Vec<_> = original[1..]
                    .iter()
                    .map(|value| value.native_lifetime_lease().into_value())
                    .collect();
                let result = vm.invoke_host_original_object_vector(&original[0], &args);
                assert!(
                    vm.refused_completion().is_none(),
                    "{profile} {}: {:?}",
                    case[0],
                    vm.refused_completion()
                );
                assert_eq!(
                    result.code.as_int(),
                    fields[3].parse::<i64>().unwrap(),
                    "{profile} {}",
                    case[0]
                );
                assert_eq!(
                    calls.get(),
                    fields[4].parse::<usize>().unwrap(),
                    "{profile} {}: callback count",
                    case[0]
                );
                assert_eq!(
                    result
                        .result
                        .native_string_bytes(
                            vm.actual_native_invocation_dialect()
                                .native_string_protocol()
                                .unwrap()
                        )
                        .unwrap()
                        .as_ref(),
                    hex(fields[5]).as_slice(),
                    "{profile} {}",
                    case[0]
                );
                if profile == "jim" {
                    assert_jim_original_objects(profile, rows, index, case[0], &original);
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 192);
    }
    fn assert_jim_original_objects(
        profile: &str,
        rows: &str,
        index: usize,
        case: &str,
        original: &[Value],
    ) {
        for object in rows
            .lines()
            .filter(|line| line.starts_with(&format!("OBJECT\t{index}\t")))
        {
            let object: Vec<_> = object.split('\t').collect();
            let value = &original[object[2].parse::<usize>().unwrap()];
            let expected = if object[3] == "NULL" {
                "none"
            } else {
                object[3]
            };
            assert_eq!(
                value.native_object_type_name(),
                expected,
                "{profile} {} original {}",
                case,
                object[2]
            );
            assert_eq!(
                value.native_object_reference_count(),
                object[4].parse::<usize>().unwrap(),
                "{profile} {} original {} references",
                case,
                object[2]
            );
            assert_eq!(
                value.resident_string_bytes().is_some(),
                object[5] == "1",
                "{profile} {} original {} resident",
                case,
                object[2]
            );
        }
    }

    fn install_refetch_matcher(
        vm: &mut Vm,
        outcome: usize,
        original: crate::value::NativeObjectLifetimeLease,
        observer: Rc<std::cell::RefCell<Option<Value>>>,
        captured: Rc<std::cell::RefCell<Vec<String>>>,
    ) {
        register_matcher(vm, move |vm, args| {
            let items = original
                .value()
                .native_object_list_elements(NativeStringProtocol::Jim084)
                .unwrap();
            let body = items.elements().unwrap()[1].clone();
            let pattern = &args[args.len() - 2];
            captured.borrow_mut().push(format!(
                "BEFORE\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{}",
                original.value().native_object_reference_count(),
                body.native_object_type_name(),
                body.native_object_reference_count(),
                pattern.native_object_type_name(),
                pattern.native_object_reference_count(),
                usize::from(
                    pattern.native_object_identity()
                        == items.elements().unwrap()[0].native_object_identity()
                )
            ));
            *observer.borrow_mut() = Some(body);
            drop(items);
            original.value().native_character_count_with_protocol(NativeStringProtocol::Jim084,tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString).unwrap();
            let old = observer.borrow();
            let body = old.as_ref().unwrap();
            captured.borrow_mut().push(format!(
                "AFTER_SHIMMER\t{outcome}\t{}\t{}\t{}\t{}\t{}",
                original.value().native_object_type_name(),
                body.native_object_type_name(),
                body.native_object_reference_count(),
                pattern.native_object_type_name(),
                pattern.native_object_reference_count()
            ));
            if outcome == 1 {
                return Completion::new(
                    tcl_runtime_api::Code::Error,
                    Value::new_native_string_bytes(b"FAILED".as_slice()),
                    Value::empty(),
                );
            }
            if outcome == 2 {
                return ok(Value::new_native_string_bytes(b"NONNUMERIC".as_slice()));
            }
            let _ = vm;
            ok(Value::int(1))
        });
    }

    #[test]
    fn same_case_list_is_refetched_in_all_12_native_ownership_windows() {
        use std::cell::RefCell;
        let mut windows = Vec::new();
        for outcome in 0..3 {
            let axis = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                Rc::new(crate::host_native::NativeHost::new()),
                axis,
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let root = Value::new_native_string_bytes(b"X {list MATCH}".as_slice());
            let original = root.native_lifetime_lease();
            let old = Rc::new(RefCell::new(None::<Value>));
            let observer = Rc::clone(&old);
            let transcript = Rc::new(RefCell::new(Vec::new()));
            let captured = Rc::clone(&transcript);
            install_refetch_matcher(&mut vm, outcome, original, observer, captured);
            let head = Value::new_native_string_bytes(b"switch".as_slice());
            let args = [
                Value::new_native_string_bytes(b"-command".as_slice()),
                Value::new_native_string_bytes(b"matcher".as_slice()),
                Value::new_native_string_bytes(b"X".as_slice()),
                root.native_lifetime_lease().into_value(),
            ];
            let result = vm.invoke_host_original_object_vector(&head, &args);
            assert!(
                vm.refused_completion().is_none(),
                "outcome={outcome}: {:?}",
                vm.execution_refusal
            );
            windows.extend(transcript.borrow().iter().cloned());
            let items = root
                .native_object_list_elements(NativeStringProtocol::Jim084)
                .unwrap();
            let child = &items.elements().unwrap()[1];
            let old = old.borrow();
            let old = old.as_ref().unwrap();
            windows.push(format!(
                "AFTER_SWITCH\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                result.code.as_int(),
                root.native_object_type_name(),
                root.native_object_reference_count(),
                old.native_object_type_name(),
                old.native_object_reference_count(),
                child.native_object_type_name(),
                child.native_object_reference_count(),
                usize::from(child.native_object_identity() == old.native_object_identity())
            ));
            windows.push(format!(
                "RESULT\t{outcome}\t{}",
                std::str::from_utf8(
                    &result
                        .result
                        .native_string_bytes(NativeStringProtocol::Jim084)
                        .unwrap()
                )
                .unwrap()
            ));
        }
        assert_eq!(
            windows.join("\n"),
            include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv")
                .strip_suffix('\n')
                .expect("native fixture ends with one record newline")
        );
        assert_eq!(windows.len(), 12);
    }
}
