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

//! BIG-IP object specs for `ltm profile dhcpv6`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_dhcpv6",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile dhcpv6"],
    },
    header_types: &[("ltm", "profile dhcpv6")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "authentication",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["authentication"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "user-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["authentication"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "virtual",
                    value_type: ValueKind::String,
                    in_sections: &["authentication"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["authentication"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-name",
            value_type: ValueKind::Unknown,
            in_sections: &["authentication"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "format",
                    value_type: ValueKind::Enum,
                    in_sections: &["authentication", "user-name"],
                    enum_values: &[
                        "address",
                        "as",
                        "concatenation",
                        "defined",
                        "mac",
                        "mac-and-option37",
                        "mac-and-option37-and-option38",
                        "mac-and-option38",
                        "option37",
                        "option37-and-option38",
                        "option38",
                        "separator1",
                        "separator2",
                        "symbols",
                        "tcl-snippet",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "separator1",
                    value_type: ValueKind::String,
                    in_sections: &["authentication", "user-name"],
                    allow_none: true,
                    default: Some("@"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "separator2",
                    value_type: ValueKind::String,
                    in_sections: &["authentication", "user-name"],
                    allow_none: true,
                    default: Some("@"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcl",
                    value_type: ValueKind::String,
                    in_sections: &["authentication", "user-name"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "format",
            value_type: ValueKind::Enum,
            in_sections: &["authentication", "user-name"],
            enum_values: &[
                "address",
                "as",
                "concatenation",
                "defined",
                "mac",
                "mac-and-option37",
                "mac-and-option37-and-option38",
                "mac-and-option38",
                "option37",
                "option37-and-option38",
                "option38",
                "separator1",
                "separator2",
                "symbols",
                "tcl-snippet",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "separator1",
            value_type: ValueKind::String,
            in_sections: &["authentication", "user-name"],
            allow_none: true,
            default: Some("@"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "separator2",
            value_type: ValueKind::String,
            in_sections: &["authentication", "user-name"],
            allow_none: true,
            default: Some("@"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcl",
            value_type: ValueKind::String,
            in_sections: &["authentication", "user-name"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "virtual",
            value_type: ValueKind::String,
            in_sections: &["authentication"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-lease-time",
            value_type: ValueKind::Integer,
            default: Some("86400"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_dhcpv6"],
            default: Some("dhcpv6"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("60 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lease-query-max-retry",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lease-query-only",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["forwarding", "relay"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-id-option",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "add",
                    value_type: ValueKind::Enum,
                    in_sections: &["remote-id-option"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enterprise-number",
                    value_type: ValueKind::Integer,
                    in_sections: &["remote-id-option"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remove",
                    value_type: ValueKind::Enum,
                    in_sections: &["remote-id-option"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::Enum,
                    in_sections: &["remote-id-option"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "add",
            value_type: ValueKind::Enum,
            in_sections: &["remote-id-option"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enterprise-number",
            value_type: ValueKind::Integer,
            in_sections: &["remote-id-option"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove",
            value_type: ValueKind::Enum,
            in_sections: &["remote-id-option"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Enum,
            in_sections: &["remote-id-option"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subscriber-discovery",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["subscriber-discovery"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "subscriber-id",
                    value_type: ValueKind::Unknown,
                    in_sections: &["subscriber-discovery"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["subscriber-discovery"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subscriber-id",
            value_type: ValueKind::Unknown,
            in_sections: &["subscriber-discovery"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "format",
                    value_type: ValueKind::Enum,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    enum_values: &[
                        "address",
                        "as",
                        "concatenation",
                        "defined",
                        "mac",
                        "mac-and-option37",
                        "mac-and-option37-and-option38",
                        "mac-and-option38",
                        "option37",
                        "option37-and-option38",
                        "option38",
                        "separator1",
                        "separator2",
                        "symbols",
                        "tcl-snippet",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "separator1",
                    value_type: ValueKind::String,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    allow_none: true,
                    default: Some("@"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "separator2",
                    value_type: ValueKind::String,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    allow_none: true,
                    default: Some("@"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcl",
                    value_type: ValueKind::String,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "format",
            value_type: ValueKind::Enum,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            enum_values: &[
                "address",
                "as",
                "concatenation",
                "defined",
                "mac",
                "mac-and-option37",
                "mac-and-option37-and-option38",
                "mac-and-option38",
                "option37",
                "option37-and-option38",
                "option38",
                "separator1",
                "separator2",
                "symbols",
                "tcl-snippet",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "separator1",
            value_type: ValueKind::String,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            allow_none: true,
            default: Some("@"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "separator2",
            value_type: ValueKind::String,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            allow_none: true,
            default: Some("@"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcl",
            value_type: ValueKind::String,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subscriber-id-option",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "add",
                    value_type: ValueKind::Enum,
                    in_sections: &["subscriber-id-option"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remove",
                    value_type: ValueKind::Enum,
                    in_sections: &["subscriber-id-option"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::Enum,
                    in_sections: &["subscriber-id-option"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "add",
            value_type: ValueKind::Enum,
            in_sections: &["subscriber-id-option"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove",
            value_type: ValueKind::Enum,
            in_sections: &["subscriber-id-option"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Enum,
            in_sections: &["subscriber-id-option"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transaction-timeout",
            value_type: ValueKind::Integer,
            default: Some("45 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
