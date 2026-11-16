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

//! Building `errorInfo`: the `while executing` frames, procedure, body, namespace and lambda frames, and `return -code error` options.

use super::super::dispatch::{MethodFrameWhat, ProcFrame};
use super::super::evaluation::line_of;
use super::super::{
    drop_fresh, native_operation_currency, new_string, Code, ExceptionState, Interp,
};
use crate::{obj, parse};
use tcl_runtime_api::error_stack::validate_error_stack;

impl Interp {
    /// Log the command now running at the top of the `info frame` stack as an
    /// error unwinds through it ([`Self::log_command_bytes`]).
    pub(crate) fn log_current_command(&mut self) {
        let Some((line, cmd)) = self
            .cmd_frames
            .borrow()
            .last()
            .map(|f| (f.line.saturating_sub(f.line_base), f.cmd.clone()))
        else {
            return;
        };
        self.log_command_bytes(line, &cmd);
    }

    /// Pre-seed the error trace for `error msg info ?code?` / `throw`: the result
    /// is `msg`, the trace starts at `info` (so the throwing command is **not**
    /// re-logged — `ERR_ALREADY_LOGGED`), and `-errorcode` is `code`. Returns
    /// [`Code::Error`].
    pub(crate) fn raise_with_info(&mut self, msg: &[u8], info: &[u8], code: &[u8]) -> Code {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        self.set_result_bytes(msg);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return Code::Error;
        }
        if self.uses_jim_error_stack()
            && self
                .adopt_jim_stacktrace(obj::Owned::fresh(new_string(info)))
                .is_err()
        {
            return Code::Error;
        }
        if scope.currency().ensure_current_or_refuse().is_err() {
            return Code::Error;
        }
        let state = ExceptionState {
            native: Default::default(),
            info: Some(info.to_vec()),
            code: code.to_vec(),
            code_explicit: false,
            already_logged: true,
            primitive_getter: None,
            expression_error_stage: None,
        };
        if self
            .replace_exception_state_checked(scope.currency(), state)
            .and_then(|()| self.capture_native_error_objects_checked(scope.currency()))
            .is_err()
        {
            return Code::Error;
        }
        self.exc.borrow_mut().native.legacy_copy = true;
        Code::Error
    }

    /// Begin a fresh error whose result the caller has already set, with
    /// `-errorcode` `code` and an empty trace (it accumulates as the error
    /// unwinds). Used by `error msg`/`throw`. Returns [`Code::Error`].
    pub(crate) fn set_error_state(&mut self, code: &[u8]) -> Code {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        let state = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        if self
            .replace_exception_state_checked(scope.currency(), state)
            .and_then(|()| self.capture_native_error_objects_checked(scope.currency()))
            .is_err()
        {
            return Code::Error;
        }
        Code::Error
    }

    /// Apply the error-related options of a `return -code error` to the live
    /// exception state (`TclProcessReturn`'s `TCL_ERROR` arm). This populates the
    /// interp's errorInfo/errorCode/errorStack — *not* the `::errorInfo`/
    /// `::errorCode` globals, which are written only when the error is actually
    /// reported (caught as code 1 / reaching the top level). An explicit non-empty
    /// `-errorinfo` is taken verbatim and marks the error already-logged (so the
    /// unwind does not append a `while executing` frame); otherwise the trace
    /// accumulates normally. `-errorcode` defaults to `NONE`; a valid `-errorstack`
    /// replaces the built stack.
    pub(crate) fn process_return_error(
        &mut self,
        errorinfo: Option<&[u8]>,
        errorcode: Option<&[u8]>,
        errorstack: Option<&[u8]>,
    ) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.uses_jim_error_stack() {
            if let Some(info) = errorinfo {
                if self
                    .adopt_jim_stacktrace(obj::Owned::fresh(new_string(info)))
                    .is_err()
                {
                    return;
                }
            }
        }
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        let (info, already_logged) = match errorinfo {
            Some(i) if !i.is_empty() => (Some(i.to_vec()), true),
            _ => (None, false),
        };
        let state = ExceptionState {
            native: Default::default(),
            info,
            code: errorcode.unwrap_or(b"NONE").to_vec(),
            code_explicit: errorcode.is_some(),
            already_logged,
            primitive_getter: None,
            expression_error_stage: None,
        };
        if self
            .replace_exception_state_checked(scope.currency(), state)
            .and_then(|()| self.capture_native_error_objects_checked(scope.currency()))
            .is_err()
        {
            return;
        }
        self.exc.borrow_mut().native.legacy_copy = true;
        if let Some(es) = errorstack.filter(|_| self.runtime_version().has_error_stack()) {
            if let Ok(parts) = validate_error_stack(crate::parse::split_list(es)) {
                if self.adopt_error_stack_bytes_checked(&parts).is_err() {
                    return;
                }
            }
        }
        let _ = scope.currency().ensure_current_or_refuse();
    }

    /// Append one `while executing` / `invoked from within` frame for the command
    /// `src[cmd.start..cmd.end]` as an error unwinds through it — the
    /// `TclLogCommandInfo` mirror (`tclNamesp.c`). A no-op (consuming the flag)
    /// when the command was already logged deeper in the same script; otherwise
    /// it computes the 1-based source line, seeds `errorInfo` from the result on
    /// the first frame, truncates the command to 150 bytes (`...` on overflow),
    /// and sets `already_logged`.
    pub(in super::super) fn log_command_info(&mut self, src: &[u8], cmd: &parse::Command) {
        #[cfg(test)]
        self.observe_c84_error_episode("log.command_info_enter", None);
        // The same guard [`log_command_bytes`] applies. Repeated here so the
        // already-logged path — the common one on a deep unwind — skips the
        // newline scan `line_of` costs.
        if self.exc.borrow().already_logged {
            return;
        }
        self.log_evaluated_command_bytes(line_of(src, cmd.start), &src[cmd.start..cmd.end]);
    }

    /// [`log_command_info`](Self::log_command_info) over the command's bytes
    /// and its 1-based line within the enclosing script.
    ///
    /// The one implementation of C's `TclLogCommandInfo`, shared by the eval
    /// loop and by `tcl_codegen_log_command` — so `error_line` keeps its
    /// single writer, and a compiled statement gets the identical
    /// `already_logged` protocol, error-stack entry, and 150-byte truncation
    /// rather than a second, drifting copy of them.
    pub(crate) fn log_evaluated_command_bytes(&mut self, raw_line: u32, cmd_bytes: &[u8]) {
        self.log_command_bytes(raw_line, cmd_bytes);
        if self.check_entered_native_operation().is_err() {
            return;
        }
        if self
            .native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::evaluation_owns_logged_flag,
            )
        {
            self.exc.borrow_mut().already_logged = true;
        }
    }

    pub(crate) fn log_command_bytes(&mut self, raw_line: u32, cmd_bytes: &[u8]) {
        #[cfg(test)]
        self.observe_c84_error_episode("log.command_bytes_enter", None);
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.propagate_error_stage().is_err() {
            return;
        }
        if self.uses_jim_error_stack() {
            let _ = self.capture_jim_error_stack();
            return;
        }
        // Already logged deeper in the same script (e.g. an inner `[cmd]` subst,
        // or an inline `if`/`while` body): the enclosing command is the same C
        // bytecode frame, so it is *not* re-logged. The flag stays set and is
        // cleared only at a real frame boundary (`make_proc_error` /
        // `append_body_frame`), which is what lets the proc-*call* command log.
        if self.exc.borrow().already_logged {
            return;
        }
        // TIP 348: record this frame's inner context / uplevel boundary into the
        // error stack (the errorStack half of C's `Tcl_LogCommandInfo`).
        if self.error_stack_log(cmd_bytes).is_err() {
            return;
        }
        // errorLine, measured against the enclosing `codePtr->source` (C's
        // `TclLogCommandInfo`): the command's file-absolute line
        // (`line_base + 1 + count('\n')`) minus the proc/eval body's base, so an
        // inline `catch`/`if` body's commands still report their proc-body line.
        // This is the *only* writer of `error_line`; it persists across `catch`
        // and a subsequent `error msg info`.
        let line = self.cmd_frames.borrow().last().map_or(raw_line, |f| {
            (f.line_base + raw_line)
                .saturating_sub(f.proc_line_base)
                .max(1)
        });
        self.error_line.set(line);
        let started = self.exc.borrow().info.is_some();
        if self.seed_error_info_checked(scope.currency()).is_err() {
            #[cfg(test)]
            self.observe_c84_error_episode("log.seed_error", Some(false));
            return;
        }
        #[cfg(test)]
        self.observe_c84_error_episode("log.seed_success", Some(true));

        let verb: &[u8] = if started {
            b"invoked from within"
        } else {
            b"while executing"
        };
        let overflow = cmd_bytes.len() > 150;
        let slice = if overflow {
            &cmd_bytes[..150]
        } else {
            cmd_bytes
        };
        let mut exc = self.exc.borrow_mut();
        let buf = exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    ");
        buf.extend_from_slice(verb);
        buf.extend_from_slice(b"\n\"");
        buf.extend_from_slice(slice);
        if overflow {
            buf.extend_from_slice(b"...");
        }
        buf.push(b'"');
        exc.already_logged = true;
        drop(exc);
        if self
            .update_native_error_info_checked(scope.currency())
            .is_err()
        {
            return;
        }
        // Pre-8.5 compatibility (C's `TclLogCommandInfo`, tclNamesp.c): if user
        // code traces `::errorInfo` for writes, push the value out to the variable
        // *now*, mid-unwind — firing the write trace while the failing command's
        // call frame is still live (so the handler's `info level` sees it). Skipped
        // (no var write) when nothing traces `::errorInfo`, so the normal path
        // still publishes once, at the `catch`/top level.
        if !self.uses_c84_global_error_info() && self.errorinfo_has_write_trace() {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                self.publish_traced_native_error_info();
            } else {
                let info = match self.error_info() {
                    Ok(info) => info,
                    Err(first) => {
                        self.refuse_native_execution(first);
                        return;
                    }
                };
                let ei = new_string(&info);
                if self.var_set(b"::errorInfo", ei).is_err() {
                    drop_fresh(ei);
                }
            }
        }
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        if self
            .native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::evaluation_owns_logged_flag,
            )
        {
            // C8.4 Tcl_LogCommandInfo clears this after AddObjErrorInfo and
            // its setters' trace chains. The evaluating command owns the set.
            self.exc.borrow_mut().already_logged = false;
        }
    }

    /// Whether `::errorInfo` carries a user write-trace — C's `TclIsVarTraced`
    /// gate in `TclLogCommandInfo`. Hidden core read/unset registrations
    /// do not qualify as user write observers.
    fn errorinfo_has_write_trace(&self) -> bool {
        if self.traces.borrow().traces.is_empty() {
            return false;
        }
        // Same `(home, simple name)` key registration and firing use — a
        // literal `::errorInfo` here would miss every trace now that traces
        // are keyed by the resolved variable rather than the spelling.
        let home = self.trace_identity(b"::errorInfo");
        let cell = self.variable_trace_scope(&home, None);
        self.traces
            .borrow()
            .traces
            .iter()
            .any(|trace| cell.matches(trace, b"write"))
    }

    /// Append the `(procedure "NAME" line N)` / `(lambda term "..." line N)`
    /// frame when a proc/lambda body unwinds with an error (`MakeProcError` /
    /// `MakeLambdaError`, `tclProc.c`), then clear `already_logged` so the
    /// proc-call command itself is logged by its enclosing eval. The line is the
    /// body-relative `error_line` the innermost body command recorded.
    pub(in super::super) fn make_proc_error(&mut self, frame: ProcFrame) {
        if self.uses_jim_error_stack() {
            return;
        }
        // `(procedure "NAME" line N)` / `(lambda term "NAME" line N)` — the name
        // quoted, truncated to 60 bytes (`...` on overflow).
        // Append a name quoted and truncated to 60 bytes (`...` on overflow),
        // C's ELLIPSIFY.
        fn push_ellipsified(out: &mut Vec<u8>, name: &[u8]) {
            let overflow = name.len() > 60;
            out.push(b'"');
            out.extend_from_slice(if overflow { &name[..60] } else { name });
            if overflow {
                out.extend_from_slice(b"...");
            }
            out.push(b'"');
        }
        let inner = match frame {
            ProcFrame::JimLambda => return,
            ProcFrame::Proc(n) | ProcFrame::Lambda(n) => {
                let kind: &[u8] = if matches!(frame, ProcFrame::Lambda(_)) {
                    b"lambda term"
                } else {
                    b"procedure"
                };
                let mut inner = Vec::with_capacity(kind.len() + n.len() + 8);
                inner.extend_from_slice(kind);
                inner.push(b' ');
                push_ellipsified(&mut inner, n);
                inner
            }
            ProcFrame::Method { kind, owner, what } => {
                // `KIND "OWNER" method "NAME"` / `KIND "OWNER" constructor`.
                let mut inner = Vec::new();
                inner.extend_from_slice(kind);
                inner.push(b' ');
                push_ellipsified(&mut inner, owner);
                match what {
                    MethodFrameWhat::Named(name) => {
                        inner.extend_from_slice(b" method ");
                        push_ellipsified(&mut inner, name);
                    }
                    MethodFrameWhat::Constructor => inner.extend_from_slice(b" constructor"),
                    MethodFrameWhat::Destructor => inner.extend_from_slice(b" destructor"),
                }
                inner
            }
        };
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append a `("LABEL" body line N)` frame (the `eval`/`uplevel`/`foreach`
    /// body trace, e.g. `("eval" body line 1)`), then clear `already_logged` so
    /// the enclosing command logs. C emits these for script bodies that evaluate
    /// through a fresh `CmdFrame` (`eval`/`uplevel`/`foreach`), unlike the
    /// inline-compiled `if`/`while`/`for`/`switch`.
    pub(crate) fn append_body_frame(&mut self, label: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        // The `"label" body` shape: `("eval" body line N)`.
        let mut inner = Vec::with_capacity(label.len() + 8);
        inner.push(b'"');
        inner.extend_from_slice(label);
        inner.extend_from_slice(b"\" body");
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append the `(in namespace eval "<fqn>" script line N)` errorInfo frame
    /// when a `namespace eval` (`command` `eval`) or `namespace inscope` body
    /// unwinds with an error (C's `NsEval_Callback`), then clear
    /// `already_logged` so the command itself logs.
    pub(crate) fn append_namespace_eval_frame(&mut self, command: &str, fqn: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        let inner =
            tcl_cmd_core::namespace::script_frame_text(command, fqn, self.runtime_version());
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Shared tail of the `(... line N)` frames: append `"\n    (<inner> line
    /// <N>)"` to `errorInfo` (seeding it from the result message if no frame has
    /// been logged yet), where `inner` is the caller-built body — e.g.
    /// `procedure "p"`, `lambda term "..."`, or `"eval" body`.
    /// Clear `already_logged` after adding a frame at a real frame boundary
    /// (e.g. an OO definition script), so the enclosing command logs its own
    /// `invoked from within` frame.
    pub(crate) fn clear_error_logged(&self) {
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append the independently selected lambda-parse frame to errorInfo.
    /// This carries no line suffix and clears already_logged so the enclosing
    /// original invocation can still log its own frame.
    pub(crate) fn append_lambda_parse_frame(&mut self, frame: &[u8]) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.seed_error_info_checked(scope.currency()).is_err() {
            return;
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(frame);
        }
        if self
            .update_native_error_info_checked(scope.currency())
            .is_err()
        {
            return;
        }
        self.exc.borrow_mut().already_logged = false;
    }

    pub(crate) fn append_frame_line(&mut self, inner: &[u8]) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let line = self.error_line.get();
        if self.seed_error_info_checked(scope.currency()).is_err() {
            return;
        }
        let mut exc = self.exc.borrow_mut();
        let buf = exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    (");
        buf.extend_from_slice(inner);
        buf.extend_from_slice(b" line ");
        buf.extend_from_slice(line.to_string().as_bytes());
        buf.push(b')');
        drop(exc);
        let _ = self.update_native_error_info_checked(scope.currency());
    }

    /// Append a frame with no `line N` suffix — `"\n    (<text>)"`, e.g.
    /// `("for" initial command)` / `("for" loop-end command)` (C's
    /// `Tcl_AddErrorInfo` for the `for` init/next scripts) — seeding errorInfo
    /// from the result if needed, then clear `already_logged` so the enclosing
    /// command logs its own `invoked from within` frame.
    pub(crate) fn append_frame_noline(&mut self, text: &[u8]) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.seed_error_info_checked(scope.currency()).is_err() {
            return;
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    (");
            buf.extend_from_slice(text);
            buf.push(b')');
        }
        if self
            .update_native_error_info_checked(scope.currency())
            .is_err()
        {
            return;
        }
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append a literal `"\n    <text>"` errorInfo context line. Ensemble
    /// unknown-handler result diagnostics use this Tcl_AddErrorInfo shape (no
    /// parentheses and no line suffix), then allow the enclosing command to log
    /// its ordinary `invoked from within` frame.
    pub(crate) fn append_error_info_context(&mut self, text: &[u8]) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.seed_error_info_checked(scope.currency()).is_err() {
            return;
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    ");
            buf.extend_from_slice(text);
        }
        if self
            .update_native_error_info_checked(scope.currency())
            .is_err()
        {
            return;
        }
        self.exc.borrow_mut().already_logged = false;
    }
}
