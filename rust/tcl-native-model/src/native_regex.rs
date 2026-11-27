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

//! Original compiled-pattern recipes selected by the actual C string issuer.

impl crate::InvocationDialect {
    /// Select the actual C issuer's compiled-pattern protocol independently of
    /// logical grammar. Missing or Jim issuers supply no C `RegExp` authority.
    #[must_use]
    pub fn native_regex_protocol(self) -> Option<tcl_syntax::native_regex::NativeRegexpRecipe> {
        self.native_string_protocol()?
            .tcl_version()
            .map(tcl_syntax::native_regex::NativeRegexpRecipe::for_version)
    }
}

impl crate::InvocationDialect {
    /// Select only the actual bundled Jim issuer, independently of C ARE.
    #[must_use]
    pub fn native_jim_regex_protocol(self) -> Option<tcl_syntax::native_regex::JimRegexpRecipe> {
        (self.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084))
        .then(tcl_syntax::native_regex::JimRegexpRecipe::jim084)
    }
}
