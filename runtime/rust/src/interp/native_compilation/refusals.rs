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

//! Retained host and native refusals: recording the first refusal of an operation and reading it back.

use super::super::{Code, Interp};
use tcl_runtime_api::NativeCompilationAdmissionError;

#[cfg(debug_assertions)]
pub(super) fn trace_retained_native_refusal(slot: &str, cause: &impl std::fmt::Debug) {
    if std::env::var_os("TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        eprintln!("RUNTIME_RETAINED_NATIVE_REFUSAL slot={slot} cause={cause:?}");
    }
}

impl Interp {
    /// Retained host admission error, distinct from a native Tcl completion.
    /// A failed preflight prevents script effects even if Tcl catches its result.
    pub fn native_compilation_admission_error(&self) -> Option<NativeCompilationAdmissionError> {
        let state = self.native_compilation.borrow();
        state
            .admission_error
            .or_else(|| match &state.execution_refusal {
                Some(tcl_runtime_api::NativeExecutionError::CompilationAdmission(error)) => {
                    Some(*error)
                }
                _ => None,
            })
    }

    /// Reached Unicode access failure retained outside guest completion.
    pub fn unicode_access_refusal(&self) -> Option<tcl_syntax::raw_string::UnicodeAccessError> {
        match self.native_access_refusal() {
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(error)) => Some(error),
            _ => None,
        }
    }

    /// Reached native value operation refusal retained outside guest completion.
    pub fn native_access_refusal(
        &self,
    ) -> Option<tcl_syntax::raw_string::NativeValueAccessRefusal> {
        let state = self.native_compilation.borrow();
        state
            .native_access_refusal
            .or_else(|| match &state.execution_refusal {
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(error)) => {
                    Some(*error)
                }
                _ => None,
            })
    }

    /// Any retained host-only execution failure; guest capture cannot consume it.
    pub fn host_refusal_pending(&self) -> bool {
        let state = self.native_compilation.borrow();
        state.execution_refusal.is_some()
            || state.admission_error.is_some()
            || state.native_access_refusal.is_some()
            || state.host_command_refusal.is_some()
    }

    /// Transport a reached child host failure without manufacturing guest options.
    /// Each interpreter retains its own actual state; no value or lookup identity
    /// crosses this channel, and an earlier parent refusal remains authoritative.
    pub(crate) fn transport_host_refusal_from(&mut self, origin: &Interp) -> super::super::Code {
        if self.host_refusal_pending() {
            return super::super::Code::Error;
        }
        let original_cause = origin.native_execution_refusal();
        let origin = origin.native_compilation.borrow();
        let mut target = self.native_compilation.borrow_mut();
        target.execution_refusal = original_cause;
        if target.admission_error.is_none() {
            target.admission_error = origin.admission_error;
        }
        if target.native_access_refusal.is_none() {
            target.native_access_refusal = origin.native_access_refusal;
        }
        if target.host_command_refusal.is_none() {
            target
                .host_command_refusal
                .clone_from(&origin.host_command_refusal);
        }
        super::super::Code::Error
    }

    /// Test-only Copy snapshot of held failure fields; no native operation is entered.
    #[cfg(test)]
    pub(crate) fn capture_env_failure_snapshot(&self) -> crate::env_trace::EnvFailureSnapshot {
        use crate::env_trace::EnvFailureSnapshot;
        use tcl_runtime_api::NativeExecutionError;

        let mut snapshot = EnvFailureSnapshot {
            compilation_busy: false,
            execution_kind: None,
            execution_access: None,
            access: None,
            admission: None,
            host_command: false,
            result: None,
        };
        match self.native_compilation.try_borrow() {
            Ok(state) => {
                snapshot.execution_kind =
                    state.execution_refusal.as_ref().map(|error| match error {
                        NativeExecutionError::CompilationAdmission(_) => "compilation-admission",
                        NativeExecutionError::CompileServiceRefusal(_) => "compile-service",
                        NativeExecutionError::ExpressionRefusal(_) => "expression",
                        NativeExecutionError::ValueAccessRefusal(_) => "value-access",
                        NativeExecutionError::HostCommandRefusal(_) => "host-command",
                    });
                if let Some(NativeExecutionError::ValueAccessRefusal(error)) =
                    &state.execution_refusal
                {
                    snapshot.execution_access = Some(*error);
                }
                snapshot.access = state.native_access_refusal;
                snapshot.admission = state.admission_error;
                snapshot.host_command = state.host_command_refusal.is_some();
            }
            Err(_) => snapshot.compilation_busy = true,
        }
        let result = self.result.get();
        // SAFETY: the live interpreter owns its result role; null is explicit.
        // No callback, getter, conversion or native reference change occurs.
        snapshot.result = unsafe { result.as_ref() }.map(|header| {
            (
                result as usize,
                header.ref_count,
                header.length,
                header.bytes.is_null(),
                header.type_ptr as usize,
            )
        });
        snapshot
    }

    /// Actual retained host cause; guest completion state never supplies it.
    pub fn native_execution_refusal(&self) -> Option<tcl_runtime_api::NativeExecutionError> {
        if !self.0.native_compilation.refused_flag().get() {
            return None;
        }
        let state = self.native_compilation.borrow();
        if let Some(error) = &state.execution_refusal {
            return Some(error.clone());
        }
        if let Some(error) = state.admission_error {
            return Some(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
                error,
            ));
        }
        if let Some(error) = state.native_access_refusal {
            return Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                error,
            ));
        }
        state
            .host_command_refusal
            .clone()
            .map(tcl_runtime_api::NativeExecutionError::HostCommandRefusal)
    }

    /// Original host callback metadata, without projecting its diagnostic text.
    pub fn native_host_command_refusal(&self) -> Option<tcl_runtime_api::NativeHostCommandRefusal> {
        let state = self.native_compilation.borrow();
        state
            .host_command_refusal
            .as_deref()
            .cloned()
            .or_else(|| match &state.execution_refusal {
                Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(error)) => {
                    Some((**error).clone())
                }
                _ => None,
            })
    }

    /// A host callback's retained reporting text, independently of guest result bytes.
    pub fn host_command_refusal(&self) -> Option<String> {
        self.native_host_command_refusal()
            .map(|failure| failure.reason)
    }

    pub(crate) fn refuse_host_command(&mut self, reason: impl Into<String>) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let namespace_token = self.current_ns();
        let namespace = {
            let namespaces = self.namespaces();
            if namespaces.native_context_path(namespace_token).is_some() {
                namespaces.qualified_name(namespace_token)
            } else {
                b"<unavailable namespace>".to_vec()
            }
        };
        // This field is diagnostic display only; actual ownership remains the
        // retained interpreter and namespace token, including for opaque bytes.
        let namespace = std::str::from_utf8(&namespace)
            .map_or_else(|_| namespace.escape_ascii().to_string(), str::to_owned);
        let profile = self.dialect_profile().cache_key();
        let failure = tcl_runtime_api::NativeHostCommandRefusal {
            reason: reason.into(),
            source_profile: profile,
            native_profile: profile,
            interpreter: self.native_command_interpreter,
            frame: self.frames.borrow().current_level(),
            namespace: namespace.into(),
            namespace_token: Some(namespace_token as u64),
        };
        self.native_compilation.borrow_mut().host_command_refusal = Some(Box::new(failure));
        #[cfg(debug_assertions)]
        trace_retained_native_refusal(
            "host_command_refusal",
            &self.native_compilation.borrow().host_command_refusal,
        );
        Code::Error
    }

    pub(crate) fn refuse_unicode_access(
        &mut self,
        error: tcl_syntax::raw_string::UnicodeAccessError,
    ) -> crate::interp::Code {
        self.refuse_native_access(error.into())
    }

    pub(crate) fn refuse_native_access(
        &mut self,
        error: tcl_syntax::raw_string::NativeValueAccessRefusal,
    ) -> crate::interp::Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        self.native_compilation
            .borrow_mut()
            .native_access_refusal
            .get_or_insert(error);
        #[cfg(debug_assertions)]
        trace_retained_native_refusal(
            "native_access_refusal",
            &self.native_compilation.borrow().native_access_refusal,
        );
        crate::interp::Code::Error
    }

    /// Retain the first actual cause before unwinding a failed value projection.
    /// Nested consumers never reset or replace an earlier refusal.
    pub(crate) fn refuse_native_execution(
        &mut self,
        error: tcl_runtime_api::NativeExecutionError,
    ) -> Code {
        if !self.host_refusal_pending() {
            self.native_compilation.borrow_mut().execution_refusal = Some(error);
            #[cfg(debug_assertions)]
            trace_retained_native_refusal(
                "execution_refusal",
                &self.native_compilation.borrow().execution_refusal,
            );
        }
        Code::Error
    }
}
