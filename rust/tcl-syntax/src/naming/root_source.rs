// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
