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

//! BIG-IP object specs for `net routing profile`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_routing_profile_bgp",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["routing profile bgp"],
    },
    header_types: &[("net", "routing profile bgp")],
    properties: &[
        BigipPropertySpec {
            name: "adj-out",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "aggregate-nexthop-check",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-local-count",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-multiple-instance",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Boolean,
            allow_none: true,
            references: &["net_routing_profile_bgp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Boolean,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extended-asn-cap",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-paths",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "ebgp",
                    value_type: ValueKind::Integer,
                    in_sections: &["max-paths"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ibgp",
                    value_type: ValueKind::Integer,
                    in_sections: &["max-paths"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ebgp",
            value_type: ValueKind::Integer,
            in_sections: &["max-paths"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ibgp",
            value_type: ValueKind::Integer,
            in_sections: &["max-paths"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "nexthop-trigger",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "delay",
                    value_type: ValueKind::Integer,
                    in_sections: &["nexthop-trigger"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "state",
                    value_type: ValueKind::Enum,
                    in_sections: &["nexthop-trigger"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "delay",
            value_type: ValueKind::Integer,
            in_sections: &["nexthop-trigger"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            in_sections: &["nexthop-trigger"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rfc1771",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "path-select",
                    value_type: ValueKind::Enum,
                    in_sections: &["rfc1771"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "strict",
                    value_type: ValueKind::Enum,
                    in_sections: &["rfc1771"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path-select",
            value_type: ValueKind::Enum,
            in_sections: &["rfc1771"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "strict",
            value_type: ValueKind::Enum,
            in_sections: &["rfc1771"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-id",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
