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

//! Jim switch's original two-byte option scan, without object/cache authority.

/// Pure pinned Jim switch recipe; live engines independently select its issuer.
#[derive(Clone, Copy, Debug)]
pub struct NativeJimSwitchProtocol(());
/// One reached original switch option, not an Enum lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimSwitchOption {
    /// This original argument starts the subject/case vector.
    Subject,
    /// End option scanning after this argument.
    End,
    /// Select counted exact object equality.
    Exact,
    /// Select the original Jim glob matcher.
    Glob,
    /// Select a fresh original regexp command and retain the -- flag.
    Regexp,
    /// Consume the following original command-name object.
    Command,
    /// Original diagnostic input for an unknown option.
    Invalid,
}
impl NativeJimSwitchProtocol {
    /// Pinned pure recipe, without interpreter authority.
    #[must_use]
    pub const fn jim084() -> Self {
        Self(())
    }
    /// Reach Jim's strncmp(option, literal, 2) scan on original counted bytes.
    /// The scanner installs no Enum or `ComparedString` primary.
    #[must_use]
    pub fn option(self, original: &[u8]) -> NativeJimSwitchOption {
        use NativeJimSwitchOption as O;
        if original.first() != Some(&b'-') {
            return O::Subject;
        }
        match original.get(1).copied().unwrap_or(0) {
            b'-' => O::End,
            b'e' => O::Exact,
            b'g' => O::Glob,
            b'r' => O::Regexp,
            b'c' => O::Command,
            _ => O::Invalid,
        }
    }
}
