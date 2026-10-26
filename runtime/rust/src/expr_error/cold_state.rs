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

//! Cold expression obligations keep the public error fields and diagnostics intact.

use super::ExprError;
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ExprErrorState {
    pub(super) command_error: Option<Box<tcl_cmd_core::CmdError>>,
    pub(super) preserve_result: bool,
    pub(super) string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
    pub(super) error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

impl ExprError {
    pub(super) fn state_mut(&mut self) -> &mut ExprErrorState {
        self.state.get_or_insert_with(Default::default)
    }

    #[cfg(all(test, have_tommath))]
    pub(crate) fn retained_command_error(&self) -> Option<&tcl_cmd_core::CmdError> {
        self.state.as_ref()?.command_error.as_deref()
    }
}

impl fmt::Debug for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let empty = ExprErrorState::default();
        let state = self.state.as_deref().unwrap_or(&empty);
        f.debug_struct("ExprError")
            .field("msg", &self.msg)
            .field("command_error", &state.command_error)
            .field("code", &self.code)
            .field("preserve_result", &state.preserve_result)
            .field("native_access_refusal", &self.native_access_refusal)
            .field("native_execution_refusal", &self.native_execution_refusal)
            .field("string_result", &state.string_result)
            .field("error_stage", &state.error_stage)
            .finish()
    }
}

impl PartialEq for ExprError {
    fn eq(&self, other: &Self) -> bool {
        let empty = ExprErrorState::default();
        self.msg == other.msg
            && self.code == other.code
            && self.native_access_refusal == other.native_access_refusal
            && self.native_execution_refusal == other.native_execution_refusal
            && self.state.as_deref().unwrap_or(&empty) == other.state.as_deref().unwrap_or(&empty)
    }
}

impl Eq for ExprError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cold_storage_preserves_flattened_debug_equality_and_guest_bytes() {
        let original = ExprError::from_parts(vec![0, 255], vec![b'X']);
        let mut with_storage = original.clone();
        let _ = with_storage.state_mut();
        assert_eq!(original, with_storage);
        assert_eq!(original.msg, [0, 255]);
        assert_eq!(original.code, Some(vec![b'X']));
        assert_eq!(format!("{original:?}"), format!("{with_storage:?}"));
        assert_eq!(
            format!("{original:?}"),
            "ExprError { msg: [0, 255], command_error: None, code: Some([88]), preserve_result: false, native_access_refusal: None, native_execution_refusal: None, string_result: None, error_stage: None }"
        );
        assert!(std::mem::size_of::<ExprError>() < 128);
    }

    #[test]
    fn cloned_cold_state_retains_complete_typed_refusal_and_public_fields() {
        let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retained expression first cause",
            ),
        );
        let mut error = ExprError::from_execution_refusal(cause.clone());
        error.msg = vec![0, 255];
        error.code = Some(vec![b'X']);
        error.state_mut().preserve_result = true;
        let cloned = error.clone();
        assert_eq!(cloned, error);
        assert_eq!(cloned.native_execution_refusal, Some(cause));
        assert_eq!(cloned.msg, [0, 255]);
        assert_eq!(cloned.code, Some(vec![b'X']));
        assert!(cloned.state.as_ref().unwrap().preserve_result);
        assert_eq!(format!("{cloned:?}"), format!("{error:?}"));
    }
}
