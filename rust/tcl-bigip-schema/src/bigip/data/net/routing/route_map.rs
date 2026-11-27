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

//! BIG-IP object specs for `net routing route-map`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_routing_route_map",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["routing route-map"],
    },
    header_types: &[("net", "routing route-map")],
    properties: &[
        BigipPropertySpec {
            name: "access-list",
            value_type: ValueKind::Boolean,
            in_sections: &["address"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix-list",
            value_type: ValueKind::Boolean,
            in_sections: &["address"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            in_sections: &["aggregator"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as",
            value_type: ValueKind::Integer,
            in_sections: &["aggregator"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "additive",
            value_type: ValueKind::Boolean,
            in_sections: &["community"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "exact-match",
            value_type: ValueKind::Boolean,
            in_sections: &["community"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "exact-set",
            value_type: ValueKind::Boolean,
            in_sections: &["community"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Integer,
            in_sections: &["community"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reachability-half-life",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reuse",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suppress",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suppress-max",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unreachability-half-life",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Boolean,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Boolean,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "match",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "set",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Boolean,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "set",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "exact-match",
            value_type: ValueKind::Boolean,
            in_sections: &["extcommunity"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rt",
            value_type: ValueKind::Boolean,
            in_sections: &["extcommunity"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "soo",
            value_type: ValueKind::Boolean,
            in_sections: &["extcommunity"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["ip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            in_sections: &["ipv4"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["ipv4"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer",
            value_type: ValueKind::String,
            in_sections: &["ipv4"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            in_sections: &["ipv6"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["ipv6"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer",
            value_type: ValueKind::String,
            in_sections: &["ipv6"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-path",
            value_type: ValueKind::Boolean,
            in_sections: &["match"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community",
            value_type: ValueKind::String,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extcommunity",
            value_type: ValueKind::String,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv4",
            value_type: ValueKind::String,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::String,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metric",
            value_type: ValueKind::Integer,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "origin",
            value_type: ValueKind::Boolean,
            in_sections: &["match"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-type",
            value_type: ValueKind::Boolean,
            in_sections: &["match"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::Integer,
            in_sections: &["match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlan",
            value_type: ValueKind::Reference,
            in_sections: &["match"],
            allow_none: true,
            references: &["net_vlan"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Boolean,
            in_sections: &["metric"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Boolean,
            in_sections: &["metric"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-list",
            value_type: ValueKind::Boolean,
            in_sections: &["next-hop"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            in_sections: &["next-hop"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local",
            value_type: ValueKind::String,
            in_sections: &["next-hop"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix-list",
            value_type: ValueKind::Boolean,
            in_sections: &["next-hop"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-list",
            value_type: ValueKind::Boolean,
            in_sections: &["peer"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "aggregator",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-path-prepend",
            value_type: ValueKind::Integer,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "atomic-aggregate",
            value_type: ValueKind::Boolean,
            in_sections: &["set"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dampening",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extcommunity",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "level",
            value_type: ValueKind::Boolean,
            in_sections: &["set"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-preference",
            value_type: ValueKind::Integer,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metric",
            value_type: ValueKind::String,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "origin",
            value_type: ValueKind::Boolean,
            in_sections: &["set"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "originator-id",
            value_type: ValueKind::Integer,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::Integer,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Integer,
            in_sections: &["set"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
