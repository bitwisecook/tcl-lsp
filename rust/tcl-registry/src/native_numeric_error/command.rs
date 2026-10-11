// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected public command conversions, independent of operand currency.

use tcl_dialect::TclVersion;
use tcl_syntax::scalar_getter::NativeScalarGetterKind;

/// Reached stock command argument, separate from expression arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNumericCommandPurpose {
    /// Optional channel read count.
    ReadCount,
    /// Channel seek displacement.
    SeekOffset,
    /// Base64 encoder line length.
    Base64LineLength,
    /// Uuencoder line length.
    UuencodeLineLength,
    /// Interpreter debug frame switch.
    DebugFrame,
    /// Original list search group length.
    ListSearchStride,
    /// Original list sort group length.
    ListSortStride,
}

/// Actual read count dispatch and failed-getter continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeReadCountProtocol {
    /// Obtain String first; only a leading digit enters Int, otherwise accept
    /// the old nonewline suffix or report the original bad argument.
    LeadingDigitOrSuffix,
    /// Int failure accepts the old nonewline suffix; other failures propagate.
    IntOrSuffix,
    /// Int failure or negative value accepts the old nonewline suffix or
    /// publishes the nonnegative wrapper diagnostic.
    NonnegativeIntOrSuffix,
    /// Error-neutral Wide conversion; failure or negative value publishes
    /// the nonnegative wrapper diagnostic, with no suffix alternative.
    QuietNonnegativeWide,
    /// Jim AIO's leading-hyphen option dispatch before Wide conversion.
    JimAio,
}

/// Command-owned diagnostic after the selected primitive stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNumericCommandError {
    /// Exact authored result bytes.
    pub message: Vec<u8>,
    /// Exact authored error-code list bytes.
    pub error_code: Vec<u8>,
}

/// Selected argument recipe. This metadata supplies no object, callback,
/// engine lifetime, successful conversion or backend I/O permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeNumericCommandProtocol {
    version: Option<TclVersion>,
    purpose: NativeNumericCommandPurpose,
}

impl crate::InvocationDialect {
    /// Select the command's actual primitive and continuation. Unsupported
    /// stock commands and absent or foreign native axes remain unavailable.
    #[must_use]
    pub fn native_numeric_command_protocol(
        self,
        purpose: NativeNumericCommandPurpose,
    ) -> Option<NativeNumericCommandProtocol> {
        use NativeNumericCommandPurpose as Purpose;
        let scalar = self.native_scalar_getter_protocol()?;
        let version = scalar.tcl_version();
        if version.is_none() && !scalar.is_jim084() {
            return None;
        }
        match purpose {
            Purpose::ListSearchStride
                if matches!(
                    version,
                    Some(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6)
                ) =>
            {
                return None;
            }
            Purpose::ListSortStride
                if matches!(version, Some(TclVersion::V8_4 | TclVersion::V8_5)) =>
            {
                return None;
            }
            Purpose::Base64LineLength | Purpose::UuencodeLineLength
                if !matches!(
                    version,
                    Some(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
                ) =>
            {
                return None;
            }
            Purpose::DebugFrame
                if !matches!(
                    version,
                    Some(TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
                ) =>
            {
                return None;
            }
            _ => {}
        }
        Some(NativeNumericCommandProtocol { version, purpose })
    }
}

impl NativeNumericCommandProtocol {
    /// Actual primitive output width and Boolean entry point.
    #[must_use]
    pub const fn getter_kind(self) -> NativeScalarGetterKind {
        use NativeNumericCommandPurpose as Purpose;
        use NativeScalarGetterKind as Kind;
        match self.purpose {
            Purpose::DebugFrame => Kind::Boolean,
            Purpose::ListSearchStride | Purpose::ListSortStride if self.version.is_none() => {
                Kind::Long
            }
            Purpose::ListSortStride if matches!(self.version, Some(TclVersion::V8_6)) => Kind::Int,
            Purpose::ListSearchStride | Purpose::ListSortStride => Kind::Wide,
            Purpose::SeekOffset => Kind::Wide,
            Purpose::ReadCount
                if matches!(
                    self.version,
                    Some(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6)
                ) =>
            {
                Kind::Int
            }
            Purpose::ReadCount => Kind::Wide,
            Purpose::Base64LineLength
                if matches!(self.version, Some(TclVersion::V9_0 | TclVersion::V9_1)) =>
            {
                Kind::Wide
            }
            Purpose::Base64LineLength | Purpose::UuencodeLineLength => Kind::Int,
        }
    }

    /// Read-only dispatch facts for a genuine read-count purpose.
    #[must_use]
    pub const fn read_count(self) -> Option<NativeReadCountProtocol> {
        if !matches!(self.purpose, NativeNumericCommandPurpose::ReadCount) {
            return None;
        }
        Some(match self.version {
            Some(TclVersion::V8_4) => NativeReadCountProtocol::LeadingDigitOrSuffix,
            Some(TclVersion::V8_5) => NativeReadCountProtocol::IntOrSuffix,
            Some(TclVersion::V8_6) => NativeReadCountProtocol::NonnegativeIntOrSuffix,
            Some(TclVersion::V9_0 | TclVersion::V9_1) => {
                NativeReadCountProtocol::QuietNonnegativeWide
            }
            None => NativeReadCountProtocol::JimAio,
        })
    }

    /// Actual Jim leading-hyphen read table; other read recipes have no table.
    #[must_use]
    pub const fn read_leading_options(self) -> Option<&'static [&'static str]> {
        if matches!(self.purpose, NativeNumericCommandPurpose::ReadCount) && self.version.is_none()
        {
            Some(&["-pending", "-nonewline"])
        } else {
            None
        }
    }

    /// Newline effect of a selected member of the actual read option table.
    #[must_use]
    pub const fn read_option_suppresses_newline(self, index: usize) -> Option<bool> {
        if self.read_leading_options().is_none() {
            return None;
        }
        match index {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    /// Jim checks the exact origin before offset conversion; C checks it after.
    #[must_use]
    pub const fn seek_origin_first(self) -> bool {
        matches!(self.purpose, NativeNumericCommandPurpose::SeekOffset) && self.version.is_none()
    }

    /// Jim's AIO origin uses exact comparisons, not C's prefix index table.
    #[must_use]
    pub const fn seek_origin_is_exact(self) -> bool {
        self.seek_origin_first()
    }

    /// Authored nonnegative-count rejection; primitive failures stay separate.
    #[must_use]
    pub fn negative_read_error(self, original: &[u8]) -> Option<NativeNumericCommandError> {
        if !matches!(self.purpose, NativeNumericCommandPurpose::ReadCount)
            || matches!(self.version, Some(TclVersion::V8_4))
        {
            return None;
        }
        let (message, error_code) = if self.version.is_none() {
            (
                b"invalid parameter: negative len".to_vec(),
                b"NONE".to_vec(),
            )
        } else {
            let mut message = b"expected non-negative integer but got \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(original));
            message.push(b'"');
            (message, b"TCL VALUE NUMBER".to_vec())
        };
        Some(NativeNumericCommandError {
            message,
            error_code,
        })
    }

    /// Actual command minimum after its selected primitive succeeds.
    /// C has a structured command code; Jim supplies its own NULL-primary text.
    #[must_use]
    pub fn invalid_list_stride_error(self, integer: i64) -> Option<NativeNumericCommandError> {
        let (minimum, command) = match self.purpose {
            NativeNumericCommandPurpose::ListSearchStride => (1, "LSEARCH"),
            NativeNumericCommandPurpose::ListSortStride => (2, "LSORT"),
            _ => return None,
        };
        (integer < minimum).then(|| NativeNumericCommandError {
            message: format!("stride length must be at least {minimum}").into_bytes(),
            error_code: if self.version.is_none() {
                b"NONE".to_vec()
            } else {
                format!("TCL OPERATION {command} BADSTRIDE").into_bytes()
            },
        })
    }

    /// Actual encoder negative-line-length rejection after successful getter.
    #[must_use]
    pub fn negative_line_length_error(self) -> Option<NativeNumericCommandError> {
        matches!(
            self.purpose,
            NativeNumericCommandPurpose::Base64LineLength
                | NativeNumericCommandPurpose::UuencodeLineLength
        )
        .then(|| NativeNumericCommandError {
            message: b"line length out of range".to_vec(),
            error_code: b"TCL BINARY ENCODE LINE_LENGTH".to_vec(),
        })
    }
    /// Supported invalid-target backend rejection, using the actual addressed
    /// channel and Jim's retained opened path. Missing path remains unavailable.
    #[must_use]
    pub fn negative_seek_error(
        self,
        channel: &[u8],
        opened_path: Option<&[u8]>,
    ) -> Option<NativeNumericCommandError> {
        if !matches!(self.purpose, NativeNumericCommandPurpose::SeekOffset) {
            return None;
        }
        if self.version.is_none() {
            let mut message = tcl_core_types::c_string_extent(opened_path?).to_vec();
            message.extend_from_slice(b": Invalid argument");
            Some(NativeNumericCommandError {
                message,
                error_code: b"NONE".to_vec(),
            })
        } else {
            let mut message = b"error during seek on \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(channel));
            message.extend_from_slice(b"\": invalid argument");
            Some(NativeNumericCommandError {
                message,
                error_code: b"POSIX EINVAL {invalid argument}".to_vec(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationDialect;

    #[test]
    fn original_numeric_command_recipes_keep_actual_width_order_and_unavailable_workers() {
        // naming.numeric.io-debug-public-conversion-and-side-effect-order
        // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
        // Source/API recipe checks only; provider observations are separate.
        use NativeNumericCommandPurpose as Purpose;
        use NativeScalarGetterKind as Kind;
        for (version, read) in [
            (
                TclVersion::V8_4,
                NativeReadCountProtocol::LeadingDigitOrSuffix,
            ),
            (TclVersion::V8_5, NativeReadCountProtocol::IntOrSuffix),
            (
                TclVersion::V8_6,
                NativeReadCountProtocol::NonnegativeIntOrSuffix,
            ),
            (
                TclVersion::V9_0,
                NativeReadCountProtocol::QuietNonnegativeWide,
            ),
            (
                TclVersion::V9_1,
                NativeReadCountProtocol::QuietNonnegativeWide,
            ),
        ] {
            let actual = InvocationDialect::for_version(version);
            let selected = actual
                .native_numeric_command_protocol(Purpose::ReadCount)
                .unwrap();
            assert_eq!(selected.read_count(), Some(read));
            assert_eq!(selected.read_leading_options(), None);
            let seek = actual
                .native_numeric_command_protocol(Purpose::SeekOffset)
                .unwrap();
            assert_eq!(seek.getter_kind(), Kind::Wide);
            assert!(!seek.seek_origin_first());
            assert!(!seek.seek_origin_is_exact());
            assert_eq!(seek.read_count(), None);
            assert_eq!(selected.negative_seek_error(b"actual-channel", None), None);
            let binary = actual.native_numeric_command_protocol(Purpose::Base64LineLength);
            if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) {
                assert_eq!(binary, None);
            } else {
                assert_eq!(
                    binary.unwrap().getter_kind(),
                    if version == TclVersion::V8_6 {
                        Kind::Int
                    } else {
                        Kind::Wide
                    }
                );
                assert_eq!(
                    actual
                        .native_numeric_command_protocol(Purpose::UuencodeLineLength)
                        .unwrap()
                        .getter_kind(),
                    Kind::Int
                );
            }
            assert_eq!(
                actual
                    .native_numeric_command_protocol(Purpose::DebugFrame)
                    .is_some(),
                version != TclVersion::V8_4
            );
        }
        let jim =
            InvocationDialect::of_profile(crate::model::resolve_environment("jim").unit_profile());
        let read = jim
            .native_numeric_command_protocol(Purpose::ReadCount)
            .unwrap();
        assert_eq!(read.read_count(), Some(NativeReadCountProtocol::JimAio));
        assert_eq!(read.getter_kind(), Kind::Wide);
        assert_eq!(
            read.read_leading_options(),
            Some(["-pending", "-nonewline"].as_slice())
        );
        assert_eq!(read.read_option_suppresses_newline(0), Some(false));
        assert_eq!(read.read_option_suppresses_newline(1), Some(true));
        assert_eq!(read.read_option_suppresses_newline(2), None);
        let seek = jim
            .native_numeric_command_protocol(Purpose::SeekOffset)
            .unwrap();
        assert!(seek.seek_origin_first());
        assert!(seek.seek_origin_is_exact());
        assert_eq!(seek.negative_seek_error(b"actual-channel", None), None);
        assert_eq!(
            seek.negative_seek_error(b"actual-channel", Some(b"original-path"))
                .unwrap()
                .message,
            b"original-path: Invalid argument"
        );
        assert_eq!(
            jim.native_numeric_command_protocol(Purpose::Base64LineLength),
            None
        );
        assert_eq!(
            jim.native_numeric_command_protocol(Purpose::DebugFrame),
            None
        );
    }

    #[test]
    fn original_numeric_command_recipes_refuse_absent_and_foreign_scalar_axes() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use NativeNumericCommandPurpose as Purpose;
        let original = InvocationDialect::for_version(TclVersion::V8_6);
        assert!(
            original
                .native_numeric_command_protocol(Purpose::ReadCount)
                .is_some()
        );
        let mut absent = original;
        absent.core_point = None;
        absent.native_family = None;
        assert_eq!(
            absent.native_numeric_command_protocol(Purpose::ReadCount),
            None
        );
        let mut foreign = original;
        foreign.native_family = Some(tcl_dialect::model::Family::Jim);
        assert_eq!(
            foreign.native_numeric_command_protocol(Purpose::ReadCount),
            None
        );
        let mut unknown = original;
        unknown.core_point = None;
        unknown.tcl_version = None;
        assert_eq!(
            unknown.native_numeric_command_protocol(Purpose::DebugFrame),
            None
        );
        let selected = original
            .native_numeric_command_protocol(Purpose::ReadCount)
            .unwrap();
        assert_eq!(selected.negative_line_length_error(), None);
        assert_eq!(selected.read_option_suppresses_newline(0), None);
    }
    #[test]
    fn original_stride_recipes_keep_actual_command_tables_primitive_widths_and_minima() {
        // naming.list.original-stride-primitive-and-results
        // docs/design/analysis/name-resolution-proofs/list-original-stride-primitive-and-results.md
        // Source/API purpose controls; actual108 command outcomes are independent.
        use NativeNumericCommandPurpose as Purpose;
        use NativeScalarGetterKind as Kind;
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            let selected = InvocationDialect::for_version(version);
            assert_eq!(
                selected.native_numeric_command_protocol(Purpose::ListSearchStride),
                None
            );
            assert_eq!(
                selected.native_numeric_command_protocol(Purpose::ListSortStride),
                None
            );
        }
        let c86 = InvocationDialect::for_version(TclVersion::V8_6);
        assert_eq!(
            c86.native_numeric_command_protocol(Purpose::ListSearchStride),
            None
        );
        assert_eq!(
            c86.native_numeric_command_protocol(Purpose::ListSortStride)
                .unwrap()
                .getter_kind(),
            Kind::Int
        );
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let selected = InvocationDialect::for_version(version);
            for (purpose, minimum, code) in [
                (
                    Purpose::ListSearchStride,
                    1,
                    b"TCL OPERATION LSEARCH BADSTRIDE".as_slice(),
                ),
                (
                    Purpose::ListSortStride,
                    2,
                    b"TCL OPERATION LSORT BADSTRIDE".as_slice(),
                ),
            ] {
                let recipe = selected.native_numeric_command_protocol(purpose).unwrap();
                assert_eq!(recipe.getter_kind(), Kind::Wide);
                assert_eq!(recipe.invalid_list_stride_error(minimum), None);
                assert_eq!(
                    recipe
                        .invalid_list_stride_error(minimum - 1)
                        .unwrap()
                        .error_code,
                    code
                );
            }
        }
        let jim =
            InvocationDialect::of_profile(crate::model::resolve_environment("jim").unit_profile());
        for (purpose, minimum) in [(Purpose::ListSearchStride, 1), (Purpose::ListSortStride, 2)] {
            let recipe = jim.native_numeric_command_protocol(purpose).unwrap();
            assert_eq!(recipe.getter_kind(), Kind::Long);
            assert_eq!(recipe.invalid_list_stride_error(minimum), None);
            assert_eq!(
                recipe
                    .invalid_list_stride_error(minimum - 1)
                    .unwrap()
                    .error_code,
                b"NONE"
            );
        }
        let mut absent = c86;
        absent.core_point = None;
        absent.native_family = None;
        assert_eq!(
            absent.native_numeric_command_protocol(Purpose::ListSortStride),
            None
        );
        assert_eq!(
            c86.native_numeric_command_protocol(Purpose::ReadCount)
                .unwrap()
                .invalid_list_stride_error(0),
            None
        );
    }
}
