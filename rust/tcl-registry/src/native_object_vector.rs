// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Public original object-vector evaluation, independent of source-command logging.

use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Argument projection performed after a public object-vector evaluation errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeObjectVectorErrorProjection {
    /// C84 constructs its command with `Tcl_DStringAppendElement` and `CString` words.
    CStringWords,
    /// C85+ constructs a transient List containing the same original argv children.
    OriginalList,
    /// The boundary uses no argument-string projection.
    None,
}

/// Authentic engine recipe for an already evaluated, original object vector.
/// This packet supplies no command resolution, compilation or source-text authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeObjectVectorProtocol {
    strings: NativeStringProtocol,
}

impl NativeObjectVectorProtocol {
    /// Actual object-string recipe, retained independently of lexical grammar.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }

    /// Jim's public vector entry pins each original child through invocation.
    #[must_use]
    pub const fn pins_arguments(self) -> bool {
        matches!(self.strings, NativeStringProtocol::Jim084)
    }

    /// Select the public error-log projection from the actual completion's logged flag.
    #[must_use]
    pub const fn error_projection(self, already_logged: bool) -> NativeObjectVectorErrorProjection {
        use NativeObjectVectorErrorProjection as Projection;
        match self.strings {
            NativeStringProtocol::Jim084 => Projection::None,
            NativeStringProtocol::C(TclVersion::V8_4) => Projection::CStringWords,
            NativeStringProtocol::C(TclVersion::V8_5) => Projection::OriginalList,
            NativeStringProtocol::C(_) if already_logged => Projection::None,
            NativeStringProtocol::C(_) => Projection::OriginalList,
        }
    }

    /// C86+ `TEOV_Error` clears the logged flag even when it skips projection.
    #[must_use]
    pub const fn clears_logged_on_exit(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        )
    }

    /// Public C completion normalization after `TclUpdateReturnInfo`.
    /// C85 rejects a non-OK original completion even when settling Return
    /// produced OK; other C versions accept that settled OK. Jim preserves
    /// its vector completion without the C outermost normalization.
    #[must_use]
    pub const fn unexpected_completion(self, original: i64, settled: i64) -> bool {
        match self.strings {
            NativeStringProtocol::Jim084 => false,
            NativeStringProtocol::C(TclVersion::V8_5) => original != 0 && settled != 1,
            NativeStringProtocol::C(_) => settled != 0 && settled != 1,
        }
    }

    /// Structured `ProcessUnexpectedResult` error code, absent in C84/85 and Jim.
    #[must_use]
    pub fn unexpected_completion_error_code(self, code: i64) -> Option<Vec<u8>> {
        if !matches!(
            self.strings,
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        ) {
            return None;
        }
        Some(
            tcl_syntax::list_result::NativeListResultSerialization::Tcl85Plus.render(&[
                b"TCL".as_slice(),
                b"UNEXPECTED_RESULT_CODE",
                code.to_string().as_bytes(),
            ]),
        )
    }

    /// Native C append/format producers retain an unknown-count String primary.
    #[must_use]
    pub const fn missing_command_has_string_primary(self) -> bool {
        matches!(self.strings, NativeStringProtocol::C(_))
    }

    /// C86+ publishes the structured lookup error code on an absent unknown handler.
    #[must_use]
    pub const fn missing_command_has_lookup_code(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        )
    }
}

impl crate::InvocationDialect {
    /// Issue the public vector recipe only from an actual supported engine point.
    #[must_use]
    pub fn native_object_vector_protocol(self) -> Option<NativeObjectVectorProtocol> {
        Some(NativeObjectVectorProtocol {
            strings: self.native_string_protocol()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_vector_projection_keeps_cstring_list_and_jim_boundaries_distinct() {
        use NativeObjectVectorErrorProjection as Projection;
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = crate::InvocationDialect::for_version(version)
                .native_object_vector_protocol()
                .unwrap();
            assert!(!recipe.pins_arguments());
            assert!(recipe.missing_command_has_string_primary());
            assert_eq!(
                recipe.missing_command_has_lookup_code(),
                version >= TclVersion::V8_6
            );
            assert_eq!(
                recipe.unexpected_completion(2, 0),
                version == TclVersion::V8_5
            );
            assert!(!recipe.unexpected_completion(0, 0));
            assert!(!recipe.unexpected_completion(2, 1));
            assert!(recipe.unexpected_completion(3, 3));
            assert_eq!(
                recipe.error_projection(false),
                if version == TclVersion::V8_4 {
                    Projection::CStringWords
                } else {
                    Projection::OriginalList
                }
            );
            assert_eq!(
                recipe.error_projection(true),
                if version >= TclVersion::V8_6 {
                    Projection::None
                } else {
                    recipe.error_projection(false)
                }
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        )
        .native_object_vector_protocol()
        .unwrap();
        assert!(jim.pins_arguments());
        assert_eq!(jim.error_projection(false), Projection::None);
        assert!(!jim.missing_command_has_string_primary());
        assert!(!jim.unexpected_completion(2, 2));
        let mut unknown = crate::InvocationDialect::for_version(TclVersion::V9_0);
        unknown.core_point = None;
        unknown.native_family = None;
        assert_eq!(unknown.native_object_vector_protocol(), None);
    }
}
