// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute the original retained Jim Script token objects.

use super::{CmdFrame, Code, Interp};
use crate::obj::Owned;
use tcl_syntax::{
    jim_script_objects::{missing_message, JimScriptObjectKind, JimScriptObjects, JimScriptWord},
    native_string::NativeStringProtocol,
    value::ValueError,
};

impl Interp {
    /// Enter the fresh Source object selected by the sourced-byte API. Its
    /// original filename and line become the Script token owners' source info.
    pub(super) fn eval_native_jim_source_unpublished(
        &mut self,
        source: &[u8],
        filename: &[u8],
        frame: CmdFrame,
    ) -> Code {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let context = match self.native_jim_object_context() {
            Ok(context) => context,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let filename = Owned::fresh(crate::obj::new_string_bytes(filename));
        let original = Owned::fresh(crate::obj::new_string_bytes(source));
        if let Err(error) = crate::native_source::install_source(
            original.as_ptr(),
            crate::native_source::NativeJimSourceInfo { filename, line: 1 },
            &context,
        ) {
            return self.report_cmd_error(error.into());
        }
        if !self.codegen_activation_enter() {
            return Code::Error;
        }
        let code = self.eval_native_jim_script(original, frame);
        // The sourced API settles return and projects options before publishing
        // the outermost completion or processing background errors.
        self.codegen_activation_leave_unpublished();
        code
    }

    pub(super) fn eval_native_jim_script(&mut self, original: Owned, frame: CmdFrame) -> Code {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
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
            let code = self.error(message);
            self.capture_original_jim_parse_failure(script);
            return code;
        }
        self.set_result(context.empty_object().as_ptr());
        if script.objects.tokens().is_empty() {
            return Code::Ok;
        }
        let _lease = crate::native_script::activate(original, &backing);
        self.cmd_frames.borrow_mut().push(frame);
        self.reset_jim_error_capture();
        let mut cursor = 0;
        let code = loop {
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
            let _evaluation = self.enter_original_jim_evaluation(
                u32::from_ne_bytes(line.to_ne_bytes()),
                script.filename.as_ptr(),
            );
            if let Some(frame) = self.cmd_frames.borrow_mut().last_mut() {
                frame.line = u32::from_ne_bytes(line.to_ne_bytes());
            }
            let code = self.eval_native_jim_words(&backing, &script.objects, command.words);
            if code == Code::Error {
                // Capture while the actual Script frame and procedure level are
                // live. Failed lookup has no selected invocation argv.
                self.capture_jim_error_stack();
            }
            if code != Code::Ok {
                break code;
            }
        };
        self.cmd_frames.borrow_mut().pop();
        code
    }

    fn capture_original_jim_parse_failure(
        &self,
        script: &tcl_syntax::native_jim_substitution::JimOrdinaryScript<Owned>,
    ) {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        self.jim_error_stack.borrow_mut().capture_object(|| {
            let frames = self.materialized_jim_evaluation_frames();
            let empty = Owned::fresh(crate::obj::new_string_bytes(b""));
            let location = tcl_runtime_api::jim_error_stack::JimScriptLocation {
                file: script.filename.clone(),
                line: u32::from_ne_bytes(script.linenr.get().to_ne_bytes()),
            };
            tcl_runtime_api::jim_error_stack::capture_jim_error_frames(
                self.jim_procedure_level.get(),
                &frames,
                Some(&location),
                &empty,
            )
            .map(|frames| self.jim_trace_object(frames))
        });
    }

    fn eval_native_jim_words(
        &mut self,
        backing: &std::rc::Rc<crate::native_script::NativeJimScript>,
        objects: &JimScriptObjects<Owned>,
        words: Vec<JimScriptWord>,
    ) -> Code {
        let mut argv = Vec::<Owned>::with_capacity(words.len());
        for word in words {
            let mut parts = Vec::with_capacity(word.tokens.len());
            for token in &objects.tokens()[word.tokens.clone()] {
                let value = match self.eval_native_jim_token(token.kind, &token.value) {
                    Ok(value) => value,
                    Err(Code::Return) if word.tokens.len() > 1 => Owned::retain(self.result_obj()),
                    Err(code @ (Code::Break | Code::Continue)) if word.tokens.len() > 1 => {
                        let action = if code == Code::Break {
                            "break"
                        } else {
                            "continue"
                        };
                        return self
                            .error(format!("invoked \"{action}\" outside of a loop").as_bytes());
                    }
                    Err(code) => return code,
                };
                if word.tokens.len() > 1 {
                    if let Err(error) = crate::dict::native_object_bytes(
                        value.as_ptr(),
                        NativeStringProtocol::Jim084,
                    ) {
                        return self.report_cmd_error(error.into());
                    }
                }
                parts.push(value);
            }
            let value = if parts.len() == 1 {
                parts.pop().expect("single original token")
            } else {
                match self.interpolate_native_jim_values(backing, word.tokens.clone(), parts) {
                    Ok(value) => value,
                    Err(code) => return code,
                }
            };
            if word.expand {
                match crate::list::list_elements_native_checked(
                    value.as_ptr(),
                    NativeStringProtocol::Jim084,
                ) {
                    Ok(items) => argv.extend(items.into_iter().map(Owned::retain)),
                    Err(error) => return self.report_cmd_error(error.into()),
                }
            } else {
                argv.push(value);
            }
        }
        if argv.is_empty() {
            Code::Ok
        } else {
            let pointers: Vec<_> = argv.iter().map(Owned::as_ptr).collect();
            self.dispatch(&pointers)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interp::default_host, native_source};

    #[test]
    fn sourced_jim_body_tokens_keep_original_lines_and_unlocated_objects_stay_unlocated() {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        assert_eq!(
            interp.eval_sourced(
                b"# first\n# second\n\nproc retained {} {error BOOM}",
                b"source.jim"
            ),
            Code::Ok,
        );
        let context = interp.native_jim_object_context().unwrap();
        let procedure = interp.proc_def(b"retained").unwrap();
        let original = procedure.body.checked_ptr().unwrap();
        let info = native_source::pin_source_info(original, &context).unwrap();
        assert_eq!(info.line, 4);
        assert_eq!(crate::obj::bytes_of(info.filename.as_ptr()), b"source.jim");
        drop(info);
        drop(procedure);

        let unlocated = Owned::fresh(crate::obj::new_string_bytes(b"error BOOM"));
        let head = Owned::fresh(crate::obj::new_string_bytes(b"proc"));
        let name = Owned::fresh(crate::obj::new_string_bytes(b"unlocated"));
        let parameters = Owned::fresh(crate::obj::new_string_bytes(b""));
        assert_eq!(
            interp.dispatch(&[
                head.as_ptr(),
                name.as_ptr(),
                parameters.as_ptr(),
                unlocated.as_ptr()
            ]),
            Code::Ok
        );
        let procedure = interp.proc_def(b"unlocated").unwrap();
        let info = native_source::pin_source_info(procedure.body.checked_ptr().unwrap(), &context)
            .unwrap();
        assert_eq!(info.line, 1);
        assert_eq!(info.filename.as_ptr(), context.empty_object().as_ptr());
    }

    #[test]
    fn jim_control_body_preserves_original_source_in_error_frames() {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(crate::obj::new_string_bytes(b"original.jim"));
        let body = Owned::fresh(crate::obj::new_string_bytes(b"error BOOM"));
        native_source::install_source(
            body.as_ptr(),
            native_source::NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        assert_eq!(interp.eval_control_body(body.as_ptr()), Code::Error);
        assert!(!interp.host_refusal_pending());
        let trace = interp.jim_stacktrace_object();
        let fields = crate::list::list_elements_native_checked(
            trace.as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[1], filename.as_ptr());
        assert_eq!(crate::obj::bytes_of(fields[2]), b"7");
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
    }

    #[test]
    fn jim_original_lookup_error_keeps_live_procedure_and_script_frames() {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        assert_eq!(
            interp.eval_sourced(
                b"proc fail {} {missingCommand BOOM}\ncatch {fail} r o; dict get $o -errorinfo",
                b"stdin",
            ),
            Code::Ok,
        );
        assert_eq!(interp.result_bytes(), b"fail stdin 1 {} {} stdin 2 fail");
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
        assert_eq!(interp.jim_procedure_level.get(), 0);
    }

    #[test]
    fn normal_original_script_does_not_materialise_its_borrowed_filename() {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(crate::obj::new_wide_int_obj(37));
        let original = Owned::fresh(crate::obj::new_string_bytes(b"list OK"));
        native_source::install_source(
            original.as_ptr(),
            native_source::NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        assert!(!crate::obj::has_string_rep(filename.as_ptr()));
        assert_eq!(interp.eval_body_obj(original.as_ptr()), Code::Ok);
        assert!(!crate::obj::has_string_rep(filename.as_ptr()));
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
    }
}
