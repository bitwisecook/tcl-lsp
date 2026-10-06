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
    pub(super) fn tick_native_jim_script(&mut self, frame: &mut Frame) -> Tick {
        if let Some(entry) = frame.jim_script_entry.take() {
            match NativeJimScriptState::new(entry, self) {
                Ok(state) => frame.jim_script = Some(state),
                Err(error) => return Tick::Return(completion_from_tcl_error(self, error)),
            }
        }
        loop {
            let step = match frame
                .jim_script
                .as_mut()
                .expect("native Script state")
                .next(self)
            {
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
                    let (kind, original) = frame
                        .jim_script
                        .as_ref()
                        .expect("native Script state")
                        .original_token(index);
                    let original = original.native_lifetime_lease();
                    let completion = match kind {
                        JimScriptObjectKind::Source(
                            tcl_lexer::JimScriptTokenKind::String
                            | tcl_lexer::JimScriptTokenKind::Escaped,
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
                                    return Tick::PushScript {
                                        script: prepared
                                            .prefix
                                            .expect("native Script always owns an activation"),
                                        label: None,
                                        cleanup_proc: None,
                                        fatal_tail: None,
                                        namespace: ScriptNamespace::Inherit,
                                    };
                                }
                                Err(error) => completion_from_tcl_error(self, error),
                            }
                        }
                        JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => {
                            match self.prepare_expression_value(original.value()) {
                                Ok(node) => {
                                    return Tick::PushExpression {
                                        req: ExpressionReq {
                                            state: tcl_syntax::expr::ExprEvalState::new(node),
                                            awaiting_array: None,
                                            normalize: false,
                                            jim_objects: original
                                                .value()
                                                .native_jim_expression_objects(),
                                            restore_primary: original
                                                .value()
                                                .retain_expression_primary(),
                                        },
                                        placeholder: self.current_placeholder_unit(),
                                    };
                                }
                                Err(error) => completion_from_tcl_error(self, error),
                            }
                        }
                        JimScriptObjectKind::Source(
                            tcl_lexer::JimScriptTokenKind::IndexedVariable,
                        ) => {
                            let context = match self.native_jim_object_context() {
                                Ok(context) => context,
                                Err(error) => {
                                    return Tick::Return(completion_from_tcl_error(
                                        self,
                                        error.into(),
                                    ));
                                }
                            };
                            if let Err(error) = original
                                .value()
                                .ensure_native_jim_dictionary_substitution(&context)
                            {
                                return Tick::Return(completion_from_tcl_error(self, error.into()));
                            }
                            let (name, key) = original
                                .value()
                                .with_native_jim_dictionary_substitution(|name, key| {
                                    (name.native_lifetime_lease(), key.native_lifetime_lease())
                                })
                                .expect("prepared original dictionary substitution");
                            frame
                                .jim_script
                                .as_mut()
                                .expect("native Script state")
                                .begin_dictionary(name);
                            return Tick::PushSubst {
                                req: SubstReq {
                                    original: Some(key),
                                    template: Vec::<u8>::new().into(),
                                    backslashes: true,
                                    commands: true,
                                    variables: true,
                                    control: crate::subst::SubstitutionControl::Word,
                                },
                                placeholder: self.current_placeholder_unit(),
                            };
                        }
                        _ => Completion::new(
                            Code::Error,
                            self.native_jim_object_context()
                                .expect("active native context")
                                .result_object(),
                            crate::Value::empty(),
                        ),
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
}

impl Vm {
    pub(crate) fn read_native_jim_dictionary_member(
        &mut self,
        name: &crate::Value,
        key: &crate::Value,
    ) -> Result<crate::Value, Completion<crate::Value>> {
        let root = self.read_original_named_variable(name)?;
        let root_lease = root.native_lifetime_lease();
        drop(root);
        let root = root_lease.value();
        let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
        if let Err(error) = self.native_object_dict_pairs_in(root, protocol) {
            if error.native_access_refusal().is_some() {
                return Err(completion_from_tcl_error(self, error.into()));
            }
            return Err(self.native_jim_dictionary_read_error(name, key, true));
        }
        let key_bytes = match crate::native_jim_script::native_script_bytes(key) {
            Ok(bytes) => bytes,
            Err(error) => return Err(completion_from_tcl_error(self, error)),
        };
        let found = root
            .with_cached_dictionary_representation(|pairs, _| {
                for (candidate, value) in pairs {
                    if crate::native_jim_script::native_script_bytes(candidate)?.as_ref()
                        == key_bytes.as_ref()
                    {
                        return Ok(Some(value.clone()));
                    }
                }
                Ok(None)
            })
            .expect("prepared dictionary");
        match found {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(self.native_jim_dictionary_read_error(name, key, false)),
            Err(error) => Err(completion_from_tcl_error(self, error)),
        }
    }
    fn native_jim_dictionary_read_error(
        &mut self,
        name: &crate::Value,
        key: &crate::Value,
        malformed: bool,
    ) -> Completion<crate::Value> {
        let bytes = (|| -> Result<Vec<u8>, crate::TclError> {
            let name = crate::native_jim_script::native_script_bytes(name)?;
            let key = crate::native_jim_script::native_script_bytes(key)?;
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
        match bytes {
            Ok(bytes) => crate::interp::err(bytes),
            Err(error) => completion_from_tcl_error(self, error),
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
            .is_some_and(|protocol| protocol.is_jim084())
        {
            return self.refuse_host_command("original Jim substitution issuer".into());
        }
        let req = SubstReq {
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
        let context = self
            .native_jim_object_context()
            .map_err(|error| completion_from_tcl_error(self, error.into()))?;
        original
            .ensure_native_jim_dictionary_substitution(&context)
            .map_err(|error| completion_from_tcl_error(self, error.into()))?;
        let (name, index) = original
            .with_native_jim_dictionary_substitution(|name, index| {
                (name.native_lifetime_lease(), index.native_lifetime_lease())
            })
            .expect("prepared dictionary substitution");
        let completion = self.substitute_native_jim_original(index.value(), 0);
        if completion.code != Code::Ok {
            return Err(completion);
        }
        let key = completion.result.into_native_reference();
        self.read_native_jim_dictionary_member(name.value(), &key)
    }
}
