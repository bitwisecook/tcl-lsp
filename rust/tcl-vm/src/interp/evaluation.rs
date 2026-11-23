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

//! Evaluating source: the `eval_source*` and `eval_file` entry points, level-zero settlement, direct evaluation, and preparing a script value's commands.

use super::{
    NamespacePath, PreparedScript, Vm, err, namespace_context, native_operation_currency, ok,
};
use crate::command::err_with_code;
use crate::compiled::CompiledUnit;
use crate::error::TclError;
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::ModuleAsm;
use tcl_runtime_api::{Code, Completion, FatalTail, ScriptCompileTarget};

/// What [`Vm::eval_original_script_value`] runs, prepared out of line.
enum OriginalScriptRun {
    /// A pure list evaluated as one command's words.
    Words(crate::NativeListItems),
    /// A compiled script unit.
    Unit(Box<CompiledUnit>),
    /// Nothing to run: the completion stands.
    Finished(Completion<Value>),
}

impl Vm {
    /// [`Self::eval_source`], the script's own commands evaluated as
    /// `evaluation` says: `Direct` as C's `Tcl_EvalEx` evaluates them, one at a
    /// time without compiling them (the compiler then reads each command's
    /// literal words as the command does), `Compiled` as every body a command
    /// evaluates.
    pub(crate) fn eval_source_as(
        &mut self,
        src: &str,
        evaluation: tcl_runtime_api::CommandEvaluation,
    ) -> Result<Completion<Value>, TclError> {
        if evaluation == tcl_runtime_api::CommandEvaluation::Compiled {
            return self.eval_source(src);
        }
        self.host_execution_depth += 1;
        let result = self.eval_source_direct_internal(src);
        self.host_execution_depth -= 1;
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        result
    }

    /// [`Self::eval_source`] for the script file `tclsh` is given, whose own
    /// commands the emulated release evaluates as `Tcl_EvalEx` does, one at a
    /// time and without compiling them, where it compiles every body they
    /// evaluate and every script `source` reads
    /// ([`tcl_runtime_api::CommandEvaluation`]).
    ///
    /// # Errors
    /// A script that compiles to nothing that runs, or a host refusal.
    pub fn eval_source_directly(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        self.eval_source_as(src, tcl_runtime_api::CommandEvaluation::Direct)
    }

    pub(crate) fn eval_source_direct_internal(
        &mut self,
        src: &str,
    ) -> Result<Completion<Value>, TclError> {
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_native_jim_source_bytes(src.as_bytes(), None);
        }
        self.claim_number_grammar();
        self.reset_error_state_for_eval();
        // C parses one command immediately before evaluating it, so every
        // command ahead of a malformed one runs before the parse error.
        let (prefix, fatal_tail, prefix_commands) = match self.script_prefix_and_fatal_tail(src) {
            Ok(plan) => plan,
            Err(error) => {
                let namespace = self.current_ns().to_owned();
                return Err(self.compile_service_error(
                    error,
                    src,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                ));
            }
        };
        if prefix_commands == 0
            && let Some(tail) = fatal_tail
        {
            if !tail.command_text.is_empty() {
                self.log_command_info_only(&tail.command_text, &tail.message, tail.line);
            }
            return Err(TclError::new(tail.message));
        }
        let module = self.compile_direct_module(prefix)?;
        let mut comp = self.run_current_module_at(&module, None);
        if self.execution_refusal.is_some() {
            return Ok(comp);
        }
        if comp.code == Code::Ok
            && let Some(tail) = fatal_tail
        {
            comp = self.raise_fatal_tail(tail);
        }
        if comp.code == Code::Error {
            self.clear_error_logged();
        }
        Ok(comp)
    }

    /// Compile a script whose own commands the release evaluates directly. A
    /// release whose compiles fail whole runs such a script one command at a
    /// time, each command compiling its own scripts as it runs them: the
    /// plain dispatch of every command.
    fn compile_direct_module(&mut self, src: &str) -> Result<Rc<ModuleAsm>, TclError> {
        self.current_native_compilation_namespace()?;
        let namespace = self.current_ns().to_owned();
        let evaluation = tcl_runtime_api::CommandEvaluation::Direct;
        if self.requires_plain_command_dispatch()
            || self.runtime_version().compile_errors_fail_scripts()
        {
            return self
                .compile_plain_module_as(src, &namespace, evaluation)
                .map(Rc::new);
        }
        let entry = self.native_compilation_entry_for_namespace(&namespace, false)?;
        let Some(compiler) = self.compiler.clone() else {
            return Err(self.compile_service_error(
                tcl_runtime_api::CompileError::Unsupported(
                    "direct evaluation requires a CompileService".into(),
                ),
                src,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let compilation_outcome = compiler.compile_script_with_entry(
            ScriptCompileTarget {
                source: src,
                namespace: &namespace,
                evaluation,
            },
            self.source_profile(),
            &entry,
        );
        let module = compilation_outcome.map_err(|error| {
            self.compile_service_error(
                error,
                src,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            )
        })?;
        self.validate_module_profile(&module)?;
        Self::validate_module_namespace(&module, &namespace)?;
        if !self.function_command_bindings_match_with_manifest(
            &module.top_level,
            module.manifest.as_deref(),
        ) {
            return self
                .compile_plain_module_as(src, &namespace, evaluation)
                .map(Rc::new);
        }
        Ok(Rc::new(module))
    }

    /// Settle a completion that reached the outermost level, as C's level-0
    /// `Tcl_EvalEx` does for `tclsh` (`TclUpdateReturnInfo`, then
    /// `ProcessUnexpectedResult`): a `return` takes its `-code` once its
    /// `-level` runs out, `break` and `continue` become `invoked "break"
    /// outside of a loop` errors and any other code `command returned bad code:
    /// N`, under `TCL UNEXPECTED_RESULT_CODE N` from 8.6. An error left
    /// uncaught publishes `errorInfo` and `errorCode`, which the next command
    /// at the same prompt can read.
    pub fn settle_top_level(&mut self, completion: Completion<Value>) -> Completion<Value> {
        let completion = self.settle_level_zero(completion);
        if completion.code == Code::Error {
            let info = self
                .error_info_value()
                .map_or_else(|| completion.result.string_bytes().to_vec(), <[u8]>::to_vec);
            let code = self
                .completion_options_snapshot(&completion)
                .ok()
                .and_then(|options| {
                    crate::command::opt_get_checked(self, &options, b"-errorcode")
                        .ok()
                        .flatten()
                })
                .unwrap_or_else(|| Value::string("NONE"));
            self.publish_error(&info, &code);
        }
        completion
    }

    /// `TclEvalEx` at `numLevels == 0`: a `return` is settled
    /// (`TclUpdateReturnInfo`), and any other code but `ok` or `error` becomes
    /// an error (`ProcessUnexpectedResult`).
    pub(crate) fn settle_level_zero(
        &mut self,
        mut completion: Completion<Value>,
    ) -> Completion<Value> {
        let command = self.pending.top_level_command.take();
        // From 8.5 a `return` carrying `-errorinfo` logged its error as it
        // ran (`TclProcessReturn`), so the trace names no command here.
        let logged_by_return = completion.code == Code::Return
            && self.runtime_version().has_return_options_dictionary()
            && crate::command::opt_get_checked(self, &completion.options, b"-errorinfo")
                .ok()
                .flatten()
                .is_some_and(|info| !info.string_bytes().is_empty());
        if completion.code == Code::Return {
            self.update_return_info(&mut completion);
        }
        if completion.code == Code::Error && logged_by_return {
            self.mark_error_logged();
        }
        if !matches!(completion.code, Code::Ok | Code::Error) {
            let unexpected = completion.code;
            completion = self.unexpected_result(unexpected);
            // `ProcessUnexpectedResult` sets the code before the command is
            // logged, so logging finds it rather than defaulting to NONE.
            if self.runtime_version().has_unexpected_result_error_codes() {
                self.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
                    format!("TCL UNEXPECTED_RESULT_CODE {}", unexpected.as_int()).into_bytes(),
                ));
            }
            // A fresh error: its trace starts at the command that raised the
            // code (`while executing "break"`), or at its message alone.
            let _ = self.take_error_info();
            if command.is_none() {
                let message = completion.result.string_bytes();
                self.seed_error_info(message);
            }
        }
        // An error not yet logged logs the command the top level ran (C's
        // `TclEvalEx` at `numLevels == 0`).
        if completion.code == Code::Error
            && !self.command_error_is_logged()
            && let Some((text, line, enclosing)) = command
        {
            let message = completion.result.string_bytes();
            self.log_command_with_enclosing(
                &text,
                Value::from_native_string_bytes(text.clone()),
                &enclosing,
                &message,
                line,
            );
        }
        completion
    }

    /// The error C's `ProcessUnexpectedResult` makes of a completion code a
    /// context cannot take.
    pub(crate) fn unexpected_result(&mut self, code: Code) -> Completion<Value> {
        let message = match code {
            Code::Break => "invoked \"break\" outside of a loop".to_owned(),
            Code::Continue => "invoked \"continue\" outside of a loop".to_owned(),
            other => format!("command returned bad code: {}", other.as_int()),
        };
        if self.runtime_version().has_unexpected_result_error_codes() {
            err_with_code(
                message,
                format!("TCL UNEXPECTED_RESULT_CODE {}", code.as_int()),
            )
        } else {
            err(message)
        }
    }

    /// Evaluate a command `tclsh` reads from its input, with no command
    /// active: a completion leaving it is settled as C settles one at
    /// `numLevels == 0`.
    pub fn eval_interactive(&mut self, src: &str) -> Completion<Value> {
        let completion = match self.eval_source(src) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.settle_top_level(completion)
    }

    /// The commands `tclsh` reads from its stdin, as `tclvm` reads them: line
    /// by line until the lines read form complete commands, each such chunk
    /// evaluated by [`Self::eval_interactive`]. A chunk that does not complete
    /// normally publishes its error and hands its result to `report`, and the
    /// next runs, as `tclsh` prints the error and reads on; an `exit` ends the
    /// run, and an incomplete last chunk never runs.
    pub fn eval_source_commands(&mut self, src: &str, mut report: impl FnMut(&mut Self, &str)) {
        for chunk in stdin_chunks(src) {
            let completion = self.eval_interactive(chunk);
            if self.exit_pending() {
                return;
            }
            if !completion.code.is_ok() {
                // `settle_top_level` already published the error's
                // `errorInfo`/`errorCode` from the interpreter's trace.
                let message = completion.result.to_str().to_string();
                report(self, &message);
            }
        }
    }

    /// Run a script file's contents as C's `Tcl_FSEvalFileEx` does: a
    /// `return` reaching the file is settled as at a procedure boundary, and
    /// an error adds the `(file "…" line N)` frame. `top_level` runs the file
    /// `tclsh` is given, with no command active: its own commands are
    /// evaluated as [`Self::eval_source_directly`] does, and a completion
    /// leaving one is settled as C settles one at `numLevels == 0`.
    pub fn eval_file(&mut self, path: &str, src: &str, top_level: bool) -> Completion<Value> {
        let evaluation = if top_level
            || tcl_registry::release_compilation::evaluates_script_directly(
                self.runtime_version(),
                "source",
                1,
            ) {
            tcl_runtime_api::CommandEvaluation::Direct
        } else {
            tcl_runtime_api::CommandEvaluation::Compiled
        };
        self.push_script(path.to_owned());
        let mut c = match self.eval_source_as(src, evaluation) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.pop_script();
        // An error the `return` settles to names no file, as C's `else if`;
        // the file `tclsh` is given settles its completion at level 0 first.
        let returned = c.code == Code::Return;
        if top_level {
            c = self.settle_level_zero(c);
        } else if returned {
            self.update_return_info(&mut c);
        }
        // The file `tclsh` is given names itself even for an error its
        // `return` settled to (8.4.20/8.5.19 `return -code error …`,
        // `return -level 2 …` at file level).
        if c.code == Code::Error && (!returned || top_level) && !self.uses_jim_error_stack() {
            let text =
                tcl_cmd_core::error::file_frame_text(path.as_bytes(), self.runtime_version());
            let mut frame = b"\n    (".to_vec();
            frame.extend_from_slice(&text);
            frame.extend_from_slice(format!(" line {})", self.error_line()).as_bytes());
            let message = c.result.string_bytes();
            self.seed_error_info_frame(&*message, frame);
        }
        if top_level && c.code == Code::Error {
            c = self.settle_top_level(c);
        }
        c
    }

    /// Evaluate source in the current frame through the actual Jim Script owner
    /// or the independently installed C bytecode [`CompileService`].
    pub fn eval_source(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        self.host_execution_depth += 1;
        let result = self.eval_source_internal(src, None);
        self.host_execution_depth -= 1;
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        result
    }

    /// Compile and evaluate source with neutral provider refusal kept outside
    /// Tcl completion. A guest catch cannot bind its result or run a handler.
    ///
    /// # Errors
    /// Returns a host refusal after preserving effects before the reached domain.
    pub fn try_eval_source(
        &mut self,
        src: &str,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = match self.eval_source_internal(src, None) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    /// Evaluate a script value in the current frame, as C's `Tcl_EvalEx` does:
    /// its text reaches the parser untranslated, so a lone CR in a braced word
    /// stays a CR. [`Self::eval_source`] reads its text as a script file is
    /// read (`-translation auto`), as `tclsh FILE` and `source` do.
    pub fn eval_value_source(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        self.host_execution_depth += 1;
        let image = tcl_lexer::SourceImage::native(src.as_bytes().to_vec());
        let result = self.eval_source_image_internal(&image);
        self.host_execution_depth -= 1;
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        result
    }

    /// Evaluate original native source bytes in the current frame.
    /// Source bytes are neither decoded into Rust Unicode nor rewritten into
    /// escaped source. Native compilation obligations remain host refusals.
    ///
    /// # Errors
    /// Returns the reached native compiler/execution provider refusal.
    pub fn try_eval_source_bytes(
        &mut self,
        source: &[u8],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.try_eval_source_image(&tcl_lexer::SourceImage::native(source.to_vec()))
    }

    /// Evaluate an original image with its explicitly retained source channel.
    /// Valid commands before a malformed tail execute in native parser order;
    /// a failed prefix cannot donate execution of the later parse error.
    ///
    /// # Errors
    /// Returns an explicit host refusal before executing unavailable bytecode.
    pub fn try_eval_source_image(
        &mut self,
        source: &tcl_lexer::SourceImage,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = match self.eval_source_image_internal(source) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    fn eval_source_image_internal(
        &mut self,
        source: &tcl_lexer::SourceImage,
    ) -> Result<Completion<Value>, TclError> {
        self.eval_source_image_at_internal(source, None)
    }

    pub(crate) fn eval_source_image_at_internal(
        &mut self,
        source: &tcl_lexer::SourceImage,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Result<Completion<Value>, TclError> {
        self.eval_source_image_as_internal(
            source,
            location,
            tcl_runtime_api::CommandEvaluation::Compiled,
        )
    }

    pub(crate) fn eval_source_image_as_internal(
        &mut self,
        source: &tcl_lexer::SourceImage,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
        evaluation: tcl_runtime_api::CommandEvaluation,
    ) -> Result<Completion<Value>, TclError> {
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_native_jim_source_bytes(source.bytes(), location);
        }
        self.claim_number_grammar();
        self.reset_error_state_for_eval();
        let current = self.current_native_compilation_namespace()?;
        let namespace = self.ns_path(current);
        let direct = evaluation == tcl_runtime_api::CommandEvaluation::Direct;
        let mut entry = self.native_compilation_entry_for_namespace_token(current, false);
        entry.direct_evaluation = direct;
        let Some(compiler) = self.compiler.clone() else {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "eval requires an original-byte CompileService".into(),
                ),
                source,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let plan = compiler
            .script_command_plan_bytes_with_entry(source, self.source_profile(), &entry)
            .map_err(|error| {
                self.compile_service_error_bytes(
                    error,
                    source,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                )
            })?;
        if plan.complete_prefix_len > source.len() {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "byte CompileService returned an invalid command boundary".into(),
                ),
                source,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        }
        if self.native_invocation_dialect().script_parse_timing()
            == Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript)
            && let Some(tail) = plan.fatal_tail.clone()
        {
            let level = self
                .jim_errors
                .frames
                .last()
                .map_or(0, |frame| frame.procedure_level);
            return Ok(self.raise_jim_script_parse_failure(tail, level, location));
        }
        if plan.complete_prefix_commands == 0
            && let Some(tail) = plan.fatal_tail
        {
            if !tail.command_text.is_empty() {
                self.log_command_info_only(&tail.command_text, &tail.message, tail.line);
            }
            return Err(TclError::new(tail.message));
        }
        let prefix = tcl_lexer::SourceImage::from_bytes(
            source.bytes()[..plan.complete_prefix_len].to_vec(),
            source.channel(),
        );
        let plain_dispatch = self.requires_plain_command_dispatch()
            || (direct && self.runtime_version().compile_errors_fail_scripts());
        let module = self.compile_module_bytes_as(&prefix, &namespace, plain_dispatch, direct)?;
        let mut completion = self.run_current_module_at(&module, location);
        if self.execution_refusal.is_some() {
            return Ok(completion);
        }
        if completion.code == Code::Ok
            && let Some(tail) = plan.fatal_tail
        {
            completion = self.raise_fatal_tail(tail);
        }
        if completion.code == Code::Error {
            self.clear_error_logged();
        }
        Ok(completion)
    }

    /// Evaluate framework source under the host engine explicitly installed by
    /// [`Self::set_native_engine_profile`]. Source grammar remains unchanged;
    /// logical invocation policy is restored before this method returns.
    ///
    /// This capability is for embedding-host initialization. Availability or a
    /// vendor compatibility profile cannot supply its physical engine receipt.
    /// Compiled bodies and frozen activations retain the selected policy, so
    /// later user calls must validate or recompile under their own policy.
    ///
    /// # Errors
    /// Returns a host refusal when no actual engine was installed, or when a
    /// reached compilation/execution capability is unavailable.
    pub fn try_eval_native_host_source(
        &mut self,
        src: &str,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        let Some(profile) = self.actual_engine_profile else {
            let completion = self.refuse_host_command(
                "framework initialization requires an explicit native engine".into(),
            );
            return self.finish_host_execution(completion);
        };
        let saved = self.active_native_profile.replace(profile);
        let result = self.try_eval_source(src);
        self.active_native_profile = saved;
        result
    }

    /// Evaluate an actual source entry with its retained filename and base line.
    ///
    /// # Errors
    /// Returns compilation failure or a typed native provider refusal.
    pub fn try_eval_source_at(
        &mut self,
        src: &str,
        location: tcl_runtime_api::script_source_location::ScriptSourceLocation,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = match self.eval_source_internal(src, Some(location)) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    pub(crate) fn eval_source_internal(
        &mut self,
        src: &str,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Result<Completion<Value>, TclError> {
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_native_jim_source_bytes(src.as_bytes(), location);
        }
        self.claim_number_grammar();
        self.reset_error_state_for_eval();
        // C parses one command immediately before evaluating it, so every
        // command ahead of a malformed one runs before the parse error is
        // raised (#1603). Compile and run that prefix, then raise.
        let (prefix, fatal_tail, prefix_commands) = match self.script_prefix_and_fatal_tail(src) {
            Ok(plan) => plan,
            Err(error) => {
                let namespace = self.current_ns().to_owned();
                return Err(self.compile_service_error(
                    error,
                    src,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                ));
            }
        };
        if self.native_invocation_dialect().script_parse_timing()
            == Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript)
            && let Some(tail) = fatal_tail.clone()
        {
            let level = self
                .jim_errors
                .frames
                .last()
                .map_or(0, |frame| frame.procedure_level);
            return Ok(self.raise_jim_script_parse_failure(tail, level, location));
        }
        // No command parsed, so none ran and there is no prefix completion to
        // carry: report the error on the channel this entry point has always
        // used for a script it could not compile at all. Only a script that
        // *did* run something reports through the completion below.
        //
        // The test is the command count, not the prefix's length: when the
        // *first* command is the malformed one the prefix still spans any
        // leading whitespace and comments, so ` \n set x "` has a nonzero
        // prefix that runs nothing at all.
        if prefix_commands == 0
            && let Some(tail) = fatal_tail
        {
            // Nothing ran, but C still names the command that failed to parse.
            if !tail.command_text.is_empty() {
                self.log_command_info_only(&tail.command_text, &tail.message, tail.line);
            }
            return Err(TclError::new(tail.message));
        }
        let module = self.compile_cached(prefix)?;
        let mut comp = self.run_current_module_at(&module, location);
        if self.execution_refusal.is_some() {
            return Ok(comp);
        }
        // Only a prefix that ran to completion reaches the parse error: an
        // error (or any non-`ok` completion) in an earlier command is what C
        // reports, the malformed tail never being parsed at all.
        if comp.code == Code::Ok
            && let Some(tail) = fatal_tail
        {
            comp = self.raise_fatal_tail(tail);
        }
        // Crossing back out of a nested script is a frame boundary: clear
        // `ERR_ALREADY_LOGGED` so the enclosing command (the `eval`/`[subst]`/
        // proc call site) logs its own `invoked from within` frame.
        if comp.code == Code::Error {
            self.clear_error_logged();
        }
        Ok(comp)
    }

    /// Split `src` into the leading commands that parse and the parse error C
    /// raises once they have run.
    ///
    /// The compiler owns the dialect-aware split ([`ScriptCommandPlan`]); this
    /// is the non-compiling half of [`Self::prepare_script_commands_value`], for
    /// callers that drive a whole module rather than a deferred activation.
    /// An unavailable plan or invalid boundary is a typed compiler refusal
    /// before source effects. Without an attached service, the original whole
    /// source proceeds to the ordinary missing-compiler boundary.
    fn script_prefix_and_fatal_tail<'s>(
        &self,
        src: &'s str,
    ) -> Result<(&'s str, Option<FatalTail>, usize), tcl_runtime_api::CompileError> {
        let Some(compiler) = self.compiler.as_ref() else {
            return Ok((src, None, usize::from(!src.is_empty())));
        };
        let plan = compiler.script_command_plan_for_profile(src, self.source_profile())?;
        if plan.complete_prefix_len > src.len() || !src.is_char_boundary(plan.complete_prefix_len) {
            return Err(tcl_runtime_api::CompileError::Unsupported(
                "CompileService returned an invalid original script boundary".into(),
            ));
        }
        Ok((
            &src[..plan.complete_prefix_len],
            plan.fatal_tail,
            plan.complete_prefix_commands,
        ))
    }

    /// Compile a deferred/reusable script and bind its assembly to the dialect-
    /// profile generation, command-semantics epoch, and command-resolution
    /// namespace at which the cache lookup validated its assumptions. Consumers
    /// must carry this owner through to frame creation; using the VM's later
    /// ambient state there can acknowledge changes the assembly has never seen.
    pub(crate) fn compile_script_cached(&mut self, src: &str) -> Result<CompiledUnit, TclError> {
        self.current_native_compilation_namespace()?;
        let namespace = self.current_ns().to_owned();
        self.compile_script_cached_in_namespace(src, &namespace)
    }

    /// Compile a deferred script in an explicit constructed namespace, carrying
    /// that provenance into the returned activation owner.
    pub(crate) fn compile_script_cached_in_namespace(
        &mut self,
        src: &str,
        namespace: &str,
    ) -> Result<CompiledUnit, TclError> {
        self.reset_error_state_for_eval();
        let module = self.compile_cached_in_namespace(src, namespace)?;
        self.validate_module_profile(&module)?;
        Self::validate_module_namespace(&module, namespace)?;
        self.merge_procs(&module);
        Ok(self
            .compiled_unit(
                Rc::new(module.top_level.clone()),
                module.source_namespace.clone(),
            )
            .with_manifest(module.manifest.clone()))
    }

    /// Public original Jim source is parsed by the actual Script object owner,
    /// independently of the C bytecode compile service and loaded Tcl helpers.
    pub(crate) fn eval_native_jim_source_bytes(
        &mut self,
        source: &[u8],
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Result<Completion<Value>, TclError> {
        let _scope = native_operation_currency::NativeOperationScope::enter_native(self)
            .map_err(TclError::from_execution_failure)?;
        let context = self.native_jim_object_context()?;
        let original = Value::new_native_string_bytes(source);
        if let Some(location) = &location {
            original.install_native_jim_source(
                crate::value::NativeJimSourceInfo {
                    filename: Value::new_native_string_bytes(location.file.as_bytes()),
                    line: location.line.cast_signed(),
                },
                &context,
            )?;
        }
        self.eval_original_script_value(
            &original,
            tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
            location,
        )
    }

    /// Materialize original script storage through the physical string issuer.
    /// Script word, expression and variable providers do not select this door.
    pub(crate) fn native_script_source_image(
        &self,
        value: &Value,
    ) -> Result<tcl_runtime_api::SourceImage, TclError> {
        // Materialize the original physical object independently of word
        // decoding, variable naming and expression parser providers.
        let protocol = self.materialization_protocol(value).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native script object string materialization",
            ),
        )?;
        let bytes = value.native_string_bytes(protocol).map_err(|error| {
            tcl_syntax::value::ValueError::NativeStringAccess(
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
            )
        })?;
        Ok(tcl_runtime_api::SourceImage::native(bytes.as_ref()))
    }

    pub(crate) fn prepare_native_jim_script_unit(
        &mut self,
        original: &Value,
        context: namespace_context::NamespaceExecutionContext,
    ) -> Result<CompiledUnit, TclError> {
        context
            .check_owner(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        original.bind_native_jim_context(&self.native_jim_object_context()?)?;
        Ok(CompiledUnit::native_script(
            crate::native_jim_script::NativeJimScriptEntry::borrowed(
                original.native_lifetime_lease(),
                None,
            ),
            context,
            self.profile_generation,
            self.native_interpreter_identity(),
        ))
    }

    /// Evaluate an original object with its selected native source purpose.
    pub(crate) fn eval_original_script_value(
        &mut self,
        value: &Value,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Result<Completion<Value>, TclError> {
        // Prepared out of line, so preparation's locals are off the native
        // stack while the script runs: a trace callback or `catch` body can
        // recurse back through here.
        let completion = match self.prepare_original_script_run(value, purpose, location)? {
            OriginalScriptRun::Words(words) => {
                return Ok(self.run_original_object_body(value.clone(), words));
            }
            OriginalScriptRun::Unit(unit) => self.run_compiled_unit(*unit),
            OriginalScriptRun::Finished(completion) => completion,
        };
        if completion.code == Code::Error {
            self.clear_error_logged();
        }
        Ok(completion)
    }

    /// What [`Self::eval_original_script_value`] runs for `value`.
    #[inline(never)]
    fn prepare_original_script_run(
        &mut self,
        value: &Value,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Result<OriginalScriptRun, TclError> {
        if let Some(refusal) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        self.claim_number_grammar();
        self.reset_error_state_for_eval();
        let direct = crate::command::original_script_list(self, value, purpose)
            .map_err(TclError::from_completion)?;
        if let Some(words) = direct {
            return Ok(OriginalScriptRun::Words(words));
        }
        let prepared = self.prepare_script_commands_value_for(value, purpose)?;
        Ok(match prepared.prefix {
            Some(unit) => OriginalScriptRun::Unit(Box::new(
                unit.with_fatal_tail(prepared.fatal_tail)
                    .with_source_location(location),
            )),
            None => OriginalScriptRun::Finished(prepared.fatal_tail.map_or_else(
                || ok(Value::empty()),
                |tail| self.raise_script_parse_failure(tail, location),
            )),
        })
    }

    /// Prepare an original script through its native Script or C bytecode owner.
    /// Preserve each owner's source and execution namespace requirements.
    pub(crate) fn prepare_script_commands_value(
        &mut self,
        value: &Value,
    ) -> Result<PreparedScript, TclError> {
        self.prepare_script_commands_value_for(
            value,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
        )
    }

    /// Prepare original source under the actual caller's evaluation purpose.
    fn native_script_command_plan(
        &mut self,
        source: &tcl_runtime_api::SourceImage,
        namespace: &NamespacePath,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, TclError> {
        let entry = self.native_compilation_entry_for_namespace_token(
            self.current_native_compilation_namespace()?,
            false,
        );
        let Some(compiler) = self.compiler.as_ref() else {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "native script preparation requires a byte CompileService".into(),
                ),
                source,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let plan = match compiler.script_command_plan_bytes_with_entry(
            source,
            self.source_profile(),
            &entry,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return Err(self.compile_service_error_bytes(
                    error,
                    source,
                    namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                ));
            }
        };
        if plan.complete_prefix_len > source.len() {
            let message = "CompileService returned an invalid native byte command boundary";
            let _ = self.refuse_host_command(message.to_owned());
            return Err(TclError::new(message));
        }
        Ok(plan)
    }

    fn direct_script_prefix(
        &mut self,
        admitted_source: &tcl_runtime_api::SourceImage,
        namespace: &NamespacePath,
        strings: tcl_syntax::native_string::NativeStringProtocol,
        evaluation: tcl_registry::native_eval_object::NativeEvalObjectProtocol,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
    ) -> Result<CompiledUnit, TclError> {
        let module = self.compile_module_bytes_as(admitted_source, namespace, true, true)?;
        self.merge_procs(&module);
        let asm = Rc::new(module.top_level.clone());
        let unit = if let Some(unit) =
            self.logical_uplevel_source_unit(&asm, &module.source_namespace, evaluation, purpose)?
        {
            unit
        } else if let Some(unit) = self.logical_eval_source_unit(
            &asm,
            &module.source_namespace,
            strings,
            evaluation,
            purpose,
        )? {
            unit
        } else {
            self.direct_source_unit(
                asm,
                module.source_namespace.clone(),
                strings,
                evaluation,
                purpose,
            )?
        };
        Ok(
            unit.with_source_location(self.script_stack.last().map(|file| {
                tcl_runtime_api::script_source_location::ScriptSourceLocation {
                    file: file.clone(),
                    line: 1,
                }
            })),
        )
    }

    pub(crate) fn prepare_script_commands_value_for(
        &mut self,
        value: &Value,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
    ) -> Result<PreparedScript, TclError> {
        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(PreparedScript {
                prefix: Some(self.prepare_native_jim_script_unit(
                    value,
                    self.current_namespace_execution_context().clone(),
                )?),
                fatal_tail: None,
            });
        }
        let actual = self.actual_native_invocation_dialect();
        let protocol = actual.native_bytecode_storage_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original C bytecode issuer"),
        )?;
        let storage = protocol.recipe();
        let version = storage.version();
        let strings = actual.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original C source storage"),
        )?;
        let evaluation = self.eval_object_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original source evaluation purpose",
            ),
        )?;
        let compiles_source = evaluation.compiles_source(purpose);
        if let Some(cache) = value.native_bytecode_cache() {
            let body = &cache.unit;
            if compiles_source
                && cache.version == version
                && cache.context == crate::value::NativeBytecodeContext::Script
                && body.interpreter == self.native_interpreter_identity()
                && body.compiler.is_current_service(self.compiler_generation)
                && body.profile_generation == self.profile_generation
                && body.source_namespace.compiled_path() == Some(&self.source_namespace_path())
                && body.native_cache == Some(self.native_cache_stamp(self.current_ns_id()))
                && self.function_command_bindings_match_with_manifest(
                    &body.asm,
                    body.manifest.as_deref(),
                )
                && self.compiled_local_layout_matches(&body.asm)
                && !self.step_trace_active()
            {
                return Ok(PreparedScript {
                    prefix: Some(body.clone()),
                    fatal_tail: body.fatal_tail.clone(),
                });
            }
        }
        let source = self.native_script_source_image(value)?;
        self.jim_errors.stack.mark_reset();
        let namespace = self.source_namespace_path();
        let plan = self.native_script_command_plan(&source, &namespace)?;
        if evaluation.parse_failure_precedes_commands(purpose) && plan.fatal_tail.is_some() {
            return Ok(PreparedScript {
                prefix: None,
                fatal_tail: plan.fatal_tail,
            });
        }
        let admitted_source = tcl_runtime_api::SourceImage::from_bytes(
            &source.bytes()[..plan.complete_prefix_len],
            source.channel(),
        );
        let prefix = if plan.complete_prefix_len != 0
            || (compiles_source && storage.retains_parse_failure())
            || plan.fatal_tail.is_none()
        {
            let mut unit = if compiles_source {
                self.compile_script_cached_bytes(&admitted_source)?
            } else {
                self.direct_script_prefix(
                    &admitted_source,
                    &namespace,
                    strings,
                    evaluation,
                    purpose,
                )?
            };
            if compiles_source {
                unit = unit
                    .with_fatal_tail(plan.fatal_tail.clone())
                    .finalize_original_source_pool(value, strings)?;
                value.install_native_bytecode_cache(
                    version,
                    crate::value::NativeBytecodeContext::Script,
                    unit.clone(),
                )?;
            }
            Some(unit)
        } else {
            None
        };
        Ok(PreparedScript {
            prefix,
            fatal_tail: plan.fatal_tail,
        })
    }
}

/// The chunks `tclsh` evaluates its stdin in: each ends where the lines read
/// so far form complete commands ([`tcl_lexer::stdin_chunk_ends`]).
fn stdin_chunks(src: &str) -> impl Iterator<Item = &str> {
    let mut start = 0;
    tcl_lexer::stdin_chunk_ends(src.as_bytes())
        .into_iter()
        .map(move |end| {
            let chunk = &src[start..end];
            start = end;
            chunk
        })
}
