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

//! BIG-IP object specs for `ltm profile dhcpv4`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_dhcpv4",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile dhcpv4"],
    },
    header_types: &[("ltm", "profile dhcpv4")],
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
                        "is",
                        "mac-and-relay-option",
                        "option",
                        "relay-agent",
                        "relay-option",
                        "separator1",
                        "symbol",
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
                    name: "suboption-id1",
                    value_type: ValueKind::Integer,
                    in_sections: &["authentication", "user-name"],
                    default: Some("1"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suboption-id2",
                    value_type: ValueKind::Integer,
                    in_sections: &["authentication", "user-name"],
                    default: Some("2"),
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
                "is",
                "mac-and-relay-option",
                "option",
                "relay-agent",
                "relay-option",
                "separator1",
                "symbol",
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
            name: "suboption-id1",
            value_type: ValueKind::Integer,
            in_sections: &["authentication", "user-name"],
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suboption-id2",
            value_type: ValueKind::Integer,
            in_sections: &["authentication", "user-name"],
            default: Some("2"),
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
            references: &["ltm_profile_dhcpv4"],
            default: Some("dhcpv4"),
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
            name: "max-hops",
            value_type: ValueKind::Integer,
            default: Some("4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["forwarding", "relay"],
            default: Some("relay"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "relay-agent-id",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "add",
                    value_type: ValueKind::Enum,
                    in_sections: &["relay-agent-id"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remove",
                    value_type: ValueKind::Enum,
                    in_sections: &["relay-agent-id"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suboption",
                    value_type: ValueKind::Unknown,
                    in_sections: &["relay-agent-id"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "add",
            value_type: ValueKind::Enum,
            in_sections: &["relay-agent-id"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove",
            value_type: ValueKind::Enum,
            in_sections: &["relay-agent-id"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suboption",
            value_type: ValueKind::Unknown,
            in_sections: &["relay-agent-id"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "id1",
                    value_type: ValueKind::Integer,
                    in_sections: &["relay-agent-id", "suboption"],
                    default: Some("1"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "id2",
                    value_type: ValueKind::Integer,
                    in_sections: &["relay-agent-id", "suboption"],
                    default: Some("2"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value1",
                    value_type: ValueKind::Enum,
                    in_sections: &["relay-agent-id", "suboption"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value2",
                    value_type: ValueKind::Enum,
                    in_sections: &["relay-agent-id", "suboption"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "id1",
            value_type: ValueKind::Integer,
            in_sections: &["relay-agent-id", "suboption"],
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "id2",
            value_type: ValueKind::Integer,
            in_sections: &["relay-agent-id", "suboption"],
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value1",
            value_type: ValueKind::Enum,
            in_sections: &["relay-agent-id", "suboption"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value2",
            value_type: ValueKind::Enum,
            in_sections: &["relay-agent-id", "suboption"],
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
                        "is",
                        "mac-and-relay-id",
                        "option",
                        "relay-agent",
                        "separator1",
                        "symbol",
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
                    name: "suboption-id1",
                    value_type: ValueKind::Integer,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    default: Some("1"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suboption-id2",
                    value_type: ValueKind::Integer,
                    in_sections: &["subscriber-discovery", "subscriber-id"],
                    default: Some("2"),
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
                "is",
                "mac-and-relay-id",
                "option",
                "relay-agent",
                "separator1",
                "symbol",
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
            name: "suboption-id1",
            value_type: ValueKind::Integer,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suboption-id2",
            value_type: ValueKind::Integer,
            in_sections: &["subscriber-discovery", "subscriber-id"],
            default: Some("2"),
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
            name: "transaction-timeout",
            value_type: ValueKind::Integer,
            default: Some("45 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ttl-dec-value",
            value_type: ValueKind::Enum,
            enum_values: &["by-0", "by-1", "by-2", "by-4"],
            default: Some("by-1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ttl-value",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
