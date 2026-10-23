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

//! Original Jim interpreter String crossings and parent-owned alias prefixes.

use super::native_numeric_ops::OriginalNumericOps;
use super::native_operation_currency::NativeOperationScope;
use super::{Completion, InterpId, Local, Vm};
use crate::Value;
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_registry::special_vars::NativeJimInterpreterCopyPurpose;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

pub(super) struct ConstructedJimObject(Option<Value>);
impl ConstructedJimObject {
    fn borrowed(&self) -> &Value {
        self.0.as_ref().expect("actual Jim constructor temporary")
    }
    fn publish(mut self) -> Value {
        self.0
            .take()
            .expect("actual Jim constructor temporary")
            .into_native_reference()
    }
}
impl Drop for ConstructedJimObject {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            value.retire_unowned_native_header();
        }
    }
}

impl Vm {
    pub(super) fn copy_original_jim_value_to(
        &mut self,
        destination: InterpId,
        original: &Value,
        purpose: NativeJimInterpreterCopyPurpose,
    ) -> Result<ConstructedJimObject, CmdError> {
        let source = NativeOperationScope::enter(self)?;
        if !self.interp_alive(destination) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim copy receiver lifetime",
            )
            .into());
        }
        let bytes = {
            let ops = OriginalNumericOps::in_original_scope(self, &source)?;
            ops.string(original, NativeStringProtocol::Jim084)?
        };
        source.check(self)?;
        if !self.interp_alive(destination) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim copy receiver lifetime",
            )
            .into());
        }
        let copied = self.in_interp(destination, |vm| {
            let receiver = NativeOperationScope::enter(vm)?;
            let context = vm.native_jim_object_context()?;
            let value = Value::new_native_string_bytes(purpose.input(&bytes));
            receiver.check(vm)?;
            value.bind_native_jim_context(&context)?;
            Ok::<_, CmdError>(ConstructedJimObject(Some(
                value.into_native_unowned_lifetime(),
            )))
        });
        source.check(self)?;
        copied
    }

    pub(super) fn capture_original_jim_parent_alias_prefix(
        &mut self,
        target: InterpId,
        words: Vec<Value>,
    ) -> Result<(Rc<Value>, Rc<Vec<Value>>), CmdError> {
        if target != self.cur_interp() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim parent alias producer",
            )
            .into());
        }
        let scope = NativeOperationScope::enter(self)?;
        let context = self.native_jim_object_context()?;
        for word in &words {
            word.bind_native_jim_context(&context)?;
            scope.check(self)?;
        }
        let prefix = Value::native_list_constructor(words, NativeStringProtocol::Jim084);
        scope.check(self)?;
        prefix.bind_native_jim_context(&context)?;
        let retained = prefix
            .native_list_backing_in(NativeStringProtocol::Jim084)?
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim parent prefix List",
            ))?;
        let words = retained
            .iter()
            .map(|word| word.native_lifetime_lease().into_value())
            .collect();
        scope.check(self)?;
        Ok((Rc::new(prefix), Rc::new(words)))
    }

    pub(super) fn copy_original_jim_child_startup_variables(
        &mut self,
        child: InterpId,
        initialisation: tcl_registry::special_vars::NativeChildInitialisation,
    ) -> Result<(), CmdError> {
        let parent = NativeOperationScope::enter(self)?;
        for &name in initialisation.copied_parent_variables() {
            parent.check(self)?;
            let Some(resolved) = self.resolve_var_from_bytes(name, 0) else {
                continue;
            };
            let Some(value) = self.read_variable_contents(&resolved) else {
                continue;
            };
            let original = value.native_lifetime_lease();
            drop(value);
            let copied = self.copy_original_jim_value_to(
                child,
                original.value(),
                NativeJimInterpreterCopyPurpose::ChildStartupVariable,
            )?;
            self.in_interp(child, |vm| {
                let receiver = NativeOperationScope::enter(vm)?;
                let target = vm.resolve_var_from_bytes(name, 0).ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim child global publication"),
                )?;
                if target.elem.is_some()
                    || target.id.is_some_and(|id| {
                        matches!(
                            vm.var_arena.get(id).map(crate::vars::VarCell::state),
                            Some(Local::Array(_))
                        )
                    })
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim child scalar publication",
                    )
                    .into());
                }
                let id = vm
                    .ensure_target_var_at_binding(&target.binding, None)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim child global cell",
                    ))?;
                if !vm
                    .var_arena
                    .replace_state(id, Local::Scalar(copied.publish()))
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim child global cell retirement",
                    )
                    .into());
                }
                receiver.check(vm)
            })?;
            parent.check(self)?;
        }
        Ok(())
    }

    pub(super) fn eval_original_jim_child_object(
        &mut self,
        target: InterpId,
        original: &Value,
    ) -> Completion<Value> {
        let caller = self.cur_interp();
        let copied = match self.copy_original_jim_value_to(
            target,
            original,
            NativeJimInterpreterCopyPurpose::Object,
        ) {
            Ok(copied) => copied,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let result = self.in_interp(target, |vm| {
            let scope = NativeOperationScope::enter(vm)?;
            let completion = match vm.eval_original_script_value(
                copied.borrowed(),
                tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
                None,
            ) {
                Ok(completion) => completion,
                Err(error) => crate::command::completion_from_tcl_error(vm, error),
            };
            scope.check(vm)?;
            drop(copied);
            scope.check(vm)?;
            let result = vm.copy_original_jim_value_to(
                caller,
                &completion.result,
                NativeJimInterpreterCopyPurpose::Object,
            )?;
            Ok::<_, CmdError>((completion.code, result))
        });
        match result {
            Ok((code, result)) => match self.current_jim_options_for_exit_code(code) {
                Ok(options) => Completion::new(code, result.publish(), options),
                Err(error) => crate::command::completion_from_cmd_error(self, error),
            },
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }

    pub(crate) fn invoke_original_jim_parent_alias(
        &mut self,
        parent: InterpId,
        prefix: &Value,
        arguments: &[Value],
    ) -> Completion<Value> {
        let caller = self.cur_interp();
        if !self.interp_alive(parent) {
            return crate::command::completion_from_cmd_error(
                self,
                ValueError::CommandProtocolUnavailable("original Jim alias parent lifetime").into(),
            );
        }
        let prepared = self.in_interp(parent, |vm| {
            let scope = NativeOperationScope::enter(vm)?;
            let context = vm.native_jim_object_context()?;
            prefix.bind_native_jim_context(&context)?;
            scope.check(vm)?;
            let duplicate = {
                let ops = OriginalNumericOps::in_original_scope(vm, &scope)?;
                prefix.duplicate_native_object_checked(
                    NativeStringProtocol::Jim084,
                    &mut || ops.current_for_value(prefix),
                    &mut |value| ops.string(value, NativeStringProtocol::Jim084),
                    &mut |error| ops.retain_failure(error),
                )?
            };
            scope.check(vm)?;
            duplicate.bind_native_jim_context(&context)?;
            Ok::<_, CmdError>(ConstructedJimObject(Some(
                duplicate.into_native_unowned_lifetime(),
            )))
        });
        let script = match prepared {
            Ok(script) => script,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let mut copied = Vec::with_capacity(arguments.len());
        for argument in arguments {
            match self.copy_original_jim_value_to(
                parent,
                argument,
                NativeJimInterpreterCopyPurpose::Object,
            ) {
                Ok(value) => copied.push(value),
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            }
        }
        if !self.interp_alive(parent) {
            return crate::command::completion_from_cmd_error(
                self,
                ValueError::CommandProtocolUnavailable("original Jim alias parent lifetime").into(),
            );
        }
        let result = self.in_interp(parent, |vm| {
            let scope = NativeOperationScope::enter(vm)?;
            {
                let ops = OriginalNumericOps::in_original_scope(vm, &scope)?;
                let words = copied
                    .iter()
                    .map(|copy| copy.borrowed().native_lifetime_lease().into_value())
                    .collect::<Vec<_>>();
                script
                    .borrowed()
                    .native_list_append_prepared_elements_checked(
                        &words,
                        NativeStringProtocol::Jim084,
                        &mut || ops.current_for_value(script.borrowed()),
                        &mut |word| ops.string(word, NativeStringProtocol::Jim084),
                    )?;
            }
            drop(copied);
            scope.check(vm)?;
            let completion = match vm.eval_original_script_value(
                script.borrowed(),
                tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
                None,
            ) {
                Ok(completion) => completion,
                Err(error) => crate::command::completion_from_tcl_error(vm, error),
            };
            scope.check(vm)?;
            drop(script);
            scope.check(vm)?;
            let result = vm.copy_original_jim_value_to(
                caller,
                &completion.result,
                NativeJimInterpreterCopyPurpose::Object,
            )?;
            Ok::<_, CmdError>((completion.code, result))
        });
        match result {
            Ok((code, result)) => match self.current_jim_options_for_exit_code(code) {
                Ok(options) => Completion::new(code, result.publish(), options),
                Err(error) => crate::command::completion_from_cmd_error(self, error),
            },
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::NativeJimSourceInfo;

    fn pair() -> (Vm, InterpId) {
        let profile = crate::environment::profile_for_dialect("jim");
        let mut vm = crate::native_fixture::core(profile);
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let child = vm
            .create_child_named_bytes(
                tcl_core_types::NameBytes::from(b"receiver".as_slice()),
                false,
            )
            .unwrap();
        (vm, child)
    }

    #[test]
    fn original_copy_has_receiver_birth_counted_bytes_and_no_source_header() {
        let (mut vm, child) = pair();
        let parent = vm.cur_interp();
        let original = {
            let _scope = NativeOperationScope::enter(&mut vm).unwrap();
            Value::new_native_string_bytes(b"A\0\xc3\xa9".as_slice())
        };
        let parent_context = vm.native_jim_object_context().unwrap();
        original
            .install_native_jim_source(
                NativeJimSourceInfo {
                    filename: parent_context.empty_object().clone(),
                    line: 7,
                },
                &parent_context,
            )
            .unwrap();
        let count = original.native_object_reference_count();
        let copied = vm
            .copy_original_jim_value_to(child, &original, NativeJimInterpreterCopyPurpose::Object)
            .unwrap();
        assert_eq!(copied.borrowed().native_object_reference_count(), 0);
        assert_eq!(
            copied.borrowed().resident_string_bytes().unwrap().as_ref(),
            b"A\0\xc3\xa9"
        );
        assert_eq!(copied.borrowed().native_object_type_name(), "none");
        let receiver = vm.in_interp(child, |vm| vm.native_jim_object_context().unwrap());
        assert!(copied.borrowed().bind_native_jim_context(&receiver).is_ok());
        assert!(
            copied
                .borrowed()
                .bind_native_jim_context(&parent_context)
                .is_err()
        );
        assert!(original.bind_native_jim_context(&receiver).is_err());
        assert_eq!(original.native_object_reference_count(), count);
        assert_eq!(
            original
                .pin_native_jim_source_info(&parent_context)
                .unwrap()
                .line,
            7
        );
        let published = copied.publish();
        assert_eq!(published.native_object_reference_count(), 1);
        let returned = vm
            .in_interp(child, |vm| {
                vm.copy_original_jim_value_to(
                    parent,
                    &published,
                    NativeJimInterpreterCopyPurpose::Object,
                )
            })
            .unwrap();
        assert!(
            returned
                .borrowed()
                .bind_native_jim_context(&parent_context)
                .is_ok()
        );
        assert!(
            returned
                .borrowed()
                .bind_native_jim_context(&receiver)
                .is_err()
        );
        assert_eq!(
            returned
                .borrowed()
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"A\0\xc3\xa9"
        );
        assert!(vm.delete_interp(child));
        assert!(
            vm.copy_original_jim_value_to(
                child,
                &original,
                NativeJimInterpreterCopyPurpose::Object
            )
            .is_err()
        );
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_parent_prefix_list_owns_members_without_snapshot_references() {
        let (mut vm, child) = pair();
        let parent = vm.cur_interp();
        let original = {
            let _scope = NativeOperationScope::enter(&mut vm).unwrap();
            Value::new_native_string_bytes(b"set".as_slice())
        };
        let count = original.native_object_reference_count();
        let (prefix, words) = vm
            .capture_original_jim_parent_alias_prefix(parent, vec![original.clone()])
            .unwrap();
        assert_eq!(prefix.native_object_reference_count(), 1);
        assert_eq!(original.native_object_reference_count(), count + 1);
        assert!(words[0].is_same_object(&original));
        assert!(
            vm.capture_original_jim_parent_alias_prefix(child, vec![original.clone()])
                .is_err()
        );
        drop(prefix);
        assert_eq!(original.native_object_reference_count(), count);
        assert!(words[0].is_same_object(&original));
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_parent_alias_copies_operands_and_result_and_keeps_return_state_separate() {
        let (mut vm, child) = pair();
        let parent = vm.cur_interp();
        let (prefix, _) = {
            let _scope = NativeOperationScope::enter(&mut vm).unwrap();
            vm.capture_original_jim_parent_alias_prefix(
                parent,
                vec![
                    Value::new_native_string_bytes(b"set".as_slice()),
                    Value::new_native_string_bytes(b"::saved".as_slice()),
                ],
            )
            .unwrap()
        };
        vm.in_interp(child, |vm| {
            let _scope = NativeOperationScope::enter(vm).unwrap();
            let argument = Value::new_native_string_bytes(b"A\0B".as_slice());
            let count = argument.native_object_reference_count();
            let completion = vm.invoke_original_jim_parent_alias(
                parent,
                &prefix,
                &[argument.native_lifetime_lease().into_value()],
            );
            assert_eq!(completion.code, tcl_core_types::Code::Ok);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"A\0B"
            );
            assert!(!completion.result.is_same_object(&argument));
            completion
                .result
                .bind_native_jim_context(&vm.native_jim_object_context().unwrap())
                .unwrap();
            assert_eq!(argument.native_object_reference_count(), count);
        });
        let (prefix, _) = {
            let _scope = NativeOperationScope::enter(&mut vm).unwrap();
            vm.capture_original_jim_parent_alias_prefix(
                parent,
                vec![
                    Value::new_native_string_bytes(b"return".as_slice()),
                    Value::new_native_string_bytes(b"-code".as_slice()),
                    Value::new_native_string_bytes(b"7".as_slice()),
                ],
            )
            .unwrap()
        };
        vm.in_interp(child, |vm| {
            let _scope = NativeOperationScope::enter(vm).unwrap();
            let argument = Value::new_native_string_bytes(b"PAYLOAD".as_slice());
            let completion = vm.invoke_original_jim_parent_alias(parent, &prefix, &[argument]);
            assert_eq!(completion.code, tcl_core_types::Code::Return);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"PAYLOAD"
            );
            assert_eq!(
                vm.jim_errors.pending_return,
                tcl_runtime_api::jim_return_state::JimReturnState::default()
            );
        });
        assert_eq!(vm.jim_errors.pending_return.code, 7);
        assert_eq!(vm.jim_errors.pending_return.level, 1);
        assert!(vm.execution_refusal.is_none());
    }
}
