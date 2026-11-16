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

//! BIG-IP object specs for `net wccp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_wccp",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["wccp"],
    },
    header_types: &[("net", "wccp")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-timeout",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "services",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "alt-hash-fields",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    allow_none: true,
                    enum_values: &["dest-ip", "none", "src-ip"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["services"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hash-fields",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    allow_none: true,
                    enum_values: &["dest-ip", "none", "src-ip"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "password",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    allow_none: true,
                    enum_values: &["dest", "none", "source"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ports",
                    value_type: ValueKind::Integer,
                    in_sections: &["services"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "priority",
                    value_type: ValueKind::Integer,
                    in_sections: &["services"],
                    default: Some("100"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    enum_values: &["tcp", "udp"],
                    default: Some("tcp"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "redirection-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    enum_values: &["gre", "l2"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "return-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    enum_values: &["gre", "l2"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "routers",
                    value_type: ValueKind::List,
                    in_sections: &["services"],
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "traffic-assign",
                    value_type: ValueKind::Enum,
                    in_sections: &["services"],
                    enum_values: &["hash", "mask"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tunnel-local-address",
                    value_type: ValueKind::String,
                    in_sections: &["services"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tunnel-remote-addresses",
                    value_type: ValueKind::List,
                    in_sections: &["services"],
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "weight",
                    value_type: ValueKind::Integer,
                    in_sections: &["services"],
                    default: Some("50"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alt-hash-fields",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            allow_none: true,
            enum_values: &["dest-ip", "none", "src-ip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["services"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hash-fields",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            allow_none: true,
            enum_values: &["dest-ip", "none", "src-ip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-type",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            allow_none: true,
            enum_values: &["dest", "none", "source"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ports",
            value_type: ValueKind::Integer,
            in_sections: &["services"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Integer,
            in_sections: &["services"],
            default: Some("100"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            enum_values: &["tcp", "udp"],
            default: Some("tcp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "redirection-method",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            enum_values: &["gre", "l2"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "return-method",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            enum_values: &["gre", "l2"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "routers",
            value_type: ValueKind::List,
            in_sections: &["services"],
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traffic-assign",
            value_type: ValueKind::Enum,
            in_sections: &["services"],
            enum_values: &["hash", "mask"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tunnel-local-address",
            value_type: ValueKind::String,
            in_sections: &["services"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tunnel-remote-addresses",
            value_type: ValueKind::List,
            in_sections: &["services"],
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Integer,
            in_sections: &["services"],
            default: Some("50"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
