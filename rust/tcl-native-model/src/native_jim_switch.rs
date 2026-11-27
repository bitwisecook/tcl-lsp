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

//! Actual Jim switch issuer, separate from logical C option grammars.
pub use tcl_syntax::native_jim_switch::NativeJimSwitchProtocol;
impl crate::InvocationDialect {
    /// Select the pinned Jim core switch's original option and matching recipe.
    #[must_use]
    pub fn native_jim_switch_protocol(self) -> Option<NativeJimSwitchProtocol> {
        self.native_string_protocol()
            .filter(|strings| strings.is_jim084())
            .map(|_| NativeJimSwitchProtocol::jim084())
    }
}
