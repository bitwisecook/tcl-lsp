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

//! Checked command operands for an explicitly retained root lookup context.

use super::{NativeNameContext, NativeNameProtocol, checked_command_slot_utf8};
use tcl_core_types::ByteCommandSlot;

/// Render actual C slot components only when publication and root lookup agree.
/// Root-relative operands retain lone-colon data that a rooted report loses.
/// Missing aliases do not withdraw the actual byte binding or issue Native access.
#[must_use]
pub fn native_command_root_source_spelling(
    protocol: NativeNameProtocol,
    slot: &ByteCommandSlot,
) -> Option<String> {
    if protocol.is_jim084() {
        return None;
    }
    let projected = checked_command_slot_utf8(slot).ok()?;
    let mut components = projected.namespace;
    components.push(projected.simple);
    let spelling = components.join("::");
    let context = NativeNameContext::root();
    (protocol
        .command_publication_slot(context, spelling.as_bytes())
        .ok()?
        == *slot
        && protocol
            .command_lookup_slot(context, spelling.as_bytes())
            .ok()?
            == *slot)
        .then_some(spelling)
}
