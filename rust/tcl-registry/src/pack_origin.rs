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

//! Which spec pack an installed command came from — the facts a specialised
//! site's pack-fact stamp is built from.
//!
//! Held beside the command index rather than on [`crate::CommandSpec`]: it is
//! a fact about an *installation*, not about the command, so no studio form,
//! renderer, or pack row has anything to say about it (the loader that
//! installs a pack fills it, `tcl_spectcl::install`). Read it through
//! [`crate::CommandRegistry::pack_origin`].

/// The pack an installed command came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackOrigin {
    /// The pack's `speclib` name.
    pub pack: String,
    /// The content hash of the pack file that declared the command — the
    /// value its snapshot key interns, folded with any `include`d fragment.
    pub content_hash: u64,
    /// The loader's vocabulary version the pack was read under.
    pub vocabulary_version: String,
}
