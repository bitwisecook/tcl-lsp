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

//! BIG-IP object specs for `net bwc`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc policy"],
        },
        header_types: &[("net", "bwc policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "categories",
                value_type: ValueKind::List,
                required: true,
                block: &[
                    BigipPropertySpec {
                        name: "ip-tos",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        enum_values: &["pass-through"],
                        default: Some("pass-through, which indicates, do not modify UDP packets"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "link-qos",
                        value_type: ValueKind::Enum,
                        in_sections: &["categories"],
                        enum_values: &["pass-through"],
                        default: Some("pass-through, which indicates, do not modify UDP packets"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-cat-rate",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-cat-rate-percentage",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "traffic-priority-map",
                        value_type: ValueKind::String,
                        in_sections: &["categories"],
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tos",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-qos",
                value_type: ValueKind::Enum,
                in_sections: &["categories"],
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-cat-rate",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-cat-rate-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-priority-map",
                value_type: ValueKind::String,
                in_sections: &["categories"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Unknown,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tos",
                value_type: ValueKind::Integer,
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-qos",
                value_type: ValueKind::Enum,
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-period",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-user-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-user-rate-pps",
                value_type: ValueKind::Integer,
                default: Some("0 (not configured)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measure",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-priority-map",
                value_type: ValueKind::String,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_priority_group",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc priority-group"],
        },
        header_types: &[("net", "bwc priority-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority-classes",
                value_type: ValueKind::List,
                block: &[
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["priority-classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight-percentage",
                        value_type: ValueKind::Integer,
                        in_sections: &["priority-classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_traffic_group",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc traffic-group"],
        },
        header_types: &[("net", "bwc traffic-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority-classes",
                value_type: ValueKind::List,
                block: &[BigipPropertySpec {
                    name: "weight-percentage",
                    value_type: ValueKind::Integer,
                    in_sections: &["priority-classes"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
