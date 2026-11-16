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

//! Entering evaluation from outside: `eval_str` and the completion forms, `tclsh`'s script file and stdin, and `source`.

use super::super::{native_operation_currency, CmdFrame, Code, FrameKind, Interp};
use crate::{obj, parse};
use std::rc::Rc;
use tcl_runtime_api::CommandEvaluation;

impl Interp {
    /// Evaluate one source boundary, projecting its live completion before
    /// outermost error propagation and background-error processing.
    fn eval_sourced_boundary<T>(
        &mut self,
        script: &[u8],
        name: &[u8],
        evaluation: CommandEvaluation,
        project: impl FnOnce(&mut Self, Code) -> T,
    ) -> T {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return project(self, Code::Error);
            }
        };
        self.reset_outermost_native_error();
        if let Err(error) = self.record_package_source_file(name) {
            let code = self.report_cmd_error(error.into());
            return project(self, code);
        }
        if let Err(first) = self.clear_return_options_checked() {
            self.refuse_native_execution(first);
            return project(self, Code::Error);
        }
        self.script_stack.borrow_mut().push(name.to_vec());
        // A `source`d file is its own `info frame` level: `type source` + the
        // file path, inheriting the enclosing proc/level. Its commands are
        // numbered by the file's own lines (base 0, the file *is* the script).
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Source;
        frame.file = Some(Rc::from(name));
        frame.line_base = 0;
        self.next_evaluation.set(Some(evaluation));
        let code = if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            self.eval_native_jim_source_unpublished(script, name, frame)
        } else {
            self.eval_framed_unpublished(script, frame)
        };
        self.script_stack.borrow_mut().pop();
        if scope.currency().ensure_current_or_refuse().is_err() {
            return project(self, Code::Error);
        }
        // An error a `return` settles to names no file, as C's `else if`.
        let raised = code == Code::Error;
        let code = self.settle_source_return(code);
        if raised && !self.uses_jim_error_stack() {
            self.append_frame_line(&tcl_cmd_core::error::file_frame_text(
                name,
                self.runtime_version(),
            ));
            self.clear_error_logged();
        }
        if scope.currency().ensure_current_or_refuse().is_err() {
            return project(self, Code::Error);
        }
        let projected = project(self, code);
        if self
            .native_invocation_dialect()
            .native_eval_object_protocol()
            .is_some_and(|protocol| protocol.clears_public_source_error_logged())
        {
            self.exc.borrow_mut().already_logged = false;
        }
        self.finish_outermost_eval(code);
        let _ = scope.currency().ensure_current_or_refuse();
        projected
    }

    /// `source`: evaluate `script` as a sourced file named `name`, tracking it on
    /// the script stack (`info script`). A top-level `return` ends the file (the
    /// return boundary maps `return` → Ok); other codes propagate.
    ///
    /// Each command is compiled, as the `source` command's script is, but
    /// where the release evaluates a sourced file directly (8.4's
    /// `Tcl_EvalFile`).
    pub fn eval_sourced(&mut self, script: &[u8], name: &[u8]) -> Code {
        let evaluation = if tcl_registry::release_compilation::evaluates_script_directly(
            self.runtime_version(),
            "source",
            1,
        ) {
            CommandEvaluation::Direct
        } else {
            CommandEvaluation::Compiled
        };
        self.eval_sourced_boundary(script, name, evaluation, |_, code| code)
    }

    /// Run `f`, whose script loop evaluates the script the command `command`
    /// runs from `script_words` words, evaluating that script's commands
    /// directly where the release does
    /// (`tcl_registry::release_compilation::evaluates_script_directly`).
    pub(crate) fn with_script_evaluation<T>(
        &mut self,
        command: &str,
        script_words: usize,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let direct = tcl_registry::release_compilation::evaluates_script_directly(
            self.runtime_version(),
            command,
            script_words,
        );
        if direct {
            self.next_evaluation.set(Some(CommandEvaluation::Direct));
        }
        let result = f(self);
        if direct {
            self.next_evaluation.set(None);
        }
        result
    }

    /// The script file `tclsh` is given, named `name`, and its owned,
    /// byte-preserving completion: its commands are evaluated directly with no
    /// command active, so a completion leaving one is settled as C settles one
    /// at `numLevels == 0` and an error is logged against the command, before
    /// the file's own frame is added.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_script_file_completion(
        &mut self,
        script: &[u8],
        name: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        self.next_top_level_file.set(true);
        let completion = self.eval_sourced_boundary(
            script,
            name,
            CommandEvaluation::Direct,
            crate::completion::capture_bytes,
        );
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// Evaluate a package file in the interpreter-global frame and namespace.
    pub(crate) fn eval_sourced_global(&mut self, script: &[u8], name: &[u8]) -> Code {
        let previous_level = self.frames.borrow_mut().set_active_level(0);
        let previous_namespace = self.current_ns.get();
        self.current_ns.set(self.frames.borrow().frame_ns(0));
        let code = self.eval_sourced(script, name);
        self.frames.borrow_mut().set_active_level(previous_level);
        self.current_ns.set(previous_namespace);
        code
    }

    /// Evaluate a sourced script and return its owned, byte-preserving
    /// completion.
    ///
    /// The source return boundary is settled before the snapshot. An uncaught
    /// error is published to Tcl's globals only after its live options,
    /// including `-during`, have been captured. Host refusals return their typed
    /// original cause without fabricating a Tcl result or options dictionary.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_sourced_completion(
        &mut self,
        script: &[u8],
        name: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let completion = self.eval_sourced_boundary(
            script,
            name,
            CommandEvaluation::Direct,
            crate::completion::capture_bytes,
        );
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// `info script` — the file currently being sourced (empty at top level).
    pub(crate) fn current_script(&self) -> Vec<u8> {
        self.script_stack
            .borrow()
            .last()
            .cloned()
            .unwrap_or_default()
    }

    /// `info script filename` — set the current script name (C's
    /// `iPtr->scriptFile`), replacing the innermost entry (or seeding one at the
    /// top level).
    pub(crate) fn set_current_script(&self, name: &[u8]) {
        let mut s = self.script_stack.borrow_mut();
        match s.last_mut() {
            Some(last) => *last = name.to_vec(),
            None => s.push(name.to_vec()),
        }
    }

    /// The file currently being sourced, as a shared handle (`None` at the top
    /// level) — for stamping a definition/method body's source provenance.
    pub(crate) fn current_source_file(&self) -> Option<Rc<[u8]>> {
        self.script_stack
            .borrow()
            .last()
            .map(|f| Rc::from(f.as_slice()))
    }

    // eval

    /// Evaluate through the common outer boundary, projecting the live
    /// completion before applying its publication tail.
    fn eval_str_boundary<T>(
        &mut self,
        src: &[u8],
        evaluation: CommandEvaluation,
        project: impl FnOnce(&mut Self, Code) -> T,
    ) -> T {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return project(self, Code::Error);
            }
        };
        self.reset_outermost_native_error();
        if self.eval_depth.get() == 0 {
            if let Err(first) = self.clear_return_options_checked() {
                self.refuse_native_execution(first);
                return project(self, Code::Error);
            }
        }
        let owned = self.cmd_frames.borrow().is_empty().then(CmdFrame::root);
        let code = if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            // Jim's source entry evaluates an original String through the Script
            // owner. Real token objects retain Source on a returned literal.
            let original = obj::Owned::fresh(obj::new_string_bytes(src));
            let frame = owned.unwrap_or_else(|| self.inherited_cmd_frame());
            if self.codegen_activation_enter() {
                let code = self.eval_native_jim_script(original, frame);
                // The public projector captures the live completion before
                // this boundary publishes its outermost exception state.
                self.eval_depth.set(self.eval_depth.get().saturating_sub(1));
                code
            } else {
                Code::Error
            }
        } else {
            if owned.is_some() {
                self.next_evaluation.set(Some(evaluation));
            }
            self.eval_script_mode_unpublished(src, owned, false)
        };
        if scope.currency().ensure_current_or_refuse().is_err() {
            return project(self, Code::Error);
        }
        let projected = project(self, code);
        if self
            .native_invocation_dialect()
            .native_eval_object_protocol()
            .is_some_and(|protocol| protocol.clears_public_source_error_logged())
        {
            self.exc.borrow_mut().already_logged = false;
        }
        self.finish_outermost_eval(code);
        let _ = scope.currency().ensure_current_or_refuse();
        projected
    }

    /// Evaluate a whole script; the result is left in the interp result. Returns
    /// the completion code of the last command (or `Ok` for an empty script).
    ///
    /// At the true top level (no `info frame` stack yet) this owns the root
    /// `CmdFrame`; nested calls (command substitution `[cmd]`) run in the
    /// enclosing frame and add none — matching C, where `[cmd]` is the same
    /// `cmdFramePtr` level. A proc body / `eval` / `source` body gets its own
    /// frame via [`eval_framed`](Self::eval_framed).
    ///
    /// The script's own commands are evaluated directly, as `Tcl_EvalEx` does;
    /// a nested call takes the evaluation of the script it runs within.
    pub fn eval_str(&mut self, src: &[u8]) -> Code {
        self.eval_str_boundary(src, CommandEvaluation::Direct, |_, code| code)
    }

    /// Evaluate a script and return an owned, byte-preserving completion.
    ///
    /// This is the public host/embedding boundary. The result and live return
    /// options are captured from the live exception. Actual C private error
    /// objects remain owned after this method returns, until a native result
    /// reset or a hidden error-variable read. No text encoding is applied.
    /// Host-only failures return their original typed cause independently of
    /// every guest completion code, including Tcl errors captured by `catch`.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_completion(
        &mut self,
        script: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let completion = self.eval_str_boundary(
            script,
            CommandEvaluation::Direct,
            crate::completion::capture_bytes,
        );
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// [`Self::eval_completion`] with each command compiled, as `tclsh`
    /// compiles every command its stdin feeds it.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_compiled_completion(
        &mut self,
        script: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let completion = self.eval_str_boundary(
            script,
            CommandEvaluation::Compiled,
            crate::completion::capture_bytes,
        );
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// One chunk `tclsh` read from its stdin, compiled and evaluated with no
    /// command active: a completion leaving it is settled as C settles one at
    /// `numLevels == 0` (`TclUpdateReturnInfo`, then
    /// `ProcessUnexpectedResult`).
    fn eval_top_level_chunk(
        &mut self,
        chunk: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            // Jim's source entry owns its own script evaluation.
            return self.eval_compiled_completion(chunk);
        }
        let refused = |interp: &mut Self| match interp.native_execution_refusal() {
            Some(error) => Err(error),
            None => crate::completion::capture_bytes(interp, Code::Error),
        };
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return refused(self);
            }
        };
        self.reset_outermost_native_error();
        if self.eval_depth.get() == 0 {
            if let Err(first) = self.clear_return_options_checked() {
                self.refuse_native_execution(first);
                return refused(self);
            }
        }
        let owned = self.cmd_frames.borrow().is_empty().then(CmdFrame::root);
        if owned.is_some() {
            self.next_evaluation.set(Some(CommandEvaluation::Compiled));
        }
        let code = self.eval_script_mode_unpublished(chunk, owned, false);
        let code = match self.settle_return(code) {
            settled @ (Code::Ok | Code::Error) => settled,
            unexpected => {
                let (message, error_code) = self.unexpected_result(unexpected);
                self.raise_with_info(&message, &message, &error_code)
            }
        };
        if scope.currency().ensure_current_or_refuse().is_err() {
            return refused(self);
        }
        let projected = crate::completion::capture_bytes(self, code);
        self.finish_outermost_eval(code);
        let _ = scope.currency().ensure_current_or_refuse();
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        projected
    }

    /// The message and error code C's `ProcessUnexpectedResult` gives a
    /// completion code no context takes.
    fn unexpected_result(&self, code: Code) -> (Vec<u8>, Vec<u8>) {
        let message = match code {
            Code::Break => b"invoked \"break\" outside of a loop".to_vec(),
            Code::Continue => b"invoked \"continue\" outside of a loop".to_vec(),
            other => format!("command returned bad code: {}", other.as_int()).into_bytes(),
        };
        let error_code = if self.runtime_version().has_unexpected_result_error_codes() {
            format!("TCL UNEXPECTED_RESULT_CODE {}", code.as_int()).into_bytes()
        } else {
            b"NONE".to_vec()
        };
        (message, error_code)
    }

    /// `code`, which the command `cmd` of the script file `tclsh` was given
    /// left, settled as C settles a completion with no command active
    /// (`TclUpdateReturnInfo`, then `ProcessUnexpectedResult`), and an error
    /// it leaves logged against `cmd` (`TclEvalEx`). From 8.5 a `return`
    /// carrying `-errorinfo` has logged its error already; 8.4's
    /// `TclUpdateReturnInfo` leaves the command to log it.
    pub(super) fn settle_top_level_command(
        &mut self,
        src: &[u8],
        cmd: &parse::Command,
        code: Code,
    ) -> Code {
        let returned = code == Code::Return;
        let code = match self.settle_return(code) {
            settled @ (Code::Ok | Code::Error) => settled,
            unexpected => {
                let (message, error_code) = self.unexpected_result(unexpected);
                self.error_with_code(&message, &error_code)
            }
        };
        if code == Code::Error {
            if returned && !self.runtime_version().has_return_options_dictionary() {
                self.exc.borrow_mut().already_logged = false;
            }
            self.log_command_info(src, cmd);
        }
        code
    }

    /// The commands `tclsh` reads from its stdin `script`: line by line until
    /// the lines read form complete commands
    /// ([`tcl_lexer::stdin_chunk_ends`]), each such chunk evaluated by
    /// [`Self::eval_compiled_completion`]. A chunk that does not complete
    /// normally hands its result to `report`, and the next runs, as `tclsh`
    /// prints the error and reads on; an `exit` ends the run, and an
    /// incomplete last chunk never runs. The completion is the last chunk's.
    ///
    /// # Errors
    /// Returns the first chunk's retained host execution refusal; no later
    /// chunk runs.
    pub fn eval_stdin(
        &mut self,
        script: &[u8],
        mut report: impl FnMut(&mut Self, &[u8]),
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let mut completion = tcl_runtime_api::ScriptCompletion::new(
            tcl_runtime_api::Code::Ok,
            Vec::new(),
            b"-code 0 -level 0".to_vec(),
        );
        let mut start = 0;
        for end in tcl_lexer::stdin_chunk_ends(script) {
            completion = self.eval_top_level_chunk(&script[start..end])?;
            start = end;
            if self.exit_pending() {
                break;
            }
            if !completion.is_ok() {
                report(self, &completion.result);
            }
        }
        Ok(completion)
    }
}
