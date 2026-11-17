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

//! Refusals kept outside every guest completion: expression refusals, host-command and host failures, and taking the refusal at the host boundary.

use super::super::{Vm, err};
use crate::error::TclError;
use crate::value::Value;
use tcl_runtime_api::Completion;

impl Vm {
    /// This empty internal unwind carrier is never a guest error. The engine
    /// prioritises the separate typed refusal before completion settlement.
    pub(in super::super) fn refuse_expression(
        &mut self,
        source: &str,
        reason: tcl_runtime_api::NativeExpressionRefusal,
    ) -> TclError {
        self.refuse_expression_bytes(source.as_bytes(), reason)
    }

    pub(in super::super) fn refuse_expression_bytes(
        &mut self,
        source: &[u8],
        reason: tcl_runtime_api::NativeExpressionRefusal,
    ) -> TclError {
        if self.execution_refusal.is_none() {
            let (namespace, namespace_token) = self.native_refusal_namespace();
            let failure = tcl_runtime_api::NativeExpressionFailure {
                reason,
                source: tcl_lexer::SourceImage::native(source),
                source_profile: self.expression_source_profile().cache_key(),
                native_profile: self.native_execution_profile().cache_key(),
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: self.owner_nonce,
                    interpreter: u64::try_from(self.cur.0).expect("interpreter slot fits u64"),
                },
                frame: self.current_level(),
                namespace,
                namespace_token,
            };
            self.retain_native_execution_refusal(
                tcl_runtime_api::NativeExecutionError::ExpressionRefusal(Box::new(failure)),
            );
        }
        TclError::from_execution_failure(
            self.execution_refusal
                .clone()
                .expect("expression refusal retained"),
        )
    }

    /// Retain an original operational failure outside every guest completion.
    /// The returned empty carrier only unwinds the current activation; public
    /// host boundaries return the original cause through their fallible result.
    pub fn refuse_tcl_host_failure(
        &mut self,
        failure: crate::error::TclHostFailure,
    ) -> Completion<Value> {
        self.retain_tcl_host_failure(failure);
        err("")
    }

    pub(crate) fn retain_native_execution_refusal(
        &mut self,
        cause: tcl_runtime_api::NativeExecutionError,
    ) -> tcl_runtime_api::NativeExecutionError {
        self.native_operation_context.withdraw_allocations();
        for slot in &self.interps {
            if let Some(state) = &slot.parked {
                state.native_operation_context.withdraw_allocations();
            }
        }
        self.execution_refusal.get_or_insert(cause).clone()
    }

    pub(crate) fn retain_tcl_host_failure(
        &mut self,
        failure: crate::error::TclHostFailure,
    ) -> tcl_runtime_api::NativeExecutionError {
        let cause = match failure {
            crate::error::TclHostFailure::ValueAccess(error) => {
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(error)
            }
            crate::error::TclHostFailure::Execution(error) => error,
            crate::error::TclHostFailure::PrimitiveErrorStateRequired(error) => {
                return self.retain_host_command_refusal(format!(
                    "primitive error state is unavailable at this context-free conversion: {error:?}"
                ));
            }
        };
        self.retain_native_execution_refusal(cause)
    }

    /// Retain an explicit host-command refusal outside guest Tcl completion.
    /// The returned carrier only unwinds the current activation; host callers
    /// retrieve the diagnostic through `try_*` execution APIs.
    #[must_use]
    pub fn refuse_host_command(&mut self, reason: String) -> Completion<Value> {
        self.retain_host_command_refusal(reason);
        err("")
    }

    pub(in super::super) fn retain_host_command_refusal(
        &mut self,
        reason: String,
    ) -> tcl_runtime_api::NativeExecutionError {
        if let Some(first) = self.execution_refusal.clone() {
            return first;
        }
        let (namespace, namespace_token) = self.native_refusal_namespace();
        let first = tcl_runtime_api::NativeExecutionError::HostCommandRefusal(Box::new(
            tcl_runtime_api::NativeHostCommandRefusal {
                reason,
                source_profile: self.expression_source_profile().cache_key(),
                native_profile: self.native_execution_profile().cache_key(),
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: self.owner_nonce,
                    interpreter: u64::try_from(self.cur.0).expect("interpreter slot fits u64"),
                },
                frame: self.current_level(),
                namespace,
                namespace_token,
            },
        ));
        self.retain_native_execution_refusal(first)
    }

    pub(crate) fn refused_completion(&self) -> Option<Completion<Value>> {
        self.execution_refusal.as_ref().map(|_| err(""))
    }

    pub(crate) fn finish_host_execution(
        &mut self,
        completion: Completion<Value>,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.install_deferred_pin();
        match self.take_host_boundary_refusal() {
            Some(error) => Err(error),
            None => Ok(completion),
        }
    }

    /// Return the first original host cause; only the outer host boundary retires it.
    pub(crate) fn take_host_boundary_refusal(
        &mut self,
    ) -> Option<tcl_runtime_api::NativeExecutionError> {
        let error = self.execution_refusal.clone()?;
        if self.activation_depth == 0
            && self.host_execution_depth == 0
            && !self.has_entered_native_operation()
        {
            self.execution_refusal = None;
            self.native_operation_context.reset_allocations();
            for slot in &self.interps {
                if let Some(state) = &slot.parked {
                    state.native_operation_context.reset_allocations();
                }
            }
        }
        Some(error)
    }
}
