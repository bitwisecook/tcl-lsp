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

//! Release-selected options for an original hidden-command invocation.

use super::{NameProjectionUnavailable, NativeNameProtocol};
use tcl_dialect::TclVersion;

/// Selected option action, independent of any interpreter or command target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHiddenInvocationOption {
    /// Execute from the actual global namespace.
    Global,
    /// Resolve and create the supplied namespace from the actual global token.
    Namespace,
    /// Stop option processing before the next original word.
    End,
}

/// Exact original static-option table and its selected ordinal meanings.
#[derive(Clone, Copy, Debug)]
pub struct NativeHiddenInvocationGrammar {
    names: &'static [&'static str],
    byte_names: &'static [&'static [u8]],
    options: &'static [NativeHiddenInvocationOption],
}

impl NativeHiddenInvocationGrammar {
    /// Original textual labels for a native static-index table.
    #[must_use]
    pub const fn names(self) -> &'static [&'static str] {
        self.names
    }

    /// The same original labels for byte-oriented native table adapters.
    #[must_use]
    pub const fn byte_names(self) -> &'static [&'static [u8]] {
        self.byte_names
    }

    /// Meaning of an ordinal returned by this exact selected table.
    #[must_use]
    pub fn option(self, index: usize) -> Option<NativeHiddenInvocationOption> {
        self.options.get(index).copied()
    }
}

impl NativeNameProtocol {
    /// Original C hidden-invocation option layout. No entry or frame is issued.
    ///
    /// # Errors
    /// Jim has no corresponding original hidden-command operation.
    pub fn hidden_invocation_grammar(
        self,
    ) -> Result<NativeHiddenInvocationGrammar, NameProjectionUnavailable> {
        use NativeHiddenInvocationOption::{End, Global, Namespace};
        match self {
            Self::C(TclVersion::V8_4) => Ok(NativeHiddenInvocationGrammar {
                names: &["-global", "--"],
                byte_names: &[b"-global", b"--"],
                options: &[Global, End],
            }),
            Self::C(_) => Ok(NativeHiddenInvocationGrammar {
                names: &["-global", "-namespace", "--"],
                byte_names: &[b"-global", b"-namespace", b"--"],
                options: &[Global, Namespace, End],
            }),
            Self::Jim084 => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_hidden_options_keep_release_and_ordinal_meanings() {
        use NativeHiddenInvocationOption::{End, Global, Namespace};
        for version in TclVersion::ALL {
            let grammar = NativeNameProtocol::C(version)
                .hidden_invocation_grammar()
                .unwrap();
            let expected: &[(&str, NativeHiddenInvocationOption)] = if version == TclVersion::V8_4 {
                &[("-global", Global), ("--", End)]
            } else {
                &[("-global", Global), ("-namespace", Namespace), ("--", End)]
            };
            assert_eq!(grammar.names().len(), expected.len());
            assert_eq!(grammar.byte_names().len(), expected.len());
            for (index, (name, option)) in expected.iter().enumerate() {
                assert_eq!(grammar.names()[index], *name);
                assert_eq!(grammar.byte_names()[index], name.as_bytes());
                assert_eq!(grammar.option(index), Some(*option));
            }
            assert_eq!(grammar.option(expected.len()), None);
        }
        assert!(
            NativeNameProtocol::Jim084
                .hidden_invocation_grammar()
                .is_err()
        );
    }
}
