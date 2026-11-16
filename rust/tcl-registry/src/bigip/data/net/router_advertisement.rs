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

//! BIG-IP object specs for `net router-advertisement`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_router_advertisement",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["router-advertisement"],
    },
    header_types: &[("net", "router-advertisement")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "autonomous",
            value_type: ValueKind::Unknown,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "current-hop-limit",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "disabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-interval",
            value_type: ValueKind::Integer,
            default: Some("600"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-interval",
            value_type: ValueKind::Integer,
            default: Some("200"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mtu",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-other-config",
            value_type: ValueKind::Unknown,
            default: Some("0 zero"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "on-link",
            value_type: ValueKind::Unknown,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "preferred-lifetime",
            value_type: ValueKind::Integer,
            default: Some("604800"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix-length",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefixes",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reachable-time",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "retransmit-timer",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-lifetime",
            value_type: ValueKind::Integer,
            default: Some("1800"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unmanaged",
            value_type: ValueKind::Unknown,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "valid-lifetime",
            value_type: ValueKind::Integer,
            default: Some("2592000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlan",
            value_type: ValueKind::Reference,
            references: &["net_vlan", "net_vlan_allowed", "net_vlan_group"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
