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

//! Future executable-argument lookup coordinates, independent of timing,
//! installed prefix ownership and the command table at actual callback entry.

/// The frame whose namespace and command paths are used when an executable
/// argument runs. This metadata grants neither callback reach nor bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptLookupScope {
    /// The receiving invocation's current physical frame at callback entry.
    InvokingFrame,
    /// The interpreter's global frame when a deferred callback actually runs.
    GlobalFrame,
    /// The frame of the operation that triggers an installed trace or unknown handler.
    TriggerFrame,
}

impl ScriptLookupScope {
    /// Complete authorable scope catalogue; none is the conservative default.
    pub const ALL: [Self; 3] = [Self::InvokingFrame, Self::GlobalFrame, Self::TriggerFrame];
}
