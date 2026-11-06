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

//! Jim's original Enum and immediate-string caches, separate from C Index.
use crate::NativeIndexCache;

/// Retained static table/literal lifetime. These fields are storage, not an issuer.
#[derive(Clone, Debug)]
pub enum NativeJimOptionCache {
    /// `GetEnum` stores the exact original table, flags and selected entry.
    Enum { entry: NativeIndexCache, flags: i32 },
    /// `CompareStringImmediate` retains the exact successful static literal identity.
    ComparedString {
        entry: NativeIndexCache,
        literal_identity: usize,
    },
}
