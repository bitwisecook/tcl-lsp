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

//! Shared minimal / generic objects for the long-tail kinds.

use crate::range::Range;

/// Shared shape for every "minimal" projection — the long-tail kinds
/// that carry only the identity tuple plus a description and TMSH kind
/// label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigipMinimalObject {
    /// Leaf name.
    pub name: String,
    /// Full TMSH path (empty for singletons).
    pub full_path: String,
    /// Full TMSH kind label (e.g. `"net routing as-path"`).
    pub kind: String,
    /// Unquoted `description`, when present.
    pub description: String,
    /// Source span, when captured.
    pub range: Option<Range>,
}

/// A generic BIG-IP stanza retained when no specialised model exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigipGenericObject {
    /// tmsh module word (e.g. `"net"`, `"auth"`, `"sys"`).
    pub module: String,
    /// tmsh object-type (e.g. `"route-domain"`, `"user"`).
    pub object_type: String,
    /// Identifier (e.g. `"/Common/0"`, `"admin"`, or `""` for singletons).
    pub identifier: String,
    /// Raw header text.
    pub header: String,
    /// Raw stanza body. This is retained for registry-driven validation of
    /// references on object kinds that do not yet have a bespoke model.
    pub body: String,
    /// Source span, when captured.
    pub range: Option<Range>,
}
