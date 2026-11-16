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

//! BIG-IP object specs for `ltm eviction-policy`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_eviction_policy",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["eviction-policy"],
    },
    header_types: &[("ltm", "eviction-policy")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "high-water",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-water",
            value_type: ValueKind::Integer,
            default: Some("85 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "slow-flow",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["slow-flow"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "eviction-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["slow-flow"],
                    enum_values: &["count", "percent"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "grace-period",
                    value_type: ValueKind::Integer,
                    in_sections: &["slow-flow"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "maximum",
                    value_type: ValueKind::Integer,
                    in_sections: &["slow-flow"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold-bps",
                    value_type: ValueKind::Integer,
                    in_sections: &["slow-flow"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "throttling",
                    value_type: ValueKind::Enum,
                    in_sections: &["slow-flow"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["slow-flow"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "eviction-type",
            value_type: ValueKind::Enum,
            in_sections: &["slow-flow"],
            enum_values: &["count", "percent"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "grace-period",
            value_type: ValueKind::Integer,
            in_sections: &["slow-flow"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "maximum",
            value_type: ValueKind::Integer,
            in_sections: &["slow-flow"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold-bps",
            value_type: ValueKind::Integer,
            in_sections: &["slow-flow"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "throttling",
            value_type: ValueKind::Enum,
            in_sections: &["slow-flow"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "strategies",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "bias-bytes",
                    value_type: ValueKind::List,
                    in_sections: &["strategies"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bias-idle",
                    value_type: ValueKind::Unknown,
                    in_sections: &["strategies"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bias-oldest",
                    value_type: ValueKind::Unknown,
                    in_sections: &["strategies"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "low-priority-geographies",
                    value_type: ValueKind::List,
                    in_sections: &["strategies"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "low-priority-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["strategies"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "low-priority-route-domain",
                    value_type: ValueKind::Unknown,
                    in_sections: &["strategies"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "low-priority-virtual-server",
                    value_type: ValueKind::Unknown,
                    in_sections: &["strategies"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bias-bytes",
            value_type: ValueKind::List,
            in_sections: &["strategies"],
            block: &[
                BigipPropertySpec {
                    name: "delay",
                    value_type: ValueKind::Integer,
                    in_sections: &["strategies", "bias-bytes"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "bias-bytes"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "delay",
            value_type: ValueKind::Integer,
            in_sections: &["strategies", "bias-bytes"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "bias-bytes"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bias-idle",
            value_type: ValueKind::Unknown,
            in_sections: &["strategies"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["strategies", "bias-idle"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "bias-idle"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bias-oldest",
            value_type: ValueKind::Unknown,
            in_sections: &["strategies"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["strategies", "bias-oldest"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "bias-oldest"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-priority-geographies",
            value_type: ValueKind::List,
            in_sections: &["strategies"],
            block: &[
                BigipPropertySpec {
                    name: "countries",
                    value_type: ValueKind::List,
                    in_sections: &["strategies", "low-priority-geographies"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "low-priority-geographies"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "countries",
            value_type: ValueKind::List,
            in_sections: &["strategies", "low-priority-geographies"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "low-priority-geographies"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-priority-port",
            value_type: ValueKind::Unknown,
            in_sections: &["strategies"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "low-priority-port"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ports",
                    value_type: ValueKind::List,
                    in_sections: &["strategies", "low-priority-port"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "low-priority-port"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ports",
            value_type: ValueKind::List,
            in_sections: &["strategies", "low-priority-port"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["strategies", "low-priority-port", "ports"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port-number",
                    value_type: ValueKind::Reference,
                    in_sections: &["strategies", "low-priority-port", "ports"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "low-priority-port", "ports"],
                    enum_values: &["any", "sctp", "tcp", "udp"],
                    default: Some("any"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["strategies", "low-priority-port", "ports"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-number",
            value_type: ValueKind::Reference,
            in_sections: &["strategies", "low-priority-port", "ports"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "low-priority-port", "ports"],
            enum_values: &["any", "sctp", "tcp", "udp"],
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-priority-route-domain",
            value_type: ValueKind::Unknown,
            in_sections: &["strategies"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "low-priority-route-domain"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "names",
                    value_type: ValueKind::List,
                    in_sections: &["strategies", "low-priority-route-domain"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "low-priority-route-domain"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "names",
            value_type: ValueKind::List,
            in_sections: &["strategies", "low-priority-route-domain"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-priority-virtual-server",
            value_type: ValueKind::Unknown,
            in_sections: &["strategies"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["strategies", "low-priority-virtual-server"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "names",
                    value_type: ValueKind::List,
                    in_sections: &["strategies", "low-priority-virtual-server"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["strategies", "low-priority-virtual-server"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "names",
            value_type: ValueKind::List,
            in_sections: &["strategies", "low-priority-virtual-server"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
