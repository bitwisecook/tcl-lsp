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

//! Actual C executable-source storage selection.

use tcl_dialect::model::Family;
use tcl_syntax::native_bytecode::NativeBytecodeStorageRecipe;

/// Actual engine selection; the caller still supplies the admitted executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBytecodeStorageProtocol(NativeBytecodeStorageRecipe);
impl NativeBytecodeStorageProtocol {
    /// Pure physical storage recipe; not a compiler or original-object receipt.
    #[must_use]
    pub const fn recipe(self) -> NativeBytecodeStorageRecipe {
        self.0
    }
}
impl crate::InvocationDialect {
    /// Select actual native C storage, without deriving it from logical source grammar.
    #[must_use]
    pub fn native_bytecode_storage_protocol(self) -> Option<NativeBytecodeStorageProtocol> {
        (self.family()? == Family::Tcl).then_some(NativeBytecodeStorageProtocol(
            NativeBytecodeStorageRecipe::for_version(self.tcl_version?),
        ))
    }
}
