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

//! BIG-IP object specs for `security blacklist-publisher`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_blacklist_publisher_category",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["blacklist-publisher category"],
        },
        header_types: &[("security", "blacklist-publisher category")],
        properties: &[BigipPropertySpec {
            name: "profile-names",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_blacklist_publisher_profile",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["blacklist-publisher profile"],
        },
        header_types: &[("security", "blacklist-publisher profile")],
        properties: &[
            BigipPropertySpec {
                name: "bgp-flowspec-advertisement-action",
                value_type: ValueKind::Enum,
                enum_values: &["drop", "qos", "rate-limit"],
                default: Some("drop"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bgp-flowspec-dscp-value",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bgp-flowspec-rate-limit",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-advertisement-nexthop",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-advertisement-nexthop-v6",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::Reference,
                references: &["cm_traffic_group", "net_bwc_traffic_group"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
