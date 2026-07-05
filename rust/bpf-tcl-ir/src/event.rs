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

//! The BPF-native event space. Inspired by F5's `when <EVENT>` shape, but a
//! whole separate namespace — these events map to eBPF program types / attach
//! points, not F5 iRules events.

use crate::ir::ProgType;

/// Resolve an event name (as written in `when <EVENT> …`) to a program type.
/// Case-insensitive. Returns `None` for unknown events.
#[must_use]
pub fn event_to_prog_type(event: &str) -> Option<ProgType> {
    match event.to_ascii_uppercase().as_str() {
        "SOCKET_FILTER" | "SOCKET" => Some(ProgType::SocketFilter),
        "XDP" => Some(ProgType::Xdp),
        _ => None,
    }
}

/// The known event names, for diagnostics and help text.
pub const KNOWN_EVENTS: &[&str] = &["SOCKET_FILTER", "XDP"];
