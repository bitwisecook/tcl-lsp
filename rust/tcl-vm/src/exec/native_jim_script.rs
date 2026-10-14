// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ExpressionReq, Frame, ScriptNamespace, SubstReq, Tick, pop};
use crate::{
    Code, Completion, Vm,
    command::completion_from_tcl_error,
    interp::ok,
    native_jim_script::{NativeJimScriptState, ScriptStep},
};
use tcl_syntax::jim_script_objects::JimScriptObjectKind;

impl Vm {
    pub(crate) fn original_jim_tuple_children_in_context(
        &mut self,
        original: &crate::Value,
        names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<
        (
            crate::NativeObjectLifetimeLease,
            crate::NativeObjectLifetimeLease,
        ),
        tcl_cmd_core::CmdError,
    > {
        names.current_for_value(self, original)?;
        let context = self.native_jim_object_context();
        names.current_for_value(self, original)?;
        let context = context?;
        let prepared = original
            .ensure_native_jim_dictionary_substitution_checked(&context, &mut || {
                names.current_for_value(self, original)
            });
        names.current_for_value(self, original)?;
        prepared?;
        let children = original
            .with_native_jim_dictionary_substitution(|name, key| {
                (name.native_lifetime_lease(), key.native_lifetime_lease())
            })
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim tuple projection",
            ))?;
        names.current_for_value(self, original)?;
        Ok(children)
    }

    pub(crate) fn retain_original_jim_tuple_failure(&mut self, error: &tcl_cmd_core::CmdError) {
        if let Some(cause) = error.native_execution_refusal() {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        } else if let Some(cause) = error.native_access_refusal() {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(cause));
        }
    }

    pub(super) fn tick_native_jim_script(&mut self, frame: &mut Frame) -> Tick {
        if let Some(entry) = frame.jim_script_entry.take() {
            let prepared = NativeJimScriptState::new_for_evaluation_frame(
                entry,
                self,
                &mut frame.jim_evaluation.script,
            );
            match prepared {
                Ok(state) => {
                    frame.jim_evaluation.evaluation_frame_entered = state.enters_evaluation_frame();
                    frame.jim_script = Some(state);
                }
                Err(error) => {
                    if let Some(current) = self.jim_errors.frames.last_mut() {
                        *current = Self::borrow_jim_evaluation_frame(&frame.jim_evaluation);
                    }
                    return Tick::Return(completion_from_tcl_error(self, error));
                }
            }
        }
        loop {
            let reached = frame
                .jim_script
                .as_mut()
                .expect("native Script state")
                .next(self);
            let location = frame
                .jim_script
                .as_ref()
                .expect("native Script state")
                .current_script_location();
            match location {
                Ok(location) => frame.jim_evaluation.script = location,
                Err(error) => return Tick::Return(completion_from_tcl_error(self, error)),
            }
            if let Some(current) = self.jim_errors.frames.last_mut() {
                *current = Self::borrow_jim_evaluation_frame(&frame.jim_evaluation);
            }
            let step = match reached {
                Ok(step) => step,
                Err(error) => return Tick::Return(completion_from_tcl_error(self, error)),
            };
            match step {
                ScriptStep::Complete(completion) => return Tick::Return(completion),
                ScriptStep::Invoke(words) => {
                    let completion = if words.is_empty() {
                        ok(self
                            .native_jim_object_context()
                            .expect("active original context")
                            .result_object())
                    } else {
                        match self.dispatch_words(frame, &words) {
                            Ok(Some(tick)) => return tick,
                            Ok(None) => {
                                Completion::new(Code::Ok, pop(frame), frame.last_options.clone())
                            }
                            Err(completion) => completion,
                        }
                    };
                    frame
                        .jim_script
                        .as_mut()
                        .expect("native Script state")
                        .accept(self, completion);
                }
                ScriptStep::Token { index } => {
                    let completion = match self.native_jim_token_completion(frame, index) {
                        Ok(completion) => completion,
                        Err(tick) => return *tick,
                    };
                    frame
                        .jim_script
                        .as_mut()
                        .expect("native Script state")
                        .accept(self, completion);
                }
            }
        }
    }
    fn native_jim_token_completion(
        &mut self,
        frame: &mut Frame,
        index: usize,
    ) -> Result<Completion<crate::Value>, Box<Tick>> {
        let (kind, original) = frame
            .jim_script
            .as_ref()
            .expect("native Script state")
            .original_token(index);
        let original = original.native_lifetime_lease();
        let completion = match kind {
            JimScriptObjectKind::Source(
                tcl_lexer::JimScriptTokenKind::String | tcl_lexer::JimScriptTokenKind::Escaped,
            ) => ok(original.value().clone()),
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Variable) => {
                match self.read_original_named_variable(original.value()) {
                    Ok(value) => ok(value),
                    Err(completion) => completion,
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Command) => {
                match self.prepare_script_commands_value(original.value()) {
                    Ok(prepared) => {
                        return Err(Box::new(Tick::PushScript {
                            script: Box::new(
                                prepared
                                    .prefix
                                    .expect("native Script always owns an activation"),
                            ),
                            label: None,
                            cleanup_proc: None,
                            fatal_tail: None,
                            namespace: ScriptNamespace::Inherit,
                        }));
                    }
                    Err(error) => completion_from_tcl_error(self, error),
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => {
                match self.prepare_original_expression_value(original.value()) {
                    Ok(node) => {
                        return Err(Box::new(Tick::PushExpression {
                            req: Box::new(ExpressionReq {
                                state: tcl_syntax::expr::ExprEvalState::new(node),
                                awaiting_array: None,
                                normalize: false,
                                jim_objects: original.value().native_jim_expression_objects(),
                                restore_primary: original.value().retain_expression_primary(),
                            }),
                            placeholder: Box::new(self.current_placeholder_unit()),
                        }));
                    }
                    Err(error) => completion_from_tcl_error(self, error),
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::IndexedVariable) => {
                return Err(Box::new(self.native_jim_indexed_token(frame, &original)));
            }
            _ => Completion::new(
                Code::Error,
                self.native_jim_object_context()
                    .expect("active native context")
                    .result_object(),
                crate::Value::empty(),
            ),
        };
        Ok(completion)
    }

    fn native_jim_indexed_token(
        &mut self,
        frame: &mut Frame,
        original: &crate::NativeObjectLifetimeLease,
    ) -> Tick {
        let mut names =
            match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self) {
                Ok(names) => Some(names),
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_cmd_error(self, error));
                }
            };
        let operation = names
            .as_ref()
            .expect("entered original tuple context")
            .operation_scope()
            .clone();
        let reached = (|| -> Result<Tick, tcl_cmd_core::CmdError> {
            let context = names.as_ref().expect("actual original tuple context");
            let (name, key) =
                self.original_jim_tuple_children_in_context(original.value(), context)?;
            let tick = Tick::PushSubst {
                req: Box::new(SubstReq {
                    compiled: None,
                    original: Some(key),
                    template: Vec::<u8>::new().into(),
                    backslashes: true,
                    commands: true,
                    variables: true,
                    control: crate::subst::SubstitutionControl::Word,
                }),
                placeholder: Box::new(self.current_placeholder_unit()),
            };
            context.current_for_value(self, original.value())?;
            let parked = names.take().expect("same original tuple context").park();
            frame
                .jim_script
                .as_mut()
                .expect("native Script state")
                .begin_dictionary(original.value().native_lifetime_lease(), name, parked);
            Ok(tick)
        })();
        if let Err(error) = &reached {
            self.retain_original_jim_tuple_failure(error);
        }
        let cleanup = names.map_or(Ok(()), |names| names.finish(self));
        if let Err(first) = operation.check(self).and(cleanup) {
            drop(reached);
            let _ = operation.check(self);
            return Tick::Return(crate::command::completion_from_cmd_error(self, first));
        }
        // The pending record contains saved currency only. This local entered
        // clone retires before suspension; genuine dispatcher parents stay separate.
        match reached {
            Ok(tick) => tick,
            Err(error) => Tick::Return(crate::command::completion_from_cmd_error(self, error)),
        }
    }
}

impl Vm {
    fn original_jim_dictionary_stage<T>(
        &mut self,
        reached: Result<T, tcl_syntax::value::ValueError>,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        let reached = reached.map_err(tcl_cmd_core::CmdError::from);
        if let Err(error) = &reached {
            self.retain_original_jim_tuple_failure(error);
        }
        reached
    }

    pub(crate) fn read_native_jim_dictionary_member(
        &mut self,
        name: &crate::Value,
        key: &crate::Value,
        names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<crate::Value, Completion<crate::Value>> {
        let reached = (|| -> Result<Result<crate::Value, Completion<crate::Value>>, tcl_cmd_core::CmdError> {
            names.current_for_value(self, name)?;
            names.current_for_value(self, key)?;
            let read = self.read_original_named_variable(name);
            names.current_for_value(self, name)?;
            let root = match read {
                Ok(root) => root,
                Err(completion) => return Ok(Err(completion)),
            };
            let root_lease = root.native_lifetime_lease();
            drop(root);
            let root = root_lease.value();
            names.current_for_value(self, root)?;
            let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
            let converted = {
                let ops = crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, names.operation_scope())?;
                ops.dictionary_with_currency(root, protocol, names.operation_scope().currency())
            };
            if let Err(error) = &converted {
                self.retain_original_jim_tuple_failure(error);
            }
            names.current_for_value(self, root)?;
            if converted.is_err() {
                drop(converted);
                return Ok(Err(self.native_jim_dictionary_read_error(name, key, true, names, root)));
            }
            // Match the old temporary projection's native role retirement stage.
            drop(converted);
            names.current_for_value(self, root)?;
            let key_bytes = names.read(self, key)?;
            names.current_for_value(self, root)?;
            // The current table is selected AFTER the key getter. Earlier legal
            // table mutations do not poison a cursor which did not yet exist.
            let table = self.original_jim_dictionary_stage(root.original_dictionary_cursor())?;
            let mut index = 0;
            loop {
                names.current_for_value(self, root)?;
                let Some((candidate, value)) = self.original_jim_dictionary_stage(table.member(root, index))? else {
                    return Ok(Err(self.native_jim_dictionary_read_error(name, key, false, names, root)));
                };
                let reached = names.read(self, candidate.value());
                if let Err(error) = &reached { self.retain_original_jim_tuple_failure(error); }
                names.current_for_value(self, root)?;
                self.original_jim_dictionary_stage(table.check(root))?;
                let bytes = reached?;
                if bytes.as_ref() == key_bytes.as_ref() {
                    names.current_for_value(self, value.value())?;
                    let retained = value.into_value().into_native_reference();
                    self.original_jim_dictionary_stage(table.check(root))?;
                    return Ok(Ok(retained));
                }
                drop(candidate);
                drop(value);
                names.current_for_value(self, root)?;
                self.original_jim_dictionary_stage(table.check(root))?;
                index += 1;
            }
        })();
        if let Err(error) = &reached {
            self.retain_original_jim_tuple_failure(error);
        }
        match names.check(self) {
            Err(first) => {
                drop(reached);
                let _ = names.check(self);
                Err(crate::command::completion_from_cmd_error(self, first))
            }
            Ok(()) => match reached {
                Ok(result) => result,
                Err(error) => Err(crate::command::completion_from_cmd_error(self, error)),
            },
        }
    }

    fn native_jim_dictionary_read_error(
        &mut self,
        name: &crate::Value,
        key: &crate::Value,
        malformed: bool,
        names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
        root: &crate::Value,
    ) -> Completion<crate::Value> {
        let bytes = (|| -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
            names.current_for_value(self, root)?;
            let name = names.read(self, name)?;
            names.current_for_value(self, root)?;
            let key = names.read(self, key)?;
            names.current_for_value(self, root)?;
            let mut bytes = b"can't read \"".to_vec();
            bytes.extend_from_slice(&name);
            bytes.push(b'(');
            bytes.extend_from_slice(&key);
            bytes.extend_from_slice(if malformed {
                b")\": variable isn't array"
            } else {
                b")\": no such element in array"
            });
            Ok(bytes)
        })();
        if let Err(error) = &bytes {
            self.retain_original_jim_tuple_failure(error);
        }
        match bytes {
            Ok(bytes) => crate::interp::err(bytes),
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }
}

impl Vm {
    /// Blocking entry to the same resumable original Subst owner, for native
    /// primitive callbacks which cannot suspend across their C call boundary.
    #[cfg(test)]
    pub(crate) fn substitute_native_jim_original(
        &mut self,
        original: &crate::Value,
        flags: u8,
    ) -> Completion<crate::Value> {
        if !self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(tcl_syntax::native_string::NativeStringProtocol::is_jim084)
        {
            return self.refuse_host_command("original Jim substitution issuer".into());
        }
        let req = SubstReq {
            compiled: None,
            original: Some(original.native_lifetime_lease()),
            template: Vec::<u8>::new().into(),
            backslashes: flags & 4 == 0,
            commands: flags & 2 == 0,
            variables: flags & 1 == 0,
            control: if flags & 128 != 0 {
                crate::subst::SubstitutionControl::Command
            } else {
                crate::subst::SubstitutionControl::Word
            },
        };
        let placeholder = self.current_placeholder_unit();
        self.run_activation(Frame::new_subst(req, placeholder))
    }

    #[cfg(test)]
    pub(crate) fn expand_native_jim_dictionary_substitution(
        &mut self,
        original: &crate::Value,
    ) -> Result<crate::Value, Completion<crate::Value>> {
        let names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let operation =
            crate::interp::native_operation_currency::NativeOperationScope::current_entered(self)
                .map_err(|cause| {
                    crate::command::completion_from_cmd_error(
                        self,
                        tcl_cmd_core::CmdError::from_execution_refusal(cause),
                    )
                })?
                .ok_or_else(|| {
                    crate::command::completion_from_cmd_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim tuple entry",
                        )
                        .into(),
                    )
                })?;
        let prepared = self.original_jim_tuple_children_in_context(original, &names);
        if let Err(error) = &prepared {
            self.retain_original_jim_tuple_failure(error);
        }
        let reached = match prepared {
            Err(error) => Err(crate::command::completion_from_cmd_error(self, error)),
            Ok((name, index)) => {
                let completion = self.substitute_native_jim_original(index.value(), 0);
                if let Err(error) = names.current_for_value(self, original) {
                    drop(completion);
                    let _ = operation.check(self);
                    Err(crate::command::completion_from_cmd_error(self, error))
                } else if completion.code != Code::Ok {
                    Err(completion)
                } else {
                    let key = completion.result.into_native_reference();
                    self.read_native_jim_dictionary_member(name.value(), &key, &names)
                }
            }
        };
        // The dictionary worker and reached child/result retirement can
        // withdraw the original root even when they returned a Guest result.
        let after_read = names.current_for_value(self, original);
        let cleanup = names.finish(self);
        if let Err(first) = operation.check(self) {
            drop(reached);
            let _ = operation.check(self);
            return Err(crate::command::completion_from_cmd_error(self, first));
        }
        if let Err(first) = after_read {
            drop(reached);
            let _ = operation.check(self);
            return Err(crate::command::completion_from_cmd_error(self, first));
        }
        if let Err(first) = cleanup {
            drop(reached);
            let _ = operation.check(self);
            return Err(crate::command::completion_from_cmd_error(self, first));
        }
        reached
    }
}

#[cfg(test)]
#[path = "native_jim_script/original_tuple_consumers_tests.rs"]
mod original_tuple_consumers_tests;

#[cfg(test)]
mod evaluation_entry_tests {
    use super::*;
    use crate::Value;

    #[test]
    fn actual_script_missing_guard_precedes_frame_entry_and_success_enters() {
        let profile = crate::environment::profile_for_dialect("jim");
        for (source, entered) in [("set x \"", false), ("", false), ("set x VALUE", true)] {
            let mut vm = crate::native_fixture::core(profile);
            let original = Value::new_native_string_bytes(source.as_bytes());
            let unit = vm
                .prepare_native_jim_script_unit(
                    &original,
                    tcl_core_types::ByteNamespacePath::root(),
                )
                .unwrap();
            let mut frame = Frame::new(unit, false);
            assert!(!frame.jim_evaluation.evaluation_frame_entered);
            let tick = vm.tick_native_jim_script(&mut frame);
            assert_eq!(frame.jim_evaluation.evaluation_frame_entered, entered);
            if source == "set x \"" {
                assert!(matches!(tick, Tick::Return(completion) if completion.code == Code::Error));
                assert!(frame.jim_script.is_none());
            } else {
                assert!(frame.jim_script.is_some());
            }
        }
    }
}
