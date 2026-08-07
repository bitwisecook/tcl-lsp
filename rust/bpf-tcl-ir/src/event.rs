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

/// Resolve an event name to a *codegen-ready* IR program type. Case-insensitive.
/// Returns `None` for an unknown event **or** an event whose codegen is still
/// pending (TC, cgroup) — the front-end distinguishes the two so it can give a
/// precise diagnostic (see [`event_is_described`]).
#[must_use]
pub fn event_to_prog_type(event: &str) -> Option<ProgType> {
    event_spec(event).and_then(|spec| prog_type_of(spec.prog_type))
}

/// Whether `event` resolves to a registry-described event whose codegen is not
/// yet implemented (a schema-only contract). `false` for unknown or ready
/// events.
#[must_use]
pub fn event_is_described(event: &str) -> bool {
    event_spec(event).is_some_and(|spec| !spec.is_codegen_ready())
}

/// Map the registry's dependency-free program-type enum onto the IR's, for the
/// codegen-ready targets. `None` for events the IR has no lowering for yet
/// (none today — issue #1310 completed TC/cgroup codegen; kept `Option` so a
/// future event can land its schema ahead of its codegen, as TC/cgroup did).
#[must_use]
pub fn prog_type_of(pt: BpfEventProgType) -> Option<ProgType> {
    match pt {
        BpfEventProgType::SocketFilter => Some(ProgType::SocketFilter),
        BpfEventProgType::Xdp => Some(ProgType::Xdp),
        BpfEventProgType::TcIngress => Some(ProgType::TcIngress),
        BpfEventProgType::TcEgress => Some(ProgType::TcEgress),
        BpfEventProgType::CgroupInet4Connect => Some(ProgType::CgroupInet4Connect),
        BpfEventProgType::CgroupInet4Bind => Some(ProgType::CgroupInet4Bind),
    }
}

/// The inverse of [`prog_type_of`]: the registry's dependency-free family for
/// an IR program type, for consulting registry-owned per-family policy (e.g.
/// [`tcl_registry::bpf_op::BpfProgTypeSet::contains`]) from lowering without
/// the registry depending back on the IR.
#[must_use]
pub fn registry_prog_type(pt: ProgType) -> BpfEventProgType {
    match pt {
        ProgType::SocketFilter => BpfEventProgType::SocketFilter,
        ProgType::Xdp => BpfEventProgType::Xdp,
        ProgType::TcIngress => BpfEventProgType::TcIngress,
        ProgType::TcEgress => BpfEventProgType::TcEgress,
        ProgType::CgroupInet4Connect => BpfEventProgType::CgroupInet4Connect,
        ProgType::CgroupInet4Bind => BpfEventProgType::CgroupInet4Bind,
    }
}

/// The canonical event names, for diagnostics and help text.
#[must_use]
pub fn known_event_names() -> Vec<&'static str> {
    tcl_registry::bpf_op::bpf_event_names()
}

/// The canonical names of events that currently compile to a program.
#[must_use]
pub fn codegen_ready_event_names() -> Vec<&'static str> {
    tcl_registry::bpf_op::codegen_ready_event_names()
}
