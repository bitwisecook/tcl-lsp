// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute the original retained Jim Script token objects.

use super::{CmdFrame, Code, Interp};
use crate::obj::Owned;
use tcl_syntax::{
    jim_script_objects::{missing_message, JimScriptObjectKind},
    native_string::NativeStringProtocol,
    value::ValueError,
};

impl Interp {
    pub(super) fn eval_native_jim_script(&mut self, original: Owned, frame: CmdFrame) -> Code {
        let context = match self.native_jim_object_context() {
            Ok(context) => context,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let backing =
            match crate::native_script::prepare(original.as_ptr(), &context, self.lexer_config()) {
                Ok(backing) => backing,
                Err(error) => return self.report_cmd_error(error.into()),
            };
        let script = match backing.ordinary() {
            Ok(script) => script,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if let Some(message) = missing_message(script.objects.missing) {
            return self.error(message);
        }
        self.set_result(context.empty_object().as_ptr());
        if script.objects.tokens().is_empty() {
            return Code::Ok;
        }
        let _lease = crate::native_script::activate(original, &backing);
        self.cmd_frames.borrow_mut().push(frame);
        let mut cursor = 0;
        let code = 'script: loop {
            let command = match script.objects.command_at(cursor) {
                Ok(Some(command)) => command,
                Ok(None) => break Code::Ok,
                Err(error) => break self.report_cmd_error(error.into()),
            };
            let line_index = cursor;
            cursor = command.next;
            let Some((argc, line)) = crate::native_script::script_line(
                script.objects.tokens()[line_index].value.as_ptr(),
            ) else {
                break self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("Jim Script original LINE primary")
                        .into(),
                );
            };
            if usize::try_from(argc).ok() != Some(command.words.len())
                || line
                    != script
                        .baseline
                        .wrapping_add(i32::from_ne_bytes(command.line_delta.to_ne_bytes()))
            {
                break self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("Jim Script original LINE payload")
                        .into(),
                );
            }
            script.linenr.set(line);
            if let Some(frame) = self.cmd_frames.borrow_mut().last_mut() {
                frame.line = u32::from_ne_bytes(line.to_ne_bytes());
            }
            let mut argv = Vec::<Owned>::with_capacity(command.words.len());
            for word in command.words {
                let mut parts = Vec::with_capacity(word.tokens.len());
                for token in &script.objects.tokens()[word.tokens.clone()] {
                    let value = match self.eval_native_jim_token(token.kind, &token.value) {
                        Ok(value) => value,
                        Err(Code::Return) if word.tokens.len() > 1 => {
                            Owned::retain(self.result_obj())
                        }
                        Err(code @ (Code::Break | Code::Continue)) if word.tokens.len() > 1 => {
                            let action = if code == Code::Break {
                                "break"
                            } else {
                                "continue"
                            };
                            break 'script self.error(
                                format!("invoked \"{action}\" outside of a loop").as_bytes(),
                            );
                        }
                        Err(code) => break 'script code,
                    };
                    if word.tokens.len() > 1 {
                        if let Err(error) = crate::dict::native_object_bytes(
                            value.as_ptr(),
                            NativeStringProtocol::Jim084,
                        ) {
                            break 'script self.report_cmd_error(error.into());
                        }
                    }
                    parts.push(value);
                }
                let value = if parts.len() == 1 {
                    parts.pop().expect("single original token")
                } else {
                    match self.interpolate_native_jim_values(&backing, word.tokens.clone(), parts) {
                        Ok(value) => value,
                        Err(code) => break 'script code,
                    }
                };
                if word.expand {
                    match crate::list::list_elements_native_checked(
                        value.as_ptr(),
                        NativeStringProtocol::Jim084,
                    ) {
                        Ok(items) => argv.extend(items.into_iter().map(Owned::retain)),
                        Err(error) => break 'script self.report_cmd_error(error.into()),
                    }
                } else {
                    argv.push(value);
                }
            }
            if !argv.is_empty() {
                let pointers: Vec<_> = argv.iter().map(Owned::as_ptr).collect();
                let code = self.dispatch(&pointers);
                if code != Code::Ok {
                    break code;
                }
            }
        };
        self.cmd_frames.borrow_mut().pop();
        code
    }

    pub(super) fn eval_native_jim_token(
        &mut self,
        kind: JimScriptObjectKind,
        original: &Owned,
    ) -> Result<Owned, Code> {
        match kind {
            JimScriptObjectKind::Source(
                tcl_lexer::JimScriptTokenKind::String | tcl_lexer::JimScriptTokenKind::Escaped,
            ) => Ok(original.clone()),
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Variable) => self
                .read_original_named_variable(original.as_ptr())
                .map(Owned::retain),
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Command) => {
                let code = self.eval_body_obj(original.as_ptr());
                if code == Code::Ok {
                    Ok(Owned::retain(self.result_obj()))
                } else {
                    Err(code)
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => {
                #[cfg(have_tommath)]
                let code = crate::builtins::eval_expr_original(self, original.as_ptr());
                #[cfg(not(have_tommath))]
                let code = self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
                );
                if code == Code::Ok {
                    Ok(Owned::retain(self.result_obj()))
                } else {
                    Err(code)
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::IndexedVariable) => self
                .expand_native_jim_dictionary_substitution(original.as_ptr())
                .map(Owned::retain),
            _ => Err(self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("native Script real token purpose").into(),
            )),
        }
    }
}
