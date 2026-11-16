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

//! BIG-IP object specs for `sys ha-group`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_ha_group",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["ha-group"],
    },
    header_types: &[("sys", "ha-group")],
    properties: &[
        BigipPropertySpec {
            name: "active-bonus",
            value_type: ValueKind::Integer,
            default: Some("10 (ten)"),
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
            name: "clusters",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["clusters"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "attribute",
                    value_type: ValueKind::Unknown,
                    in_sections: &["clusters"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minimum-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["clusters"],
                    default: Some("0 (zero), which indicates this option is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sufficient",
                    value_type: ValueKind::Enum,
                    in_sections: &["clusters"],
                    enum_values: &["all"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["clusters"],
                    usage_flags: &["deprecated"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "weight",
                    value_type: ValueKind::Integer,
                    in_sections: &["clusters"],
                    default: Some("10"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["clusters"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute",
            value_type: ValueKind::Unknown,
            in_sections: &["clusters"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minimum-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["clusters"],
            default: Some("0 (zero), which indicates this option is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sufficient",
            value_type: ValueKind::Enum,
            in_sections: &["clusters"],
            enum_values: &["all"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold",
            value_type: ValueKind::Integer,
            in_sections: &["clusters"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Integer,
            in_sections: &["clusters"],
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pools",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["pools"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "attribute",
                    value_type: ValueKind::Unknown,
                    in_sections: &["pools"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minimum-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["pools"],
                    default: Some("0 (zero), which indicates this option is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sufficient",
                    value_type: ValueKind::Integer,
                    in_sections: &["pools"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["pools"],
                    usage_flags: &["deprecated"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "weight",
                    value_type: ValueKind::Integer,
                    in_sections: &["pools"],
                    default: Some("10"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["pools"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute",
            value_type: ValueKind::Unknown,
            in_sections: &["pools"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minimum-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["pools"],
            default: Some("0 (zero), which indicates this option is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sufficient",
            value_type: ValueKind::Integer,
            in_sections: &["pools"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold",
            value_type: ValueKind::Integer,
            in_sections: &["pools"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Integer,
            in_sections: &["pools"],
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trunks",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["trunks"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "attribute",
                    value_type: ValueKind::Unknown,
                    in_sections: &["trunks"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minimum-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    default: Some("0 (zero), which indicates this option is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sufficient",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    usage_flags: &["deprecated"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "weight",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    default: Some("10"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["trunks"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute",
            value_type: ValueKind::Unknown,
            in_sections: &["trunks"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minimum-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            default: Some("0 (zero), which indicates this option is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sufficient",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
