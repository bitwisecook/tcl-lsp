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

//! Argument grammar of the original namespace-variable declaration worker.

use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Release-selected operand arity and native diagnostic for `variable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeVariableDeclarationProtocol {
    /// C8.4 and C8.5 require at least one name.
    Tcl85AndEarlier,
    /// C8.6 and later accept an empty declaration.
    Tcl86Plus,
    /// Jim accepts one name and an optional value.
    Jim084,
}

impl NativeVariableDeclarationProtocol {
    /// Validate the operand count after the command word.
    ///
    /// # Errors
    /// Returns the original worker's argument-usage suffix.
    pub const fn arguments(self, operand_count: usize) -> Result<(), &'static [u8]> {
        match self {
            Self::Tcl86Plus => Ok(()),
            Self::Tcl85AndEarlier if operand_count != 0 => Ok(()),
            Self::Jim084 if operand_count == 1 || operand_count == 2 => Ok(()),
            Self::Tcl85AndEarlier => Err(b"?name value...? name ?value?"),
            Self::Jim084 => Err(b"name ?value?"),
        }
    }
}

impl crate::InvocationDialect {
    /// Select only the authenticated worker grammar, without declaring a cell.
    #[must_use]
    pub fn native_variable_declaration_protocol(self) -> Option<NativeVariableDeclarationProtocol> {
        Some(match self.native_string_protocol()? {
            NativeStringProtocol::C(TclVersion::V8_4 | TclVersion::V8_5) => {
                NativeVariableDeclarationProtocol::Tcl85AndEarlier
            }
            NativeStringProtocol::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => {
                NativeVariableDeclarationProtocol::Tcl86Plus
            }
            NativeStringProtocol::Jim084 => NativeVariableDeclarationProtocol::Jim084,
        })
    }
}
