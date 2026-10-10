// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected `TclOO` introspection argv layouts, independent of target lookup.
//!
//! These pure recipes create no object, method, variable, Index cache or
//! registration receipt. Original getters remain the runtime adapter's job.

mod methods;
pub use methods::{
    METHOD_INFO_OPTIONS, NativeTclooMethodInfoProtocol, NativeTclooMethodInfoSelection,
};

use crate::InvocationDialect;
use crate::commands::tcl::InfoOoEnsembleKind;
use tcl_cmd_core::error::CmdError;
use tcl_dialect::{TclVersion, model::Family};
use tcl_syntax::native_string::NativeStringProtocol;

/// C8.6/C9 declared-variable introspection before target lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooVariableInfoProtocol {
    version: TclVersion,
}

impl NativeTclooVariableInfoProtocol {
    /// Select an independently identified C release that provides `TclOO`.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        let version = dialect.tcl_version?;
        (dialect.family() == Some(Family::Tcl) && version >= TclVersion::V8_6)
            .then_some(Self { version })
    }

    /// Validate post-member argc before reading the optional flag or target.
    #[must_use]
    pub const fn accepts_count(self, count: usize) -> bool {
        count == 1 || (count == 2 && matches!(self.version, TclVersion::V9_0 | TclVersion::V9_1))
    }

    /// The native wrong-arity suffix for the selected ensemble member.
    #[must_use]
    pub const fn usage(self, kind: InfoOoEnsembleKind) -> &'static str {
        match (kind, self.version) {
            (InfoOoEnsembleKind::Object, TclVersion::V9_0 | TclVersion::V9_1) => {
                "info object variables objName ?-private?"
            }
            (InfoOoEnsembleKind::Class, TclVersion::V9_0 | TclVersion::V9_1) => {
                "info class variables className ?-private?"
            }
            (InfoOoEnsembleKind::Object, _) => "info object variables objName",
            (InfoOoEnsembleKind::Class, _) => "info class variables className",
        }
    }

    /// C9 uses exact strcmp, independently of any Index abbreviation/cache.
    /// The original object is already read; only its `CString` view is compared
    /// and used by the native printf diagnostic. Call after `accepts_count`.
    ///
    /// # Errors
    /// The native OO `BAD_ARG` diagnostic for any other option.
    pub fn private_option(self, original: &[u8]) -> Result<bool, CmdError> {
        if self.version < TclVersion::V9_0 {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "TclOO private variable option",
            )
            .into());
        }
        let option = tcl_core_types::c_string_extent(original);
        if option == b"-private" {
            return Ok(true);
        }
        let mut message = b"option \"".to_vec();
        message.extend_from_slice(option);
        message.extend_from_slice(b"\" is not exactly \"-private\"");
        Err(
            CmdError::with_error_code_bytes(message, b"TCL OO BAD_ARG".to_vec())
                .with_native_string_result(NativeStringProtocol::C(self.version)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_info_flags_keep_exact_cstring_and_release_arity_separate() {
        // Source proof: naming.tcloo.variable-info-option-order
        // docs/design/analysis/name-resolution-proofs/variable-info-option-order.md
        for version in TclVersion::ALL {
            let selected =
                NativeTclooVariableInfoProtocol::select(InvocationDialect::for_version(version));
            assert_eq!(selected.is_some(), version >= TclVersion::V8_6);
            let Some(selected) = selected else {
                continue;
            };
            assert!(selected.accepts_count(1));
            assert_eq!(selected.accepts_count(2), version >= TclVersion::V9_0);
            assert!(!selected.accepts_count(0));
            assert!(!selected.accepts_count(3));
            if version >= TclVersion::V9_0 {
                assert!(selected.private_option(b"-private\0\xff").unwrap());
                for option in [b"-priv".as_slice(), b"-private\xc0\x80", b"-private\xff"] {
                    assert!(selected.private_option(option).is_err());
                }
                let error = selected.private_option(b"-bad\xff\0TAIL").unwrap_err();
                assert_eq!(
                    error.message_bytes(),
                    b"option \"-bad\xff\" is not exactly \"-private\""
                );
            }
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(NativeTclooVariableInfoProtocol::select(jim).is_none());
    }
}
