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

//! Shared static command identity projections.
//!
//! The structured slot is authoritative. Display spellings exist only at Tcl
//! boundaries, while the length-framed encoding is for legacy map layers that
//! still require a string key.

use core::fmt::Write as _;

/// One command-table slot in a statically-known namespace path.
pub type StaticNamespacePath = Vec<String>;

/// One command-table slot in a statically-known namespace path.
pub type StaticCommandSlot = crate::CommandSlot<String, StaticNamespacePath>;

/// Render a namespace path for Tcl-facing output.
#[must_use]
pub fn display_namespace(path: &[String]) -> String {
    if path.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", path.join("::"))
    }
}

/// Render a command slot for Tcl-facing output.
#[must_use]
pub fn display_command(slot: &StaticCommandSlot) -> String {
    let namespace = display_namespace(&slot.namespace);
    if namespace == "::" {
        format!("::{simple}", simple = slot.simple)
    } else {
        format!("{namespace}::{simple}", simple = slot.simple)
    }
}

/// Produce an injective internal key for string-keyed compatibility maps.
///
/// Every component is length-framed, so legal colons and embedded NULs cannot
/// collide. This is an internal interchange key, never a Tcl-visible name.
#[must_use]
pub fn encode_command_slot(slot: &StaticCommandSlot) -> String {
    let mut out = String::new();
    for segment in &slot.namespace {
        write!(out, "{}:", segment.len()).expect("writing to String cannot fail");
        out.push_str(segment);
    }
    out.push('|');
    write!(out, "{}:", slot.simple.len()).expect("writing to String cannot fail");
    out.push_str(&slot.simple);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_can_collide_but_internal_encoding_cannot() {
        let first = StaticCommandSlot::new(vec!["a:".to_owned()], "p".to_owned());
        let second = StaticCommandSlot::new(vec!["a".to_owned()], ":p".to_owned());
        assert_eq!(display_command(&first), display_command(&second));
        assert_ne!(encode_command_slot(&first), encode_command_slot(&second));
    }
}
