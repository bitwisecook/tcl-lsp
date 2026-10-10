// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lambda conversion diagnostics; original list, body and cache ownership are separate.

use crate::InvocationDialect;
use tcl_cmd_core::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

/// Pure diagnostics selected from the independently identified native engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLambdaDiagnosticProtocol {
    strings: NativeStringProtocol,
}

impl NativeLambdaDiagnosticProtocol {
    /// C8.4 has no apply; unspecified or foreign engine points decline.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        let strings = dialect.native_string_protocol()?;
        (!matches!(strings, NativeStringProtocol::C(TclVersion::V8_4))).then_some(Self { strings })
    }

    /// Only an actual selected list syntax failure is a lambda conversion error.
    /// Operational getter/capability refusals keep their original outer channel.
    #[must_use]
    pub fn wraps_list_failure(self, failure: &ValueError) -> bool {
        matches!(failure, ValueError::NativeListParse { protocol, .. } if *protocol == self.strings)
    }

    /// Format the failure after the actual original string getter has completed.
    /// C8.5 appends the counted object; later C and Jim format its `CString` view.
    /// This grants neither a result storage class nor successful lambda conversion.
    #[must_use]
    pub fn conversion_error(self, original: &[u8]) -> CmdError {
        let operand = if self.strings == NativeStringProtocol::C(TclVersion::V8_5) {
            original
        } else {
            tcl_core_types::c_string_extent(original)
        };
        let mut message = b"can't interpret \"".to_vec();
        message.extend_from_slice(operand);
        message.extend_from_slice(b"\" as a lambda expression");
        if matches!(self.strings, NativeStringProtocol::C(version) if version >= TclVersion::V8_6) {
            CmdError::with_error_code_bytes(message, b"TCL VALUE LAMBDA".to_vec())
        } else {
            CmdError::new_bytes(message)
        }
    }

    /// C appends this `CString` printf frame after a formal-parameter parse error.
    /// Jim uses its own procedure error-stack path and has no C lambda frame.
    #[must_use]
    pub fn parameter_error_frame(self, original: &[u8]) -> Option<Vec<u8>> {
        self.strings.tcl_version()?;
        let mut frame = b"\n    (parsing lambda expression \"".to_vec();
        frame.extend_from_slice(tcl_core_types::c_string_extent(original));
        frame.extend_from_slice(b"\")");
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambda_diagnostics_keep_conversion_and_parameter_frame_purposes_separate() {
        // Native proof: naming.lambda.original-conversion-diagnostic
        // docs/design/analysis/name-resolution-proofs/lambda-original-conversion-diagnostic.md
        // Native proof: naming.lambda.original-parameter-error-frame
        // docs/design/analysis/name-resolution-proofs/lambda-original-parameter-error-frame.md
        for version in TclVersion::ALL {
            let selected =
                NativeLambdaDiagnosticProtocol::select(InvocationDialect::for_version(version));
            assert_eq!(selected.is_some(), version >= TclVersion::V8_5);
            let Some(selected) = selected else { continue };
            let error = selected.conversion_error(b"opaque\0tail");
            let expected = if version == TclVersion::V8_5 {
                b"can't interpret \"opaque\0tail\" as a lambda expression".as_slice()
            } else {
                b"can't interpret \"opaque\" as a lambda expression".as_slice()
            };
            assert_eq!(error.message_bytes(), expected);
            assert_eq!(
                error.error_code_bytes(),
                (version >= TclVersion::V8_6).then_some(b"TCL VALUE LAMBDA".as_slice())
            );
            assert_eq!(
                selected
                    .parameter_error_frame(b"{{a b c}} {x\0TAIL}")
                    .unwrap(),
                b"\n    (parsing lambda expression \"{{a b c}} {x\")"
            );
            for original in [
                b"opaque\xc0\x80tail".as_slice(),
                b"opaque\xfftail",
                b"opaque\xed\xa0\x80",
            ] {
                let message = selected.conversion_error(original);
                assert!(
                    message
                        .message_bytes()
                        .windows(original.len())
                        .any(|part| part == original)
                );
            }
            let failure = ValueError::NativeListParse {
                error: tcl_syntax::list::ListError::UnmatchedBrace,
                source: b"{".to_vec(),
                protocol: NativeStringProtocol::C(version),
            };
            assert!(selected.wraps_list_failure(&failure));
            assert!(
                !selected
                    .wraps_list_failure(&ValueError::CommandProtocolUnavailable("original list"))
            );
            assert!(!selected.wraps_list_failure(&ValueError::BadList("logical parser".into())));
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let selected = NativeLambdaDiagnosticProtocol::select(jim).unwrap();
        assert_eq!(
            selected.conversion_error(b"opaque\0tail").message_bytes(),
            b"can't interpret \"opaque\" as a lambda expression"
        );
        assert!(
            selected
                .parameter_error_frame(b"{{a b c}} {x\0TAIL}")
                .is_none()
        );
        let mut unknown = InvocationDialect::for_version(TclVersion::V8_6);
        unknown.core_point = None;
        unknown.native_family = None;
        unknown.tcl_version = None;
        assert!(NativeLambdaDiagnosticProtocol::select(unknown).is_none());
        let mut mismatched = InvocationDialect::for_version(TclVersion::V8_6);
        mismatched.core_point = Some(tcl_dialect::model::DialectPoint::for_tcl_version(
            TclVersion::V8_6,
        ));
        mismatched.tcl_version = Some(TclVersion::V9_1);
        assert!(NativeLambdaDiagnosticProtocol::select(mismatched).is_none());
    }
}
