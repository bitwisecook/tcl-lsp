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

//! [`CmdError`] — the command-level error the portable helpers return.
//!
//! A command helper computes `Result<V, CmdError>`; the per-runtime adapter maps
//! `CmdError` onto that runtime's protocol (`Completion<Value>` for the VM,
//! `interp.set_result(...) + Code` for the WASM runtime). `CmdError` is the home
//! of the **canonical Tcl error-message catalogue** — built once here rather
//! than hand-assembled per runtime — and the closed coercion / host errors
//! ([`tcl_syntax::value::ValueError`], [`tcl_platform::HostError`]) lift into it
//! with a plain `From`.

use tcl_platform::HostError;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::{
    NativeStringAccessError, NativeValueAccessRefusal, UnicodeAccessError,
};
use tcl_syntax::value::ValueError;

use tcl_syntax::scalar_getter::{NativeScalarGetterError, NativeScalarGetterErrorCode};

/// Command adapter action on the actual existing interpreter error-code state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdErrorCodeUpdate {
    /// Neutral command failure: store the exact `NONE` error-code list.
    Default,
    /// Authenticated argument-count failure, independent of result wording.
    /// The consuming interpreter must select its actual wrong-arguments protocol.
    WrongArguments,
    /// Perform no error-code store, including no write traces.
    Unchanged,
    /// Store these exact native list bytes.
    Set(Vec<u8>),
}

/// Interpreter-ready error-code action with semantic selection completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedCmdErrorCodeUpdate {
    /// Keep existing state without a store or trace.
    Unchanged,
    /// Store these exact selected native list bytes.
    Set(Vec<u8>),
}

impl CmdErrorCodeUpdate {
    /// Resolve command identity before changing guest result, options or state.
    /// The provider is requested only for authenticated argument-count failures.
    pub fn resolve(
        self,
        wrong_arguments: impl FnOnce() -> Result<Vec<u8>, NativeValueAccessRefusal>,
    ) -> Result<ResolvedCmdErrorCodeUpdate, NativeValueAccessRefusal> {
        Ok(match self {
            Self::Default => ResolvedCmdErrorCodeUpdate::Set(b"NONE".to_vec()),
            Self::WrongArguments => ResolvedCmdErrorCodeUpdate::Set(wrong_arguments()?),
            Self::Unchanged => ResolvedCmdErrorCodeUpdate::Unchanged,
            Self::Set(bytes) => ResolvedCmdErrorCodeUpdate::Set(bytes),
        })
    }
}

/// Consuming guest fields with every state and propagation obligation retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdErrorDetails {
    /// Exact primitive/command result before propagation.
    pub message: Vec<u8>,
    /// Action on actual interpreter state, not an optional defaulted code.
    pub error_code: CmdErrorCodeUpdate,
    /// Already accumulated callback trace, if any.
    pub error_info: Option<Vec<u8>>,
    /// Retained native error line.
    pub error_line: Option<i64>,
    /// Selected primitive stage/origin and its separate Eval projection.
    pub primitive_getter: Option<Box<NativeScalarGetterError>>,
    /// Actual append/format producer; absence retains a fresh NULL-primary result.
    pub string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
}

type UnicodeErrorDetails = (
    String,
    CmdErrorCodeUpdate,
    Option<Vec<u8>>,
    Option<i64>,
    Option<Box<NativeScalarGetterError>>,
);

/// A failed Tcl command: its result message and optional structured error
/// metadata.
///
/// The shared command core owns the semantic error identity; each runtime
/// adapter publishes it through its native completion/error state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdError {
    message: Vec<u8>,
    metadata: Box<CmdErrorMetadata>,
}

/// Cold completion obligations stay off every successful command's stack frame.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CmdErrorMetadata {
    native_execution_refusal: Option<NativeExecutionError>,
    string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
    error_code: CmdErrorCodeUpdate,
    primitive_getter: Option<Box<NativeScalarGetterError>>,
    error_info: Option<Vec<u8>>,
    error_line: Option<i64>,
}

impl CmdError {
    /// A command error with the given Tcl result message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into().into_bytes(),
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: None,
                error_code: CmdErrorCodeUpdate::Default,
                primitive_getter: None,
                error_info: None,
                error_line: None,
            }),
        }
    }

    /// A command error carrying Tcl's structured `-errorcode` list.
    pub fn with_error_code(message: impl Into<String>, error_code: impl Into<String>) -> Self {
        Self::with_error_code_bytes(message.into().into_bytes(), error_code.into().into_bytes())
    }

    /// Byte-exact guest result and structured error-code operands.
    pub fn with_error_code_bytes(
        message: impl Into<Vec<u8>>,
        error_code: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            message: message.into(),
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: None,
                error_code: CmdErrorCodeUpdate::Set(error_code.into()),
                primitive_getter: None,
                error_info: None,
                error_line: None,
            }),
        }
    }

    /// A command error carrying the already-accumulated callback error trace.
    #[must_use]
    pub fn with_error_details(
        message: impl Into<String>,
        error_code: impl Into<String>,
        error_info: Option<Vec<u8>>,
        error_line: Option<i64>,
    ) -> Self {
        Self {
            message: message.into().into_bytes(),
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: None,
                error_code: CmdErrorCodeUpdate::Set(error_code.into().into_bytes()),
                primitive_getter: None,
                error_info,
                error_line,
            }),
        }
    }

    /// A byte-exact guest command error, without requiring a Unicode view.
    #[must_use]
    pub fn new_bytes(message: impl Into<Vec<u8>>) -> Self {
        Self {
            message: message.into(),
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: None,
                error_code: CmdErrorCodeUpdate::Default,
                primitive_getter: None,
                error_info: None,
                error_line: None,
            }),
        }
    }

    /// Retain the actual append/format String producer independently of error wording.
    #[must_use]
    pub fn with_native_string_result(
        mut self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self {
        self.metadata.string_result = Some(protocol);
        self
    }

    /// Exact native guest result bytes. Inspect the refusal tag before publishing.
    #[must_use]
    pub fn message_bytes(&self) -> &[u8] {
        &self.message
    }

    /// A retained operational refusal, which must bypass guest catch/finally.
    #[must_use]
    pub fn unicode_refusal(&self) -> Option<UnicodeAccessError> {
        match self.native_access_refusal() {
            Some(NativeValueAccessRefusal::Unicode(error)) => Some(error),
            _ => None,
        }
    }

    /// Host-only access failure; inspect this before publishing result/options.
    #[must_use]
    pub fn native_access_refusal(&self) -> Option<NativeValueAccessRefusal> {
        match &self.metadata.native_execution_refusal {
            Some(NativeExecutionError::ValueAccessRefusal(error)) => Some(*error),
            _ => None,
        }
    }

    /// Complete original host failure; adapters inspect this before guest fields.
    #[must_use]
    pub fn native_execution_refusal(&self) -> Option<&NativeExecutionError> {
        self.metadata.native_execution_refusal.as_ref()
    }

    /// Retain an actual operational failure without manufacturing a Tcl error.
    #[must_use]
    pub fn from_execution_refusal(error: NativeExecutionError) -> Self {
        let mut refusal = Self::new_bytes(Vec::new());
        refusal.metadata.native_execution_refusal = Some(error);
        refusal
    }

    /// Consume guest fields after inspecting `native_execution_refusal`.
    /// This positive projection does not transport a host failure.
    #[must_use]
    pub fn into_byte_details(self) -> CmdErrorDetails {
        CmdErrorDetails {
            message: self.message,
            error_code: self.metadata.error_code,
            error_info: self.metadata.error_info,
            error_line: self.metadata.error_line,
            primitive_getter: self.metadata.primitive_getter,
            string_result: self.metadata.string_result,
        }
    }

    /// Rebuild a guest error while retaining its state and propagation record.
    /// The caller must preserve any separately inspected host-refusal tag.
    #[must_use]
    pub fn from_byte_details(details: CmdErrorDetails) -> Self {
        Self {
            message: details.message,
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: details.string_result,
                error_code: details.error_code,
                error_info: details.error_info,
                error_line: details.error_line,
                primitive_getter: details.primitive_getter,
            }),
        }
    }

    /// A byte-exact guest error carrying accumulated structured metadata.
    #[must_use]
    pub fn with_byte_error_details(
        message: Vec<u8>,
        error_code: Vec<u8>,
        error_info: Option<Vec<u8>>,
        error_line: Option<i64>,
    ) -> Self {
        Self {
            message,
            metadata: Box::new(CmdErrorMetadata {
                native_execution_refusal: None,
                string_result: None,
                error_code: CmdErrorCodeUpdate::Set(error_code),
                primitive_getter: None,
                error_info,
                error_line,
            }),
        }
    }

    /// The checked Unicode view of the Tcl error message.
    pub fn message(&self) -> Result<&str, NativeExecutionError> {
        if let Some(error) = &self.metadata.native_execution_refusal {
            return Err(error.clone());
        }
        std::str::from_utf8(&self.message)
            .map_err(|error| NativeExecutionError::ValueAccessRefusal(unicode_error(error).into()))
    }

    /// Tcl's structured `-errorcode` list, when the command supplied one.
    pub fn error_code(&self) -> Result<Option<&str>, NativeExecutionError> {
        if let Some(error) = &self.metadata.native_execution_refusal {
            return Err(error.clone());
        }
        self.error_code_bytes()
            .map(std::str::from_utf8)
            .transpose()
            .map_err(|error| NativeExecutionError::ValueAccessRefusal(unicode_error(error).into()))
    }

    /// Positive explicit error-code bytes. Absence does not distinguish Default
    /// from Unchanged; adapters must consume the full state-update record.
    #[must_use]
    pub fn error_code_bytes(&self) -> Option<&[u8]> {
        match &self.metadata.error_code {
            CmdErrorCodeUpdate::Set(bytes) => Some(bytes),
            CmdErrorCodeUpdate::Default
            | CmdErrorCodeUpdate::Unchanged
            | CmdErrorCodeUpdate::WrongArguments => None,
        }
    }

    /// Complete state action, preserving the native no-store branch.
    #[must_use]
    pub fn error_code_update(&self) -> &CmdErrorCodeUpdate {
        &self.metadata.error_code
    }

    /// Positive result-only projection. This deliberately omits state and
    /// propagation obligations; completion adapters must use `into_byte_details`.
    pub fn into_message(self) -> Result<String, NativeExecutionError> {
        if let Some(error) = &self.metadata.native_execution_refusal {
            return Err(error.clone());
        }
        String::from_utf8(self.message).map_err(|error| {
            NativeExecutionError::ValueAccessRefusal(unicode_error(error.utf8_error()).into())
        })
    }

    /// Consume checked Unicode result bytes while retaining the complete code
    /// update and primitive propagation record.
    pub fn into_parts(
        self,
    ) -> Result<
        (
            String,
            CmdErrorCodeUpdate,
            Option<Box<NativeScalarGetterError>>,
        ),
        NativeExecutionError,
    > {
        let (message, code, _, _, primitive) = self.into_details()?;
        Ok((message, code, primitive))
    }

    /// Consume the error, including any accumulated callback trace metadata.
    pub fn into_details(self) -> Result<UnicodeErrorDetails, NativeExecutionError> {
        if let Some(error) = &self.metadata.native_execution_refusal {
            return Err(error.clone());
        }
        Ok((
            String::from_utf8(self.message).map_err(|error| {
                NativeExecutionError::ValueAccessRefusal(unicode_error(error.utf8_error()).into())
            })?,
            self.metadata.error_code,
            self.metadata.error_info,
            self.metadata.error_line,
            self.metadata.primitive_getter,
        ))
    }

    /// `wrong # args: should be "…"` — the canonical Tcl arity error.
    #[must_use]
    pub fn wrong_args(usage: &str) -> Self {
        Self::wrong_args_bytes(usage.as_bytes())
    }

    /// Native arity message retaining the presenter's exact byte header.
    #[must_use]
    pub fn wrong_args_bytes(usage: &[u8]) -> Self {
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(usage);
        message.push(b'"');
        Self::wrong_arguments_message_bytes(message)
    }

    /// Retain the semantic receipt for an already rendered native arity message.
    /// Call only at an argument-count check, never by inspecting guest text.
    #[must_use]
    pub fn wrong_arguments_message_bytes(message: impl Into<Vec<u8>>) -> Self {
        let mut error = Self::new_bytes(message);
        error.metadata.error_code = CmdErrorCodeUpdate::WrongArguments;
        error
    }

    /// `bad <what> "<got>": must be <choices>` — the canonical Tcl
    /// bad-option/subcommand error.
    #[must_use]
    pub fn bad_choice(what: &str, got: &str, choices: &str) -> Self {
        Self::new(format!("bad {what} \"{got}\": must be {choices}"))
    }

    /// A Tcl list syntax failure, preserving the list owner's message and
    /// structured error code.
    #[must_use]
    pub fn list(error: tcl_syntax::list::ListError, source: &str) -> Self {
        Self::with_error_code(error.full_message(source), error.error_code())
    }

    /// A failed `Tcl_GetIndexFromObj`-style lookup.
    #[must_use]
    pub fn lookup_index(message: impl Into<String>, what: &str, word: &str) -> Self {
        Self::lookup_index_bytes(
            message.into().into_bytes(),
            what.as_bytes(),
            word.as_bytes(),
        )
    }

    /// A table-lookup diagnostic retaining its native word and result bytes.
    #[must_use]
    pub fn lookup_index_bytes(message: impl Into<Vec<u8>>, what: &[u8], word: &[u8]) -> Self {
        let mut code = b"TCL LOOKUP INDEX ".to_vec();
        tcl_syntax::list::append_list_element(&mut code, what, true);
        code.push(b' ');
        tcl_syntax::list::append_list_element(&mut code, word, true);
        Self::with_error_code_bytes(message, code)
    }

    /// A command argument whose shape is invalid after Tcl list parsing.
    #[must_use]
    pub fn argument_format(message: impl Into<String>) -> Self {
        Self::with_error_code(message, "TCL ARGUMENT FORMAT")
    }

    /// A trace-aware variable read whose selected cell disappeared.
    #[must_use]
    pub fn variable_read_missing(name: &str, reason: &str) -> Self {
        Self::with_error_code(
            format!("can't read \"{name}\": {reason}"),
            "TCL READ VARNAME",
        )
    }
}

impl core::fmt::Display for CmdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(error) = self.native_execution_refusal() {
            return error.fmt(f);
        }
        match self.message() {
            Ok(message) => f.write_str(message),
            Err(_) => write!(f, "native byte command error: {:?}", self.message),
        }
    }
}

impl std::error::Error for CmdError {}

impl From<ValueError> for CmdError {
    fn from(e: ValueError) -> Self {
        if let ValueError::NativeListParse {
            error,
            source,
            protocol,
        } = &e
        {
            use tcl_dialect::TclVersion;
            use tcl_syntax::native_string::NativeStringProtocol;
            let mut result = Self::new_bytes(error.full_message_bytes(source));
            if matches!(
                protocol,
                NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            ) {
                result.metadata.error_code =
                    CmdErrorCodeUpdate::Set(error.error_code().as_bytes().to_vec());
            }
            if matches!(
                protocol,
                NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            ) || matches!(protocol, NativeStringProtocol::C(TclVersion::V8_5))
                && matches!(
                    error,
                    tcl_syntax::list::ListError::BraceFollowedByJunk
                        | tcl_syntax::list::ListError::QuoteFollowedByJunk
                )
            {
                result = result.with_native_string_result(*protocol);
            }
            return result;
        }
        if let ValueError::NativeScalarGetter(record) = e {
            let update = match record.error_code_update() {
                NativeScalarGetterErrorCode::Unchanged => CmdErrorCodeUpdate::Unchanged,
                NativeScalarGetterErrorCode::Set(bytes) => CmdErrorCodeUpdate::Set(bytes.clone()),
            };
            return Self::from_byte_details(CmdErrorDetails {
                string_result: None,
                message: record.message_bytes().to_vec(),
                error_code: update,
                error_info: None,
                error_line: None,
                primitive_getter: Some(record),
            });
        }
        if let Some(error) = e.native_access_refusal() {
            Self::from(error)
        } else {
            let code = match &e {
                ValueError::ListParse { error, .. } => Some(error.error_code().to_owned()),
                ValueError::DictionaryParse { error, .. } => {
                    Some(error.error_code().replace(" LIST", " DICTIONARY"))
                }
                ValueError::MissingDictionaryValue => Some("TCL VALUE DICTIONARY".to_owned()),
                _ => None,
            };
            let mut result = Self::new_bytes(e.message_bytes());
            result.metadata.error_code = code.map_or(CmdErrorCodeUpdate::Default, |code| {
                CmdErrorCodeUpdate::Set(code.into_bytes())
            });
            result
        }
    }
}

fn unicode_error(error: std::str::Utf8Error) -> UnicodeAccessError {
    UnicodeAccessError {
        valid_up_to: error.valid_up_to(),
        error_len: error.error_len(),
    }
}

impl From<UnicodeAccessError> for CmdError {
    fn from(error: UnicodeAccessError) -> Self {
        let mut refusal = Self::new_bytes(Vec::new());
        refusal.metadata.native_execution_refusal =
            Some(NativeExecutionError::ValueAccessRefusal(error.into()));
        refusal
    }
}

impl From<NativeValueAccessRefusal> for CmdError {
    fn from(error: NativeValueAccessRefusal) -> Self {
        let mut refusal = Self::new_bytes(Vec::new());
        refusal.metadata.native_execution_refusal =
            Some(NativeExecutionError::ValueAccessRefusal(error));
        refusal
    }
}
impl From<NativeStringAccessError> for CmdError {
    fn from(error: NativeStringAccessError) -> Self {
        Self::from(NativeValueAccessRefusal::from(error))
    }
}

impl From<HostError> for CmdError {
    fn from(e: HostError) -> Self {
        // The bare reason; contextual helpers (e.g. `open`) build the richer
        // `couldn't open "<path>": <reason>` form themselves.
        Self::new(e.reason())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_message_and_authenticated_arity_have_distinct_receipts() {
        let authentic = CmdError::wrong_args_bytes(b"n\0\xc0\x80\xff arg");
        let neutral = CmdError::new_bytes(authentic.message_bytes());
        assert_eq!(neutral.message_bytes(), authentic.message_bytes());
        assert_eq!(neutral.error_code_update(), &CmdErrorCodeUpdate::Default);
        assert_eq!(
            authentic.error_code_update(),
            &CmdErrorCodeUpdate::WrongArguments
        );
        assert_eq!(
            neutral
                .into_byte_details()
                .error_code
                .resolve(|| panic!("neutral error requests no provider"))
                .unwrap(),
            ResolvedCmdErrorCodeUpdate::Set(b"NONE".to_vec())
        );
        assert!(
            authentic
                .into_byte_details()
                .error_code
                .resolve(|| Err(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "wrong arguments"
                )))
                .is_err()
        );
    }

    #[test]
    fn primitive_failure_retains_origin_no_store_and_separate_eval_bytes() {
        use tcl_syntax::scalar_getter::{
            NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
        };
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_5);
        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5\0suffix",
            )
            .unwrap();
        let error = CmdError::from(ValueError::NativeScalarGetter(Box::new(record)));
        assert_eq!(error.error_code_update(), &CmdErrorCodeUpdate::Unchanged);
        let details = error.into_byte_details();
        assert_eq!(details.message, b"expected integer but got \"1.5\0suffix\"");
        let record = details.primitive_getter.unwrap();
        assert_eq!(record.protocol(), protocol);
        assert_eq!(record.getter_kind(), NativeScalarGetterKind::Wide);
        assert_eq!(
            record.failure_origin(),
            NativeScalarGetterFailure::CachedNonInteger
        );
        assert_eq!(
            record.eval_message_bytes(),
            b"expected integer but got \"1.5"
        );
        assert_eq!(
            record.eval_result_bytes(b"context: expected integer but got \"1.5\0suffix\""),
            b"context: expected integer but got \"1.5"
        );
        assert_eq!(details.error_code, CmdErrorCodeUpdate::Unchanged);
        assert_eq!(
            CmdError::new("default").into_byte_details().error_code,
            CmdErrorCodeUpdate::Default
        );
    }

    #[test]
    fn byte_error_metadata_remains_guest_data_without_unicode_projection() {
        let error = CmdError::with_byte_error_details(
            b"BOOM\xff".to_vec(),
            b"RAW \xfe".to_vec(),
            Some(b"TRACE\xfd".to_vec()),
            Some(7),
        );
        assert_eq!(error.native_access_refusal(), None);
        assert_eq!(error.message_bytes(), b"BOOM\xff");
        assert_eq!(error.error_code_bytes(), Some(b"RAW \xfe".as_slice()));
        assert!(matches!(
            error.error_code(),
            Err(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::Unicode(_)
            ))
        ));
        assert_eq!(
            error.into_byte_details(),
            CmdErrorDetails {
                string_result: None,
                message: b"BOOM\xff".to_vec(),
                error_code: CmdErrorCodeUpdate::Set(b"RAW \xfe".to_vec()),
                error_info: Some(b"TRACE\xfd".to_vec()),
                error_line: Some(7),
                primitive_getter: None,
            }
        );
    }

    #[test]
    fn result_construction_survives_error_receipt_roundtrip() {
        use tcl_syntax::native_string::NativeStringProtocol;
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let error = CmdError::new("native append").with_native_string_result(protocol);
        let details = error.into_byte_details();
        assert_eq!(details.string_result, Some(protocol));
        assert_eq!(
            CmdError::from_byte_details(details.clone()).into_byte_details(),
            details
        );
        assert_eq!(
            CmdError::new("custom callback")
                .into_byte_details()
                .string_result,
            None
        );
    }

    #[test]
    fn canonical_tcl_error_message_formats() {
        // The canonical Tcl error texts (`Tcl_WrongNumArgs` / bad-option).
        assert_eq!(CmdError::new("boom").message().unwrap(), "boom");
        assert_eq!(
            CmdError::wrong_args("string length string")
                .message()
                .unwrap(),
            r#"wrong # args: should be "string length string""#
        );
        assert_eq!(
            CmdError::bad_choice("option", "foo", "a, b, or c")
                .message()
                .unwrap(),
            r#"bad option "foo": must be a, b, or c"#
        );
        // `Display` mirrors the message, and `into_message` consumes it.
        assert_eq!(
            format!("{}", CmdError::wrong_args("x")),
            r#"wrong # args: should be "x""#
        );
        assert_eq!(CmdError::new("z").into_message().unwrap(), "z");
        let coded = CmdError::with_error_code("constant", "TCL UNSET CONST");
        assert_eq!(coded.error_code().unwrap(), Some("TCL UNSET CONST"));
        assert_eq!(
            coded.into_parts().unwrap(),
            (
                "constant".to_string(),
                CmdErrorCodeUpdate::Set(b"TCL UNSET CONST".to_vec()),
                None
            )
        );
        assert_eq!(
            CmdError::list(tcl_syntax::list::ListError::UnmatchedBrace, "{bad")
                .into_parts()
                .unwrap(),
            (
                "unmatched open brace in list".to_string(),
                CmdErrorCodeUpdate::Set(b"TCL VALUE LIST BRACE".to_vec()),
                None
            )
        );
        assert_eq!(
            CmdError::lookup_index("bad option", "option", "two words")
                .into_parts()
                .unwrap(),
            (
                "bad option".to_string(),
                CmdErrorCodeUpdate::Set(b"TCL LOOKUP INDEX option {two words}".to_vec()),
                None
            )
        );
        assert_eq!(
            CmdError::argument_format("bad shape").error_code().unwrap(),
            Some("TCL ARGUMENT FORMAT")
        );
        assert_eq!(
            CmdError::variable_read_missing("a(k)", "no such variable")
                .into_parts()
                .unwrap(),
            (
                "can't read \"a(k)\": no such variable".to_string(),
                CmdErrorCodeUpdate::Set(b"TCL READ VARNAME".to_vec()),
                None
            )
        );
    }
}
