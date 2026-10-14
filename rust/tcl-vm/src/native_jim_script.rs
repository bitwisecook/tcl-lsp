// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    Code, Completion, Value,
    interp::{
        native_name_consumers::{OriginalNameConsumerScope, ParkedOriginalNameConsumerScope},
        native_operation_currency::NativeOperationScope,
    },
    value::{NativeJimScript, NativeJimScriptLease},
};
use std::{cell::RefCell, rc::Rc};
use tcl_syntax::jim_script_objects::{JimScriptCommand, JimScriptObjectKind};

#[derive(Clone)]
pub(crate) struct NativeJimScriptEntry {
    pub(crate) original: crate::NativeObjectLifetimeLease,
    pub(crate) substitution_flags: Option<u8>,
    // Cloned preparation transports share one pending native reference.
    // The actual activation takes it once; remaining views own no reference.
    activation_original: Rc<RefCell<Option<Value>>>,
}

impl NativeJimScriptEntry {
    pub(crate) fn borrowed(
        original: crate::NativeObjectLifetimeLease,
        substitution_flags: Option<u8>,
    ) -> Self {
        Self {
            original,
            substitution_flags,
            activation_original: Rc::default(),
        }
    }

    pub(crate) fn retain_activation_original(
        &self,
        original: Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        original.check_native_header()?;
        if self.substitution_flags.is_some()
            || !original.is_same_object(self.original.value())
            || self.activation_original.borrow().is_some()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "deferred Jim Script original owner",
            ));
        }
        *self.activation_original.borrow_mut() = Some(original.into_native_reference());
        Ok(())
    }
}

pub(crate) enum ScriptStep {
    Token { index: usize },
    Invoke(crate::NativeListItems),
    Complete(Completion<Value>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenContinuation {
    Ready,
    Awaiting,
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
    awaiting: TokenContinuation,
    invocation: bool,
    completion: Option<Completion<Value>>,
    last_options: Value,
    substitution_flags: Option<u8>,
    stopped: bool,
    skipped_first: bool,
    pending_dictionary: Option<PendingOriginalDictionary>,
}

struct PendingOriginalDictionary {
    original: crate::NativeObjectLifetimeLease,
    name: crate::NativeObjectLifetimeLease,
    context: ParkedOriginalNameConsumerScope,
}

impl PendingOriginalDictionary {
    fn accept(
        self,
        state: &mut NativeJimScriptState,
        vm: &mut crate::Vm,
        completion: Completion<Value>,
    ) {
        let Self {
            original,
            name,
            context,
        } = self;
        let cleanup = match NativeOperationScope::cleanup_saved(vm, context.currency()) {
            Ok(cleanup) => cleanup,
            Err(first) => {
                drop(completion);
                drop(context);
                drop(name);
                drop(original);
                state.completion = Some(crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::from_execution_refusal(first),
                ));
                return;
            }
        };
        let mut incoming = Some(completion);
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            let context = context.resume(vm)?;
            let reached = (|| {
                context.current_for_value(vm, original.value())?;
                context.current_for_value(vm, name.value())?;
                let completion = incoming
                    .take()
                    .expect("actual pending Dictionary completion");
                let completion = if completion.code == Code::Ok {
                    // The original Jim worker owns this substituted key during lookup.
                    context.current_for_value(vm, &completion.result)?;
                    let Completion {
                        result, options, ..
                    } = completion;
                    let key = result.into_native_reference();
                    let read = vm.read_native_jim_dictionary_member(name.value(), &key, &context);
                    drop(key);
                    drop(options);
                    context.current_for_value(vm, original.value())?;
                    match read {
                        Ok(value) => crate::interp::ok(value),
                        Err(completion) => completion,
                    }
                } else {
                    completion
                };
                state.accept_completion(vm, completion, Some((&context, original.value())))
            })();
            if let Err(error) = &reached {
                vm.retain_original_jim_tuple_failure(error);
            }
            let retired = context.finish(vm);
            cleanup.check(vm)?;
            retired?;
            reached
        })();
        if let Err(error) = &reached {
            vm.retain_original_jim_tuple_failure(error);
        }
        drop(incoming);
        drop(name);
        drop(original);
        let current = cleanup.check(vm);
        if let Err(first) = current.and(reached) {
            // All reached key/completion/context cleanup precedes this unwind.
            state.completion = Some(crate::command::completion_from_cmd_error(vm, first));
            let _ = cleanup.check(vm);
        }
    }
}

impl NativeJimScriptState {
    #[cfg(test)]
    pub(crate) fn new(
        entry: NativeJimScriptEntry,
        vm: &mut crate::Vm,
    ) -> Result<Self, crate::TclError> {
        Self::prepare(entry, vm, None)
    }

    pub(crate) fn new_for_evaluation_frame(
        entry: NativeJimScriptEntry,
        vm: &mut crate::Vm,
        location: &mut Option<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>>,
    ) -> Result<Self, crate::TclError> {
        Self::prepare(entry, vm, Some(location))
    }

    fn original_script_location(
        backing: &NativeJimScript,
    ) -> Result<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>, crate::TclError> {
        let script = backing.ordinary()?;
        Ok(tcl_runtime_api::jim_error_stack::JimScriptLocation {
            file: script.filename.native_lifetime_lease().into_value(),
            line: script.linenr.get().cast_unsigned(),
        })
    }

    pub(crate) fn enters_evaluation_frame(&self) -> bool {
        self.substitution_flags.is_none() && !self.backing.storage.is_empty()
    }

    pub(crate) fn current_script_location(
        &self,
    ) -> Result<Option<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>>, crate::TclError>
    {
        if self.substitution_flags.is_some() {
            return Ok(None);
        }
        Self::original_script_location(&self.backing).map(Some)
    }

    fn prepare(
        entry: NativeJimScriptEntry,
        vm: &mut crate::Vm,
        location: Option<&mut Option<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>>>,
    ) -> Result<Self, crate::TclError> {
        let context = vm.native_jim_object_context()?;
        let flags = entry.substitution_flags;
        // A deferred eval worker transfers its same existing owner before
        // JimGetScript. Borrowed preparation and Subst retain their own order.
        let activation_original = entry.activation_original.borrow_mut().take();
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
        if let Some(location) = location {
            *location = if flags.is_none() {
                Some(Self::original_script_location(&backing)?)
            } else {
                None
            };
        }
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
        let original = activation_original.unwrap_or_else(|| entry.original.value().clone());
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
            awaiting: TokenContinuation::Ready,
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
                self.awaiting = TokenContinuation::Awaiting;
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
                self.awaiting = TokenContinuation::Awaiting;
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

    pub(crate) fn begin_dictionary(
        &mut self,
        original: crate::NativeObjectLifetimeLease,
        name: crate::NativeObjectLifetimeLease,
        context: ParkedOriginalNameConsumerScope,
    ) {
        self.pending_dictionary = Some(PendingOriginalDictionary {
            original,
            name,
            context,
        });
    }

    pub(crate) fn accept(&mut self, vm: &mut crate::Vm, completion: Completion<Value>) {
        if let Some(pending) = self.pending_dictionary.take() {
            pending.accept(self, vm, completion);
        } else {
            // Independent existing Script consumers preserve their old None route.
            let _ = self.accept_completion(vm, completion, None);
        }
    }

    fn check_dictionary_continuation(
        vm: &mut crate::Vm,
        original: Option<(&OriginalNameConsumerScope, &Value)>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some((context, root)) = original {
            context.current_for_value(vm, root)?;
        }
        Ok(())
    }

    fn accept_completion(
        &mut self,
        vm: &mut crate::Vm,
        completion: Completion<Value>,
        original: Option<(&OriginalNameConsumerScope, &Value)>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        Self::check_dictionary_continuation(vm, original)?;
        if self.invocation {
            self.invocation = false;
            if completion.code == Code::Ok {
                self.last_options = completion.options;
            } else {
                self.completion = Some(completion);
            }
            return Self::check_dictionary_continuation(vm, original);
        }
        assert!(
            self.awaiting == TokenContinuation::Awaiting,
            "actual Script token continuation"
        );
        self.awaiting = TokenContinuation::Ready;
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
                if let Some((context, root)) = original {
                    let reached = context.read(vm, &value);
                    if let Err(error) = &reached {
                        vm.retain_original_jim_tuple_failure(error);
                    }
                    context.current_for_value(vm, root)?;
                    if let Err(error) = reached {
                        self.completion =
                            Some(crate::command::completion_from_cmd_error(vm, error));
                        return Self::check_dictionary_continuation(vm, original);
                    }
                } else if let Err(error) = native_script_bytes(&value) {
                    self.completion = Some(crate::command::completion_from_tcl_error(vm, error));
                    return Ok(());
                }
            }
            Self::check_dictionary_continuation(vm, original)?;
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
        Self::check_dictionary_continuation(vm, original)
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

#[cfg(test)]
mod original_source_location_tests {
    use super::*;
    use crate::value::NativeJimSourceInfo;

    fn provided(source: &[u8]) -> (crate::Vm, Value, Value) {
        let mut vm = crate::Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let context = vm.native_jim_object_context().unwrap();
        let filename = Value::new_native_string_bytes(b"ACTUAL".as_slice());
        let original = Value::new_native_string_bytes(source);
        original
            .install_native_jim_source(
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
        (vm, original, filename)
    }

    #[test]
    fn evaluation_location_borrows_the_original_filename_and_actual_line() {
        let (mut vm, original, filename) = provided(b"\nset x ONE\nset y TWO");
        let entry = NativeJimScriptEntry::borrowed(original.native_lifetime_lease(), None);
        let mut location = None;
        let mut state =
            NativeJimScriptState::new_for_evaluation_frame(entry, &mut vm, &mut location).unwrap();
        assert!(location.as_ref().unwrap().file.is_same_object(&filename));
        let references = filename.native_object_reference_count();
        assert!(matches!(
            state.next(&mut vm).unwrap(),
            ScriptStep::Token { .. }
        ));
        let reached = state.current_script_location().unwrap().unwrap();
        assert!(reached.file.is_same_object(&filename));
        assert_eq!(reached.line, 8);
        assert_eq!(filename.native_object_reference_count(), references);
    }

    #[test]
    fn missing_script_retains_actual_location_and_subst_has_no_script_layout() {
        let (mut vm, original, filename) = provided(b"\nset x \"");
        let entry = NativeJimScriptEntry::borrowed(original.native_lifetime_lease(), None);
        let mut location = None;
        assert!(
            NativeJimScriptState::new_for_evaluation_frame(entry, &mut vm, &mut location,).is_err()
        );
        let reached = location.as_ref().unwrap();
        assert!(reached.file.is_same_object(&filename));
        assert_eq!(reached.line, 8);

        let substituted = Value::new_native_string_bytes(b"$x".as_slice());
        let entry = NativeJimScriptEntry::borrowed(substituted.native_lifetime_lease(), Some(0));
        let state =
            NativeJimScriptState::new_for_evaluation_frame(entry, &mut vm, &mut location).unwrap();
        assert!(location.is_none());
        assert!(state.current_script_location().unwrap().is_none());
    }
}
