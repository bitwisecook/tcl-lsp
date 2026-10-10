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

//! Actual native command-source error logging recipes.

use crate::InvocationDialect;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::native_tcl_utf::NativeTclUtf;

/// Native error-info formatter selected independently of authored grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeErrorLogProtocol {
    /// Canonical C Tcl byte formatter for the actual release.
    C(tcl_dialect::TclVersion),
}

impl NativeErrorLogProtocol {
    /// `Tcl_ResetResult` clears the active global error episode in C8.4.
    /// The global variable headers remain owned by their existing cells.
    #[must_use]
    pub const fn resets_global_error_episode(self) -> bool {
        matches!(self, Self::C(tcl_dialect::TclVersion::V8_4))
    }

    /// C8.4's direct Tcl_LogCommandInfo clears ERR_ALREADY_LOGGED after
    /// the global setters; the evaluating bytecode command sets it separately.
    #[must_use]
    pub const fn evaluation_owns_logged_flag(self) -> bool {
        matches!(self, Self::C(tcl_dialect::TclVersion::V8_4))
    }

    /// Original word updater selected by the actual logging engine.
    #[must_use]
    pub const fn string_protocol(self) -> NativeStringProtocol {
        match self {
            Self::C(version) => NativeStringProtocol::C(version),
        }
    }

    /// Whether direct object-vector evaluation reaches the original words'
    /// string updaters after an error. Tcl 8.4 and 8.5 construct the command
    /// before checking the already-logged flag; Tcl 8.6 and 9 check first.
    #[must_use]
    pub const fn observes_error_words(self, already_logged: bool) -> bool {
        match self {
            Self::C(tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5) => true,
            Self::C(_) => !already_logged,
        }
    }

    /// Whether the error logger owns a real temporary List of original words.
    #[must_use]
    pub const fn retains_list_words(self) -> bool {
        !matches!(self, Self::C(tcl_dialect::TclVersion::V8_4))
    }

    /// Direct object-vector error command from already-materialized original
    /// word bytes. The 8.4 `DString` path consumes NUL-terminated elements;
    /// later releases serialize a counted List object.
    #[must_use]
    pub fn object_vector_command(self, words: &[impl AsRef<[u8]>]) -> Vec<u8> {
        use tcl_syntax::list_result::NativeListResultSerialization;
        match self {
            Self::C(tcl_dialect::TclVersion::V8_4) => {
                let words = words
                    .iter()
                    .map(|word| {
                        let bytes = word.as_ref();
                        &bytes[..bytes
                            .iter()
                            .position(|byte| *byte == 0)
                            .unwrap_or(bytes.len())]
                    })
                    .collect::<Vec<_>>();
                NativeListResultSerialization::Tcl84.render(&words)
            }
            Self::C(_) => NativeListResultSerialization::Tcl85Plus.render(words),
        }
    }

    /// Original excerpt and native ellipsis decision.
    #[must_use]
    pub fn excerpt(self, source: &[u8], length: usize) -> Option<(&[u8], bool)> {
        match self {
            Self::C(version) => {
                NativeTclUtf::for_version(version).command_log_excerpt(source, length)
            }
        }
    }
}

impl InvocationDialect {
    /// Native command-source logging has no Jim or authored-grammar fallback.
    #[must_use]
    pub fn native_error_log_protocol(self) -> Option<NativeErrorLogProtocol> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(version) => Some(NativeErrorLogProtocol::C(version)),
            NativeStringProtocol::Jim084 => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_error_episode_reset_requires_the_actual_c84_protocol() {
        for name in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let profile = crate::model::resolve_environment(name).unit_profile();
            let dialect = InvocationDialect::of_profile(profile);
            assert_eq!(
                dialect
                    .native_error_log_protocol()
                    .is_some_and(NativeErrorLogProtocol::resets_global_error_episode),
                name == "tcl8.4",
                "{name}",
            );
        }
    }

    #[test]
    fn command_excerpts_match_actual_c_api_byte_formatters() {
        // Tcl_LogCommandInfo on canonical C 8.4.20 through 9.1.0; the
        // original result is captured before any potentially mutating observer.
        let cases = [
            (
                tcl_dialect::TclVersion::V8_4,
                [149, 148, 149, 148, 10, 148, 150, 150, 149, 148, 10, 150],
            ),
            (
                tcl_dialect::TclVersion::V8_5,
                [149, 148, 149, 149, 10, 148, 150, 150, 149, 150, 10, 150],
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                [149, 148, 149, 149, 10, 148, 150, 150, 149, 149, 10, 150],
            ),
            (
                tcl_dialect::TclVersion::V9_0,
                [149, 148, 149, 149, 10, 148, 150, 150, 149, 149, 10, 150],
            ),
            (
                tcl_dialect::TclVersion::V9_1,
                [149, 148, 149, 149, 10, 148, 150, 150, 149, 149, 10, 150],
            ),
        ];
        for (version, ends) in cases {
            for (case, end) in ends.into_iter().enumerate() {
                let mut source = vec![b'x'; 169];
                match case % 6 {
                    1 => source[148..150].copy_from_slice(&[0xc3, 0xa9]),
                    2 => source[149..152].copy_from_slice(&[0xe2, 0x82, 0xac]),
                    3 => source[148..151].copy_from_slice(&[0xff, 0x80, 0x80]),
                    4 => source[10] = 0,
                    5 => source[148..150].copy_from_slice(&[0xc0, 0x80]),
                    _ => {}
                }
                let ellipsis = case >= 6
                    || version == tcl_dialect::TclVersion::V8_4 && matches!(case, 1 | 3 | 5);
                assert_eq!(
                    NativeErrorLogProtocol::C(version)
                        .excerpt(&source, if case < 6 { 149 } else { 155 }),
                    Some((&source[..end], ellipsis)),
                    "{version:?} case {case}"
                );
            }
        }
    }
}
