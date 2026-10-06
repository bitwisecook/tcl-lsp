// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    Code, Completion, Value,
    value::{NativeJimScript, NativeJimScriptLease},
};
use std::rc::Rc;
use tcl_syntax::jim_script_objects::{JimScriptCommand, JimScriptObjectKind};

#[derive(Clone)]
pub(crate) struct NativeJimScriptEntry {
    pub(crate) original: crate::NativeObjectLifetimeLease,
    pub(crate) substitution_flags: Option<u8>,
}

pub(crate) enum ScriptStep {
    Token { index: usize },
    Invoke(crate::NativeListItems),
    Complete(Completion<Value>),
}

pub(crate) struct NativeJimScriptState {
    backing: Rc<NativeJimScript>,
    lease: Option<NativeJimScriptLease>,
    empty_parent: Option<Value>,
    cursor: usize,
    command: Option<JimScriptCommand>,
    word: usize,
    token: usize,
    parts: Vec<Value>,
    argv: Vec<Value>,
    awaiting: bool,
    invocation: bool,
    completion: Option<Completion<Value>>,
    last_options: Value,
    substitution_flags: Option<u8>,
    stopped: bool,
    skipped_first: bool,
    pending_dictionary: Option<crate::NativeObjectLifetimeLease>,
}

impl NativeJimScriptState {
    pub(crate) fn new(
        entry: NativeJimScriptEntry,
        vm: &mut crate::Vm,
    ) -> Result<Self, crate::TclError> {
        let context = vm.native_jim_object_context()?;
        let flags = entry.substitution_flags;
        let backing = match flags {
            Some(flags) => entry.original.value().prepare_native_jim_substitution(
                &context,
                vm.lexer_config(),
                flags,
            )?,
            None => entry
                .original
                .value()
                .prepare_native_jim_script(&context, vm.lexer_config())?,
        };
        if flags.is_none() {
            if let Some(message) =
                tcl_syntax::jim_script_objects::missing_message(backing.ordinary()?.objects.missing)
            {
                context.publish_result(&Value::new_native_string_bytes(message));
                return Err(crate::TclError::new(message));
            }
            context.publish_result(&context.empty_object());
        }
        // JimGetSubst precedes the genuine parent evaluation reference.
        let original = entry.original.value().clone();
        drop(entry);
        let empty = backing.storage.is_empty();
        let (lease, empty_parent) = if flags.is_some() {
            (
                Some(original.activate_native_jim_substitution(&backing)),
                None,
            )
        } else if empty {
            (None, Some(original))
        } else {
            (Some(original.activate_native_jim_script(&backing)), None)
        };
        Ok(Self {
            backing,
            lease,
            empty_parent,
            cursor: 0,
            command: None,
            word: 0,
            token: 0,
            parts: Vec::new(),
            argv: Vec::new(),
            awaiting: false,
            invocation: false,
            completion: None,
            last_options: Value::empty(),
            substitution_flags: flags,
            stopped: false,
            skipped_first: false,
            pending_dictionary: None,
        })
    }

    pub(crate) fn next(&mut self, vm: &mut crate::Vm) -> Result<ScriptStep, crate::TclError> {
        if let Some(completion) = self.completion.take() {
            return Ok(ScriptStep::Complete(completion));
        }
        if self.substitution_flags.is_some() {
            if self.token < self.backing.storage.len() && !self.stopped {
                self.awaiting = true;
                return Ok(ScriptStep::Token { index: self.token });
            }
            let end = self.token;
            let value = self.finish_interpolation(vm, 0..end, true)?;
            return Ok(ScriptStep::Complete(crate::interp::ok(value)));
        }
        if self.invocation {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim Script invocation continuation",
            )
            .into());
        }
        loop {
            if self.command.is_none() {
                self.command = self.backing.ordinary()?.objects.command_at(self.cursor)?;
                let Some(command) = &self.command else {
                    return Ok(ScriptStep::Complete(Completion::new(
                        Code::Ok,
                        vm.native_jim_object_context()?.result_object(),
                        std::mem::replace(&mut self.last_options, Value::empty()),
                    )));
                };
                let line = self.backing.ordinary()?.objects.tokens()[self.cursor]
                    .value
                    .native_jim_script_line()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim Script actual LINE primary",
                    ))?;
                if line.1
                    != self
                        .backing
                        .ordinary()?
                        .baseline
                        .wrapping_add(command.line_delta.cast_signed())
                {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim Script original LINE payload",
                    )
                    .into());
                }
                self.backing.ordinary()?.linenr.set(line.1);
                self.cursor = command.next;
                self.word = 0;
                self.token = command.words.first().map_or(0, |word| word.tokens.start);
            }
            let command = self.command.as_ref().expect("current command");
            if self.word == command.words.len() {
                self.command = None;
                self.invocation = true;
                return Ok(ScriptStep::Invoke(crate::NativeListItems::invocation_view(
                    Rc::new(std::mem::take(&mut self.argv)),
                )));
            }
            let word = &command.words[self.word];
            if self.token < word.tokens.end {
                self.awaiting = true;
                return Ok(ScriptStep::Token { index: self.token });
            }
            let tokens = word.tokens.clone();
            let expand = word.expand;
            let value = self
                .finish_interpolation(vm, tokens, false)?
                .into_native_reference();
            if expand {
                let items = vm.native_object_list_elements_in(
                    &value,
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                )?;
                self.argv.extend(items.iter().cloned());
            } else {
                self.argv.push(value);
            }
            self.word += 1;
            self.token = self
                .command
                .as_ref()
                .expect("current command")
                .words
                .get(self.word)
                .map_or(0, |word| word.tokens.start);
        }
    }

    fn finish_interpolation(
        &mut self,
        vm: &mut crate::Vm,
        tokens: std::ops::Range<usize>,
        always_interpolate: bool,
    ) -> Result<Value, crate::TclError> {
        if self.parts.len() == 1 && (!always_interpolate || tokens.len() == 1) {
            let original = self
                .parts
                .pop()
                .expect("single original interpolation result");
            if always_interpolate {
                return Ok(original.into_native_unowned_lifetime());
            }
            return Ok(original);
        }
        let mut bytes = Vec::new();
        for original in &self.parts {
            bytes.extend_from_slice(&native_script_bytes(original)?);
        }
        let value = Value::new_native_string_bytes(bytes.as_slice());
        let context = vm.native_jim_object_context()?;
        value.bind_native_jim_context(&context)?;
        let optimized = tokens.len() == 4
            && self.parts.len() == 4
            && self
                .backing
                .storage
                .interpolation_token(tokens.start)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Escaped))
            && self
                .backing
                .storage
                .interpolation_token(tokens.start + 1)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Escaped))
            && self
                .backing
                .storage
                .interpolation_token(tokens.start + 2)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Variable));
        if optimized {
            let (_, original_name) = self
                .backing
                .storage
                .interpolation_token(tokens.start)
                .expect("original first token");
            value.install_native_jim_interpolated(original_name, &self.parts[2], &context)?;
        } else if let Some(first) = self
            .parts
            .first()
            .filter(|first| !self.skipped_first && first.native_jim_source_cache_present())
        {
            value
                .install_native_jim_source(first.pin_native_jim_source_info(&context)?, &context)?;
        }
        self.parts.clear();
        if always_interpolate {
            Ok(value.into_native_unowned_lifetime())
        } else {
            Ok(value)
        }
    }

    pub(crate) fn begin_dictionary(&mut self, name: crate::NativeObjectLifetimeLease) {
        self.pending_dictionary = Some(name);
    }

    pub(crate) fn accept(&mut self, vm: &mut crate::Vm, mut completion: Completion<Value>) {
        if let Some(name) = self.pending_dictionary.take() {
            if completion.code == Code::Ok {
                // JimExpandDictSugar owns the actual substituted key during lookup.
                let key = completion.result.into_native_reference();
                completion = match vm.read_native_jim_dictionary_member(name.value(), &key) {
                    Ok(value) => crate::interp::ok(value),
                    Err(completion) => completion,
                };
            }
        }
        if self.invocation {
            self.invocation = false;
            if completion.code == Code::Ok {
                self.last_options = completion.options;
            } else {
                self.completion = Some(completion);
            }
            return;
        }
        assert!(self.awaiting, "actual Script token continuation");
        self.awaiting = false;
        let width = if self.substitution_flags.is_some() {
            self.backing.storage.len()
        } else {
            self.command.as_ref().expect("current word").words[self.word]
                .tokens
                .len()
        };
        let interpolates = self.substitution_flags.is_some() || width > 1;
        if completion.code == Code::Ok || completion.code == Code::Return && interpolates {
            // intv acquires a real native reference BEFORE Jim_String.
            let value = completion.result.into_native_reference();
            if interpolates {
                if let Err(error) = native_script_bytes(&value) {
                    self.completion = Some(crate::command::completion_from_tcl_error(vm, error));
                    return;
                }
            }
            self.parts.push(value);
            self.token += 1;
        } else if self
            .substitution_flags
            .is_some_and(|flags| flags & 128 != 0)
            && matches!(completion.code, Code::Break | Code::Continue)
        {
            if completion.code == Code::Break {
                self.stopped = true;
            } else {
                self.skipped_first |= self.token == 0;
                self.token += 1;
            }
        } else {
            self.completion = Some(
                if interpolates && matches!(completion.code, Code::Break | Code::Continue) {
                    crate::interp::err(format!(
                        "invoked \"{}\" outside of a loop",
                        if completion.code == Code::Break {
                            "break"
                        } else {
                            "continue"
                        }
                    ))
                } else {
                    completion
                },
            );
        }
    }

    pub(crate) fn original_token(&self, index: usize) -> (JimScriptObjectKind, &Value) {
        if self.substitution_flags.is_some() {
            let (kind, original) = self
                .backing
                .storage
                .interpolation_token(index)
                .expect("actual substitution token");
            (
                kind.map_or(JimScriptObjectKind::Word(0), JimScriptObjectKind::Source),
                original,
            )
        } else {
            let token = &self
                .backing
                .ordinary()
                .expect("validated ordinary Script activation")
                .objects
                .tokens()[index];
            (token.kind, &token.value)
        }
    }
}
impl Drop for NativeJimScriptState {
    fn drop(&mut self) {
        self.argv.clear();
        self.parts.clear();
        drop(self.lease.take());
        drop(self.empty_parent.take());
    }
}

pub(crate) fn native_script_bytes(value: &Value) -> Result<Rc<[u8]>, crate::TclError> {
    value
        .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        .map_err(|error| {
            tcl_syntax::value::ValueError::NativeStringAccess(
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
            )
            .into()
        })
}
