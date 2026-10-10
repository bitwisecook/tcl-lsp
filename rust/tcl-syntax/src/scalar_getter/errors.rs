// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primitive getter diagnostics, separately from interpreter propagation.

use super::{
    Engine, NativeScalarGetterFailure as Failure, NativeScalarGetterKind as Kind,
    NativeScalarGetterProtocol, convert, invalid, nul_prefix,
};
use crate::{list, native_tcl_utf::NativeTclUtf};
use std::borrow::Cow;
use tcl_dialect::{EscapeSyntax, ListParse, TclVersion};

/// Primitive update to the existing interpreter error-code state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScalarGetterErrorCode {
    /// Keep the actual previous state; this is not an instruction to set NONE.
    Unchanged,
    /// Replace the state with these exact native list bytes.
    Set(Vec<u8>),
}

/// Result object producer of a reached primitive error, independently of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarGetterResultProducer {
    /// A fresh string-byte object without the native String primary.
    FreshString,
    /// Appending or formatting establishes the native String primary.
    AppendString,
}

/// Exact primitive failure and its independently measured propagation stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScalarGetterError {
    protocol: NativeScalarGetterProtocol,
    kind: Kind,
    failure: Failure,
    message: Vec<u8>,
    error_code: NativeScalarGetterErrorCode,
    eval_nul_terminated: bool,
}

impl NativeScalarGetterError {
    /// Actual primitive engine protocol retained by the reached failure.
    #[must_use]
    pub const fn protocol(&self) -> NativeScalarGetterProtocol {
        self.protocol
    }

    /// Primitive getter stage; this does not identify expression propagation.
    #[must_use]
    pub const fn getter_kind(&self) -> Kind {
        self.kind
    }

    /// Original failure origin, including cached versus fresh parsing branches.
    #[must_use]
    pub const fn failure_origin(&self) -> Failure {
        self.failure
    }

    /// Original C integer getter result producer. Unsupported getter/failure
    /// combinations and Jim have no C producer receipt. This pure recipe grants
    /// neither an original result header nor interpreter execution authority.
    #[must_use]
    pub const fn integer_result_producer(&self) -> Option<NativeScalarGetterResultProducer> {
        use NativeScalarGetterResultProducer::{AppendString, FreshString};
        let Some(version) = self.protocol.tcl_version() else {
            return None;
        };
        if !matches!(self.kind, Kind::Int | Kind::Long | Kind::Wide) {
            return None;
        }
        match self.failure {
            Failure::IntWidthOverflow => Some(FreshString),
            Failure::IntegerOverflow if matches!(version, TclVersion::V8_4) => Some(AppendString),
            Failure::IntegerOverflow => Some(FreshString),
            Failure::Invalid | Failure::InvalidOctal | Failure::CachedNonInteger => {
                Some(AppendString)
            }
            _ => None,
        }
    }

    /// Primitive result bytes before an interpreter dispatch propagates them.
    #[must_use]
    pub fn message_bytes(&self) -> &[u8] {
        &self.message
    }

    /// Apply this update to the actual existing interpreter state.
    #[must_use]
    pub const fn error_code_update(&self) -> &NativeScalarGetterErrorCode {
        &self.error_code
    }

    /// Bytes exposed by the measured `Tcl_Eval` propagation stage. This is a
    /// separate projection: a primitive C8.5 cached failure can retain NUL and
    /// its suffix while legacy interpreter propagation truncates that result.
    #[must_use]
    pub fn eval_message_bytes(&self) -> &[u8] {
        self.eval_result_bytes(&self.message)
    }

    /// Apply only the retained Eval byte-extent rule to the actual current
    /// result, preserving any command-owned contextual prefix. The caller must
    /// retire this receipt on an unrelated result replacement.
    #[must_use]
    pub fn eval_result_bytes<'a>(&self, current_result: &'a [u8]) -> &'a [u8] {
        if self.eval_nul_terminated {
            nul_prefix(current_result)
        } else {
            current_result
        }
    }
}

impl NativeScalarGetterProtocol {
    /// Render a reached primitive failure with its original materialized
    /// string and retained failure origin. `None` means that this combination
    /// is not an authored native failure; it must not become a guest error.
    #[must_use]
    pub fn failure_presentation(
        self,
        kind: Kind,
        failure: Failure,
        original: &[u8],
    ) -> Option<NativeScalarGetterError> {
        let presentation = self.failure_message(kind, failure, original)?;
        Some(NativeScalarGetterError {
            protocol: self,
            kind,
            failure,
            message: presentation.message,
            error_code: presentation.error_code,
            eval_nul_terminated: presentation.eval_nul_terminated,
        })
    }

    /// Whether this selected primitive failure requires original string access.
    /// `None` refuses an unmodeled kind/origin; false describes a constant
    /// diagnostic and supplies no object getter or expression authority.
    #[must_use]
    pub fn failure_requires_original_string(self, kind: Kind, failure: Failure) -> Option<bool> {
        // Selection shares the existing presentation owner. No bytes from an
        // object are claimed by this availability-only query.
        self.failure_message(kind, failure, &[])?;
        Some(matches!(
            failure,
            Failure::Invalid | Failure::InvalidOctal | Failure::CachedNonInteger
        ))
    }

    /// Render a modeled constant diagnostic without requesting original bytes.
    /// An operand-dependent failure refuses this door rather than substituting
    /// an invented empty original string.
    #[must_use]
    pub fn failure_presentation_without_original_string(
        self,
        kind: Kind,
        failure: Failure,
    ) -> Option<NativeScalarGetterError> {
        if self.failure_requires_original_string(kind, failure)? {
            return None;
        }
        self.failure_presentation(kind, failure, &[])
    }

    fn failure_message(
        self,
        kind: Kind,
        failure: Failure,
        original: &[u8],
    ) -> Option<PrimitiveFailurePresentation> {
        if kind == Kind::Int {
            return self.int_failure_presentation(failure, original);
        }
        match failure {
            Failure::Invalid | Failure::InvalidOctal => {
                self.invalid_presentation(kind, failure, original)
            }
            Failure::CachedNonInteger if matches!(kind, Kind::Wide | Kind::Long) => {
                let version = self.tcl_version()?;
                if version == TclVersion::V8_4 {
                    return None;
                }
                let bytes = if version == TclVersion::V8_5 {
                    original
                } else {
                    nul_prefix(original)
                };
                Some(record(
                    quoted(b"integer", bytes),
                    if version == TclVersion::V8_5 {
                        None
                    } else {
                        Some(b"TCL VALUE INTEGER")
                    },
                    version == TclVersion::V8_5,
                ))
            }
            Failure::IntegerOverflow if matches!(kind, Kind::Wide | Kind::Long) => {
                if self.is_jim084() {
                    Some(record(
                        b"Integer value too big to be represented".to_vec(),
                        None,
                        false,
                    ))
                } else {
                    Some(record(
                        b"integer value too large to represent".to_vec(),
                        Some(b"ARITH IOVERFLOW {integer value too large to represent}"),
                        false,
                    ))
                }
            }
            Failure::FloatingPointNaN if kind != Kind::Wide => {
                let version = self.tcl_version()?;
                if version == TclVersion::V8_4 {
                    return None;
                }
                Some(record(
                    b"floating point value is Not a Number".to_vec(),
                    (version >= TclVersion::V8_6).then_some(b"TCL VALUE DOUBLE NAN".as_slice()),
                    false,
                ))
            }
            Failure::FloatingPointDomain | Failure::FloatingPointUnknown(_)
                if kind == Kind::Double && self.tcl_version() == Some(TclVersion::V8_4) =>
            {
                let diagnostic = match failure {
                    Failure::FloatingPointDomain => crate::expr::errors::NativeFloatError::Domain,
                    Failure::FloatingPointUnknown(errno) => {
                        crate::expr::errors::NativeFloatError::Unknown(errno)
                    }
                    _ => unreachable!(),
                };
                let (message, code) = diagnostic.diagnostic();
                Some(record(message.into_bytes(), Some(code.as_bytes()), false))
            }
            Failure::FloatingPointRange { result_is_zero }
                if kind == Kind::Double && self.tcl_version() == Some(TclVersion::V8_4) =>
            {
                let (message, code) = if result_is_zero {
                    (
                        b"floating-point value too small to represent".as_slice(),
                        b"ARITH UNDERFLOW {floating-point value too small to represent}".as_slice(),
                    )
                } else {
                    (
                        b"floating-point value too large to represent".as_slice(),
                        b"ARITH OVERFLOW {floating-point value too large to represent}".as_slice(),
                    )
                };
                Some(record(message.to_vec(), Some(code), false))
            }
            _ => None,
        }
    }

    fn int_failure_presentation(
        self,
        failure: Failure,
        original: &[u8],
    ) -> Option<PrimitiveFailurePresentation> {
        let version = self.tcl_version()?;
        if failure == Failure::IntWidthOverflow {
            let message = if version <= TclVersion::V8_5 {
                b"integer value too large to represent as non-long integer".as_slice()
            } else {
                b"integer value too large to represent".as_slice()
            };
            let code = if version == TclVersion::V8_4 {
                None
            } else if version == TclVersion::V8_5 {
                Some(
                    b"ARITH IOVERFLOW {integer value too large to represent as non-long integer}"
                        .as_slice(),
                )
            } else {
                Some(b"ARITH IOVERFLOW {integer value too large to represent}".as_slice())
            };
            return Some(record(message.to_vec(), code, false));
        }
        if version == TclVersion::V8_6
            && matches!(
                failure,
                Failure::Invalid | Failure::InvalidOctal | Failure::CachedNonInteger
            )
        {
            return Some(record(
                quoted(b"integer", nul_prefix(original)),
                Some(b"TCL VALUE INTEGER"),
                false,
            ));
        }
        let mut presentation = self.failure_message(Kind::Wide, failure, original)?;
        if version == TclVersion::V8_4
            && matches!(failure, Failure::Invalid | Failure::InvalidOctal)
        {
            presentation.error_code = NativeScalarGetterErrorCode::Unchanged;
        }
        Some(presentation)
    }

    fn invalid_presentation(
        self,
        kind: Kind,
        failure: Failure,
        original: &[u8],
    ) -> Option<PrimitiveFailurePresentation> {
        if failure == Failure::InvalidOctal
            && !matches!(
                (self.engine, kind),
                (Engine::Tcl(TclVersion::V8_4), Kind::Wide | Kind::Long)
                    | (
                        Engine::Tcl(TclVersion::V8_5 | TclVersion::V8_6),
                        Kind::Double | Kind::Boolean
                    )
            )
        {
            return None;
        }
        let expected = match kind {
            Kind::Int | Kind::Long | Kind::Wide => b"integer".as_slice(),
            Kind::Double => b"floating-point number".as_slice(),
            Kind::Boolean if self.is_jim084() => b"boolean".as_slice(),
            Kind::Boolean => b"boolean value".as_slice(),
        };
        let input = nul_prefix(original);
        let mut message = if self
            .tcl_version()
            .is_some_and(|version| version >= TclVersion::V9_0)
            && list::max_list_length_bytes(input) > 1
            && list::split_list_bytes_in(input, ListParse::Strict, EscapeSyntax::Tcl90).is_ok()
        {
            [b"expected ".as_slice(), expected, b" but got a list"].concat()
        } else {
            quoted(expected, &self.invalid_message_operand(input)?)
        };
        if failure == Failure::InvalidOctal {
            message.extend_from_slice(b" (looks like invalid octal number)");
        }
        let code = match self.engine {
            Engine::Jim084 => None,
            // Tcl_GetLongFromObj's badInteger branch resets the result without
            // setting errorCode. The later Tcl_Eval callback projection of
            // other selected getters remains a separate measured boundary.
            Engine::Tcl(TclVersion::V8_4) if kind == Kind::Long => None,
            Engine::Tcl(TclVersion::V8_4) => Some(b"NONE".as_slice()),
            Engine::Tcl(_) => Some(b"TCL VALUE NUMBER".as_slice()),
        };
        Some(record(message, code, false))
    }

    fn invalid_message_operand(self, input: &[u8]) -> Option<Cow<'_, [u8]>> {
        let Some(version) = self.tcl_version() else {
            return Some(Cow::Borrowed(input));
        };
        let policy = NativeTclUtf::for_version(version);
        let end = if input.len() <= 50 {
            input.len()
        } else if version >= TclVersion::V8_5 {
            policy.previous_character_boundary(input, 51)?
        } else {
            50
        };
        let clipped = &input[..end];
        if version < TclVersion::V9_0 || input.first().is_none_or(|byte| byte & 0xc0 != 0x80) {
            return Some(Cow::Borrowed(clipped));
        }
        // AppendLimited forces the result's Unicode representation when the
        // appended value starts with a continuation byte. Use native units.
        let mut output = Vec::new();
        let mut at = 0;
        let mut previous = None;
        while at < clipped.len() {
            let unit = policy.decode_unit(&clipped[at..], previous)?;
            policy.encode_unit(unit.value, &mut output)?;
            at += unit.width;
            previous = Some(unit.value);
        }
        Some(Cow::Owned(output))
    }

    pub(super) fn invalid_conversion(
        self,
        kind: Kind,
        input: &[u8],
    ) -> super::NativeScalarGetterConversion {
        if invalid_octal(self, kind, nul_prefix(input)) {
            convert(None, Err(Failure::InvalidOctal))
        } else {
            invalid()
        }
    }
}

fn quoted(expected: &[u8], value: &[u8]) -> Vec<u8> {
    [
        b"expected ".as_slice(),
        expected,
        b" but got \"",
        value,
        b"\"",
    ]
    .concat()
}
struct PrimitiveFailurePresentation {
    message: Vec<u8>,
    error_code: NativeScalarGetterErrorCode,
    eval_nul_terminated: bool,
}

fn record(
    message: Vec<u8>,
    code: Option<&[u8]>,
    eval_nul_terminated: bool,
) -> PrimitiveFailurePresentation {
    PrimitiveFailurePresentation {
        message,
        error_code: code.map_or(NativeScalarGetterErrorCode::Unchanged, |bytes| {
            NativeScalarGetterErrorCode::Set(bytes.to_vec())
        }),
        eval_nul_terminated,
    }
}
fn invalid_octal(protocol: NativeScalarGetterProtocol, kind: Kind, input: &[u8]) -> bool {
    let Some(version) = protocol.tcl_version() else {
        return false;
    };
    if version >= TclVersion::V9_0 {
        return false;
    }
    let input = input.trim_ascii_start();
    let input = input
        .strip_prefix(b"+")
        .or_else(|| input.strip_prefix(b"-"))
        .unwrap_or(input);
    if input.first() != Some(&b'0') {
        return false;
    }
    let end = input
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(input.len());
    if version == TclVersion::V8_4 {
        return matches!(kind, Kind::Wide | Kind::Long) && input[end..].trim_ascii().is_empty();
    }
    kind != Kind::Wide
        && input[..end]
            .iter()
            .any(|byte| *byte == b'8' || *byte == b'9')
        && !matches!(input.get(end), Some(b'.' | b'e' | b'E'))
}
