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

//! BIG-IP object specs for `sys ntp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_ntp",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["ntp"],
    },
    header_types: &[("sys", "ntp")],
    properties: &[
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "restrict",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::String,
                    in_sections: &["restrict"],
                    shape_kind: Some(ValueKind::IpAddress),
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "default-entry",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["restrict"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ignore",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "kod",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "limited",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "low-priority-trap",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mask",
                    value_type: ValueKind::String,
                    in_sections: &["restrict"],
                    shape_kind: Some(ValueKind::IpAddress),
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-modify",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-peer",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-query",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-serve-packets",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-trap",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-trust",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "non-ntp-port",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ntp-port",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "version",
                    value_type: ValueKind::Enum,
                    in_sections: &["restrict"],
                    enum_values: &["disable", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            in_sections: &["restrict"],
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-entry",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["restrict"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ignore",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "kod",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limited",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-priority-trap",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mask",
            value_type: ValueKind::String,
            in_sections: &["restrict"],
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-modify",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-peer",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-query",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-serve-packets",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-trap",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-trust",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "non-ntp-port",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ntp-port",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "version",
            value_type: ValueKind::Enum,
            in_sections: &["restrict"],
            enum_values: &["disable", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "servers",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timezone",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
