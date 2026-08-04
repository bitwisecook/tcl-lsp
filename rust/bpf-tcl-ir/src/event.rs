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
//!
//! The event table itself is **registry data**
//! ([`tcl_registry::bpf_op::BPF_EVENTS`]): each event pairs its name and
//! aliases with a program type, ELF section convention, and verdict set.
//! This module only maps the registry's dependency-free descriptor onto the
//! IR's [`ProgType`] — adding an event is a registry edit, not a new match
//! arm here.

use tcl_registry::bpf_op::{BpfEventProgType, BpfEventSpec, lookup_bpf_event};

use crate::ir::ProgType;

/// Resolve an event name (as written in `when <EVENT> …`) to its registry
/// spec. Case-insensitive, aliases included. `None` for unknown events.
#[must_use]
pub fn event_spec(event: &str) -> Option<&'static BpfEventSpec> {
    lookup_bpf_event(event)
}

/// Resolve an event name (as written in `when <EVENT> …`) to a program type.
/// Case-insensitive. Returns `None` for unknown events.
#[must_use]
pub fn event_to_prog_type(event: &str) -> Option<ProgType> {
    event_spec(event).map(|spec| prog_type_of(spec.prog_type))
}

/// Map the registry's dependency-free program-type enum onto the IR's.
#[must_use]
pub fn prog_type_of(pt: BpfEventProgType) -> ProgType {
    match pt {
        BpfEventProgType::SocketFilter => ProgType::SocketFilter,
        BpfEventProgType::Xdp => ProgType::Xdp,
    }
}

/// The canonical event names, for diagnostics and help text.
#[must_use]
pub fn known_event_names() -> Vec<&'static str> {
    tcl_registry::bpf_op::bpf_event_names()
}
