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

//! Jim's original subcommand cache, independent of Enum and C Index primaries.

use crate::NativeIndexCache;
use alloc::sync::Arc;

/// Retained original subcommand table and selected entry. This is storage;
/// the selected native parser separately authenticates the issuing table.
#[derive(Clone, Debug)]
pub struct NativeJimSubcommandCache {
    entry: NativeIndexCache,
    owner: Arc<()>,
}

impl NativeJimSubcommandCache {
    /// Retain an independently owned original subcommand declaration receipt.
    #[must_use]
    pub fn new(entry: NativeIndexCache, owner: Arc<()>) -> Self {
        Self { entry, owner }
    }

    /// Compare the independently held actual table birth, never reconstructing it.
    #[must_use]
    pub fn table_owner_is(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }

    /// Equality key for the retained table, never pointer access authority.
    #[must_use]
    pub fn table_identity(&self) -> usize {
        self.entry.table_identity()
    }

    /// Selected entry in that original table.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.entry.index()
    }
}
