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

//! Authentic original Jim local and upcall command selection.

pub use tcl_syntax::native_jim_local::{NativeJimLocalCommand, NativeJimLocalProtocol};
impl crate::InvocationDialect {
    /// Actual pinned Jim core recipe. A C compatibility version or authored
    /// logical simulation cannot donate previous-node or cleanup authority.
    #[must_use]
    pub fn native_jim_local_protocol(self) -> Option<NativeJimLocalProtocol> {
        self.native_jim_lookup_protocol()
            .map(|_| NativeJimLocalProtocol::jim084())
    }
}
