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

//! BIG-IP object specs for `ltm snat-translation`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_snat_translation",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["snat-translation"],
    },
    header_types: &[("ltm", "snat-translation")],
    properties: &[
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            requires_route_domain: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "arp",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connection-limit",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("indefinite"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("indefinite"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traffic-group",
            value_type: ValueKind::String,
            allow_none: true,
            references: &["cm_traffic_group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "udp-idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("indefinite"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
