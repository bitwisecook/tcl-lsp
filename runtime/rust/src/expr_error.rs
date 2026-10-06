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

//! Exact guest-error bytes and retained host refusal transport.
//!
//! These carriers remain available when the expression backend is absent;
//! constructing or reporting them never evaluates an expression.

/// An expr-evaluation error: Tcl's verbatim message bytes plus an optional
/// `-errorcode` (a pre-formatted list, e.g. `ARITH DIVZERO {divide by zero}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    pub msg: Vec<u8>,
    pub code: Option<Vec<u8>>,
    /// Native rejected Expression(NULL) returns error without replacing result.
    pub(crate) preserve_result: bool,
    /// Host-only operational refusal, independent of guest message/code bytes.
    pub native_access_refusal: Option<tcl_syntax::raw_string::NativeValueAccessRefusal>,
    /// Selected syntax or invalid-type state updates, excluding arithmetic failures.
    /// Reached primitive result producer, independent of diagnostic spelling.
    pub(crate) string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
    pub(crate) error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

impl ExprError {
    pub(crate) fn msg(s: &[u8]) -> ExprError {
        ExprError {
            msg: s.to_vec(),
            code: None,
            preserve_result: false,
            native_access_refusal: None,
            string_result: None,
            error_stage: None,
        }
    }
    /// An error from owned message bytes (no `-errorcode`).
    pub fn from_bytes(m: Vec<u8>) -> ExprError {
        ExprError {
            msg: m,
            code: None,
            preserve_result: false,
            native_access_refusal: None,
            string_result: None,
            error_stage: None,
        }
    }
    /// An error from message bytes plus an optional `-errorcode` (an empty code
    /// is treated as none).
    pub fn from_parts(m: Vec<u8>, code: Vec<u8>) -> ExprError {
        ExprError {
            msg: m,
            code: (!code.is_empty()).then_some(code),
            preserve_result: false,
            native_access_refusal: None,
            string_result: None,
            error_stage: None,
        }
    }
    /// An error with an explicit `-errorcode`.
    pub(crate) fn with_code(m: &[u8], code: &[u8]) -> ExprError {
        ExprError {
            msg: m.to_vec(),
            code: Some(code.to_vec()),
            preserve_result: false,
            native_access_refusal: None,
            string_result: None,
            error_stage: None,
        }
    }

    pub(crate) fn with_numeric_string_result84(mut self) -> Self {
        self.string_result = Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V8_4,
        ));
        self
    }

    pub(crate) fn retained_result() -> Self {
        let mut error = Self::msg(b"");
        error.preserve_result = true;
        error
    }

    pub(crate) fn with_invalid_type_stage(
        mut self,
        dialect: tcl_registry::InvocationDialect,
        stage: tcl_registry::native_numeric_error::NativeExpressionOperandStage,
    ) -> Self {
        self.error_stage = self.code.as_deref().and_then(|code| {
            dialect
                .expression_invalid_type_error_code(stage, code)
                .map(|stage| Box::new(stage.into()))
        });
        self
    }

    pub(crate) fn syntax(
        message: Vec<u8>,
        code: Option<Vec<u8>>,
        syntax: tcl_syntax::expr::parser::NativeExprSyntax,
    ) -> Self {
        let Some(stage) = tcl_registry::native_expression_error::NativeExpressionErrorStage::syntax(
            syntax,
            code.as_deref(),
        ) else {
            return Self::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression syntax error state",
                ),
            );
        };
        Self {
            msg: message,
            code,
            preserve_result: false,
            native_access_refusal: None,
            string_result: None,
            error_stage: Some(Box::new(stage)),
        }
    }

    pub(crate) fn host_refusal(error: tcl_syntax::raw_string::NativeValueAccessRefusal) -> Self {
        Self {
            msg: Vec::new(),
            code: None,
            preserve_result: false,
            native_access_refusal: Some(error),
            string_result: None,
            error_stage: None,
        }
    }
}

impl crate::interp::Interp {
    /// Inspect host-only failure before publishing a guest completion.
    pub(crate) fn report_expr_error(&mut self, error: ExprError) -> crate::interp::Code {
        if self.host_refusal_pending() {
            return crate::interp::Code::Error;
        }
        if let Some(refusal) = error.native_access_refusal {
            return self.refuse_native_access(refusal);
        }
        if error.preserve_result {
            return crate::interp::Code::Error;
        }
        if let Some(stage) = error.error_stage {
            let error_code = stage.code_update(
                tcl_registry::native_expression_error::ExpressionErrorPublication::Direct,
            );
            let code = self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
                tcl_cmd_core::CmdErrorDetails {
                    string_result: None,
                    message: error.msg,
                    error_code,
                    error_info: None,
                    error_line: None,
                    primitive_getter: None,
                },
            ));
            self.retain_expression_error_stage(stage);
            return code;
        }
        if let Some(protocol) = error.string_result {
            let error = match error.code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(error.msg, code),
                None => tcl_cmd_core::CmdError::new_bytes(error.msg),
            }
            .with_native_string_result(protocol);
            return self.report_cmd_error(error);
        }
        match error.code {
            Some(code) => self.error_with_code(&error.msg, &code),
            None => self.set_error(&error.msg),
        }
    }
}
