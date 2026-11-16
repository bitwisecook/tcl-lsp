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

//! The frames of `errorInfo`: `while executing` and `invoked from within`, body, clause, procedure and lambda frames, and the error line.

use super::super::Vm;
use super::ErrorLogged;
use crate::value::Value;
use std::rc::Rc;

/// [`tcl_syntax::naming::key_holder_and_tail`] for the VM's **unrooted** key
/// convention: root the key, split by the construction-inverse rule, and
/// unroot the holder.  Distinct from the written-name colon-run split — an
/// unrooted key can begin with `::` when a namespace is named `:`, and
/// the all-colon disambiguation differs between the rooted and unrooted
/// grammars.
pub(crate) fn key_holder_and_tail_unrooted(key: &str) -> (String, String) {
    let rooted = tcl_syntax::naming::root_unrooted_key(key);
    let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(&rooted);
    let holder = tcl_syntax::naming::unroot_rooted_key(holder).unwrap_or(holder);
    (holder.to_string(), tail.to_string())
}

impl Vm {
    /// Append one `while executing` / `invoked from within` frame for the
    /// command `cmd_text` to the accumulating `errorInfo` trace (C's
    /// `TclLogCommandInfo`). The first frame seeds the trace from the error
    /// message `msg` and reads "while executing"; later frames read "invoked
    /// from within". A command level already logged in the same bytecode frame
    /// (`error_logged`) is not re-logged. Command text over 150 bytes is
    /// truncated with `...`, as in C.
    pub(crate) fn log_command_info(
        &mut self,
        cmd_text: impl AsRef<[u8]>,
        msg: impl AsRef<[u8]>,
        line: u32,
    ) {
        let bytes = cmd_text.as_ref();
        self.log_command_info_with_context(
            bytes,
            Value::from_native_string_bytes(bytes),
            msg,
            line,
        );
    }

    /// Value-preserving form of [`Self::log_command_info`]. `cmd_text` remains
    /// the source spelling for `errorInfo`; `context` is the post-substitution
    /// operation stored in TIP 348's first `INNER` entry.
    pub(crate) fn log_command_info_with_context(
        &mut self,
        cmd_text: impl AsRef<[u8]>,
        context: Value,
        msg: impl AsRef<[u8]>,
        line: u32,
    ) {
        if self.execution_refusal.is_some() || self.native_errors.error_logged.is_logged() {
            return;
        }
        self.error_stack_log_value(context);
        self.log_command_info_only(cmd_text, msg, line);
    }

    /// ErrorInfo-only half of command logging. Compiler error regions call this
    /// while reconstructing synthetic body frames; those are not additional
    /// Tcl command logs and therefore must not duplicate TIP 348 `UP` entries.
    pub(crate) fn log_command_info_only(
        &mut self,
        cmd_text: impl AsRef<[u8]>,
        msg: impl AsRef<[u8]>,
        line: u32,
    ) {
        if self.uses_jim_error_stack() {
            return;
        }
        if self.execution_refusal.is_some() || self.native_errors.error_logged.is_logged() {
            return;
        }
        let source = cmd_text.as_ref();
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_error_log_protocol()
        else {
            let _ = self
                .refuse_host_command("native command-source error logging is unavailable".into());
            return;
        };
        let Some((slice, overflow)) = protocol.excerpt(source, source.len()) else {
            let _ = self.refuse_host_command(
                "native command-source error logging extent is unavailable".into(),
            );
            return;
        };
        // The innermost logged command's line drives the enclosing `(procedure …
        // line N)` / `("while" body line N)` frames (C's `iPtr->errorLine`).
        if line != 0 {
            self.native_errors.error_line = line;
        }
        let started = self.native_errors.error_info.is_some();
        if !started {
            self.seed_native_error_result_object(msg.as_ref());
        }
        let info = self
            .native_errors
            .error_info
            .get_or_insert_with(|| msg.as_ref().to_vec());
        let verb = if started {
            "invoked from within"
        } else {
            "while executing"
        };
        info.extend_from_slice(("\n    ").as_bytes());
        info.extend_from_slice((verb).as_bytes());
        info.extend_from_slice(("\n\"").as_bytes());
        info.extend_from_slice(slice);
        if overflow {
            info.extend_from_slice(("...").as_bytes());
        }
        info.push(b'"');
        self.native_errors.error_logged = ErrorLogged::Yes;
        self.update_native_error_info();
        self.publish_traced_native_error_info();
    }

    /// Append a `("<label>" body line N)` frame to the trace — the frame an
    /// *uncompiled* `while`/`for`/`foreach` (the runtime command form) adds when
    /// its body errors (C's interpreted `Tcl_WhileObjCmd` &c). `N` is the
    /// innermost logged command's line. Clears `error_logged` so the enclosing
    /// command then logs its own `invoked from within` frame.
    pub(crate) fn append_body_frame(&mut self, label: &str) {
        self.append_body_frame_line(label, self.native_errors.error_line);
    }

    /// Append a `("<label>" body line N)` frame with an explicit `line` — for an
    /// *inlined* body (`FunctionAsm::error_regions`), whose body-relative line is
    /// the innermost command's line minus the body's `line_base` (the uncompiled
    /// [`Self::append_body_frame`] uses `error_line` directly, since that path
    /// compiles the body standalone so its lines are already body-relative).
    pub(crate) fn append_body_frame_line(&mut self, label: &str, line: u32) {
        if self.uses_jim_error_stack() {
            return;
        }
        let info = self.native_errors.error_info.get_or_insert_with(Vec::new);
        info.extend_from_slice(("\n    (\"").as_bytes());
        info.extend_from_slice((label).as_bytes());
        info.extend_from_slice(("\" body line ").as_bytes());
        info.extend_from_slice(line.to_string().as_bytes());
        info.push(b')');
        self.native_errors.error_logged = self.native_errors.error_logged.past_frame();
        self.update_native_error_info();
    }

    /// Append a `(<clause> line N)` frame, `N` the innermost logged command's
    /// line: a clause other than a `body` one, such as `switch`'s
    /// `("PATTERN" arm line N)` ([`tcl_cmd_core::switch::arm_frame_text`]).
    pub(crate) fn append_clause_frame_line(&mut self, clause: &[u8]) {
        let mut frame = clause.to_vec();
        frame.extend_from_slice(b" line ");
        frame.extend_from_slice(self.native_errors.error_line.to_string().as_bytes());
        self.append_clause_frame(&frame);
    }

    /// Append a `(<clause>)` frame, such as `dict with`'s
    /// `(body of "dict with")`, so the enclosing command logs after it.
    pub(crate) fn append_clause_frame(&mut self, clause: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        let info = self.native_errors.error_info.get_or_insert_with(Vec::new);
        info.extend_from_slice(b"\n    (");
        info.extend_from_slice(clause);
        info.push(b')');
        self.native_errors.error_logged = self.native_errors.error_logged.past_frame();
        self.update_native_error_info();
    }

    /// Seed `errorInfo` for an error that *originates* in a command (not from a
    /// sub-command) with a context frame (C's `Tcl_AppendObjToErrorInfo`): start
    /// it from `msg` if unset or empty, append `frame` verbatim, and clear `error_logged`
    /// so the enclosing command then logs its `invoked from within` frame.
    /// Used by `apply` for the `(parsing lambda expression "…")` frame.
    /// C's `Tcl_AddErrorInfo` for the frames a command adds to its own error
    /// (`(reading increment)`) before its caller logs it. A trace the
    /// interpreter already holds, such as a callback's, is not added again.
    pub(crate) fn add_command_error_info(&mut self, message: &[u8], info: &[u8]) {
        if self.native_errors.error_info.as_deref() == Some(info) {
            return;
        }
        let Some(frame) = info.strip_prefix(message).filter(|frame| !frame.is_empty()) else {
            return;
        };
        self.seed_error_info_frame(message, frame);
    }

    pub(crate) fn seed_error_info_frame(&mut self, msg: impl AsRef<[u8]>, frame: impl AsRef<[u8]>) {
        if self.uses_jim_error_stack() {
            return;
        }
        if self
            .native_errors
            .error_info
            .as_ref()
            .is_some_and(Vec::is_empty)
        {
            self.native_errors.error_info = None;
        }
        if self.native_errors.error_info.is_none() {
            self.seed_native_error_result_object(msg.as_ref());
        }
        let info = self
            .native_errors
            .error_info
            .get_or_insert_with(|| msg.as_ref().to_vec());
        info.extend_from_slice(frame.as_ref());
        self.native_errors.error_logged = ErrorLogged::No;
        self.update_native_error_info();
    }

    #[cfg(debug_assertions)]
    pub(crate) fn native_diagnostic_proc_name_prefix(&self) -> Option<(&[u8], usize)> {
        self.frames
            .last()?
            .proc_name
            .as_deref()
            .map(|name| (&name.as_bytes()[..name.len().min(32)], name.len()))
    }

    /// The current call frame's proc name (unqualified), or `None` at the global
    /// frame / a non-proc activation.
    pub(crate) fn current_proc_name(&self) -> Option<String> {
        self.frames
            .last()
            .and_then(|f| f.proc_name.as_ref())
            // `proc_name` is an unrooted key: construction-inverse tail.
            .map(|q| key_holder_and_tail_unrooted(q).1)
    }

    /// Append a `(procedure "<name>" line N)` frame — the frame a proc body adds
    /// to errorInfo as an error unwinds out of it (C's `errorInfo` proc frame).
    /// `line` is the proc-relative line of the innermost logged command. Clears
    /// `error_logged` so the call site then logs its `invoked from within` frame.
    pub(crate) fn append_proc_frame(&mut self, name: &str, line: u32) {
        self.append_callable_frame("procedure", name, line);
    }

    /// Append a `(lambda term "<lambda>" line N)` frame — the frame an `apply`
    /// lambda body adds in place of a procedure's (C's `MakeLambdaError`).
    pub(crate) fn append_lambda_frame(&mut self, lambda: &str, line: u32) {
        self.append_callable_frame("lambda term", lambda, line);
    }

    /// The lambda expression of the `apply` lambda whose body the current call
    /// frame runs, if it is one ([`crate::command::ProcDef::lambda_term`]).
    /// The declaration is the frame's own procedure binding, never a lookup of
    /// its name (a lambda has no command name).
    pub(crate) fn current_lambda_term(&self) -> Option<Value> {
        let frame = self.frames.last()?;
        let declaration = match (
            frame.native_procedure_binding.as_ref(),
            frame.native_procedure_execution.as_ref(),
        ) {
            (Some(binding), _) => binding.declaration(),
            (None, Some(execution)) => Rc::clone(execution.declaration()),
            (None, None) => return None,
        };
        declaration.lambda_term.clone()
    }

    /// C's `MakeProcError`/`MakeLambdaError`: the name is cut at 60 bytes
    /// with `...`.
    fn append_callable_frame(&mut self, kind: &str, name: &str, line: u32) {
        const LIMIT: usize = 60;
        if self.uses_jim_error_stack() {
            return;
        }
        let shown = if name.len() > LIMIT {
            let mut end = LIMIT;
            while !name.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &name[..end])
        } else {
            name.to_owned()
        };
        let info = self.native_errors.error_info.get_or_insert_with(Vec::new);
        info.extend_from_slice(b"\n    (");
        info.extend_from_slice(kind.as_bytes());
        info.extend_from_slice(b" \"");
        info.extend_from_slice(shown.as_bytes());
        info.extend_from_slice(b"\" line ");
        info.extend_from_slice(line.to_string().as_bytes());
        info.push(b')');
        self.native_errors.error_logged = ErrorLogged::No;
        self.update_native_error_info();
    }

    pub(in super::super) fn append_var_trace_frame_bytes(&mut self, op: &str, name: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        let info = self.native_errors.error_info.get_or_insert_with(Vec::new);
        info.extend_from_slice(("\n    (").as_bytes());
        info.extend_from_slice((op).as_bytes());
        info.extend_from_slice((" trace on \"").as_bytes());
        info.extend_from_slice(name);
        info.extend_from_slice(("\")").as_bytes());
        self.native_errors.error_logged = ErrorLogged::No;
        self.update_native_error_info();
    }

    /// The innermost logged command's source line (C's `iPtr->errorLine`).
    pub(crate) fn error_line(&self) -> u32 {
        self.native_errors.error_line
    }

    /// Set the innermost error line directly — the proc epilogue's
    /// break/continue→error transform pins the offending command's line before
    /// the `(procedure …)` frame is appended.
    pub(crate) fn set_error_line(&mut self, line: u32) {
        self.native_errors.error_line = line;
    }

    /// Append a `(<text>)` frame — a `switch` arm's `("PATTERN" arm line N)`,
    /// `("for" initial command)` — and clear `error_logged` so the enclosing
    /// command then logs its own `invoked from within` frame.
    pub(crate) fn append_frame_text(&mut self, text: &str) {
        if self.uses_jim_error_stack() {
            return;
        }
        let info = self.native_errors.error_info.get_or_insert_with(Vec::new);
        info.extend_from_slice(b"\n    (");
        info.extend_from_slice(text.as_bytes());
        info.push(b')');
        self.native_errors.error_logged = self.native_errors.error_logged.past_frame();
        self.update_native_error_info();
    }
}
