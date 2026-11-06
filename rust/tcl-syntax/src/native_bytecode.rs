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

//! Original C executable-source storage rules, independent of admission.

use tcl_dialect::TclVersion;

/// A pure C Bytecode storage recipe; it grants no executable artifact authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBytecodeStorageRecipe(TclVersion);
impl NativeBytecodeStorageRecipe {
    /// Select a pure release recipe, without authenticating a compiler.
    #[must_use]
    pub const fn for_version(version: TclVersion) -> Self {
        Self(version)
    }
    /// Native origin of a genuinely published original-source primary.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.0
    }
    /// Modern C retains its executable parse-error body; C84 declines publication.
    #[must_use]
    pub const fn retains_parse_failure(self) -> bool {
        !matches!(self.0, TclVersion::V8_4)
    }

    /// Selected `TclProcCleanupProc` clears a surviving ByteCode.procPtr.
    /// C84/C85 release the body without this modern context mutation.
    #[must_use]
    pub const fn clears_retired_procedure_context(self) -> bool {
        matches!(
            self.0,
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
        )
    }
}
