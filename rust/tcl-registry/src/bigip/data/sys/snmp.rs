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

//! BIG-IP object specs for `sys snmp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_snmp",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["snmp"],
    },
    header_types: &[("sys", "snmp")],
    properties: &[
        BigipPropertySpec {
            name: "agent-addresses",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "agent-trap",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allowed-addresses",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("127"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-trap",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bigip-traps",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "communities",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "access",
                    value_type: ValueKind::Enum,
                    in_sections: &["communities"],
                    enum_values: &["ro", "rw"],
                    default: Some("ro"),
                    usage_flags: &["read_only"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "community-name",
                    value_type: ValueKind::String,
                    in_sections: &["communities"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["communities"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ipv6",
                    value_type: ValueKind::Enum,
                    in_sections: &["communities"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "oid-subset",
                    value_type: ValueKind::String,
                    in_sections: &["communities"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source",
                    value_type: ValueKind::String,
                    in_sections: &["communities"],
                    shape_kind: Some(ValueKind::IpAddress),
                    default: Some(
                        "default, which means allow any source address to access the MIB",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access",
            value_type: ValueKind::Enum,
            in_sections: &["communities"],
            enum_values: &["ro", "rw"],
            default: Some("ro"),
            usage_flags: &["read_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community-name",
            value_type: ValueKind::String,
            in_sections: &["communities"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["communities"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::Enum,
            in_sections: &["communities"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oid-subset",
            value_type: ValueKind::String,
            in_sections: &["communities"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::String,
            in_sections: &["communities"],
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("default, which means allow any source address to access the MIB"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "disk-monitors",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["disk-monitors"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minspace",
                    value_type: ValueKind::Integer,
                    in_sections: &["disk-monitors"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minspace-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["disk-monitors"],
                    enum_values: &["percent", "size"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path",
                    value_type: ValueKind::String,
                    in_sections: &["disk-monitors"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["disk-monitors"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minspace",
            value_type: ValueKind::Integer,
            in_sections: &["disk-monitors"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minspace-type",
            value_type: ValueKind::Enum,
            in_sections: &["disk-monitors"],
            enum_values: &["percent", "size"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["disk-monitors"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l2forward-vlan",
            value_type: ValueKind::List,
            required: true,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "load-max1",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "load-max15",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "load-max5",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "process-monitors",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["process-monitors"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-processes",
                    value_type: ValueKind::Integer,
                    in_sections: &["process-monitors"],
                    default: Some("1"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "min-processes",
                    value_type: ValueKind::Integer,
                    in_sections: &["process-monitors"],
                    default: Some("1"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "process",
                    value_type: ValueKind::String,
                    in_sections: &["process-monitors"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["process-monitors"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-processes",
            value_type: ValueKind::Integer,
            in_sections: &["process-monitors"],
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-processes",
            value_type: ValueKind::Integer,
            in_sections: &["process-monitors"],
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "process",
            value_type: ValueKind::String,
            in_sections: &["process-monitors"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmpv1",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmpv2",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sys-contact",
            value_type: ValueKind::String,
            default: Some("\"Customer Name<admin@customer"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sys-location",
            value_type: ValueKind::String,
            default: Some("Network Closet 1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sys-services",
            value_type: ValueKind::Integer,
            default: Some("78"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trap-community",
            value_type: ValueKind::String,
            default: Some("public"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trap-source",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traps",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "auth-password",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auth-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["traps"],
                    allow_none: true,
                    enum_values: &["md5", "none", "sha"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "community",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "engine-id",
                    value_type: ValueKind::Unknown,
                    in_sections: &["traps"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    required: true,
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Integer,
                    in_sections: &["traps"],
                    default: Some("162"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "privacy-password",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "privacy-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["traps"],
                    allow_none: true,
                    enum_values: &["aes", "des", "none"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "security-level",
                    value_type: ValueKind::Enum,
                    in_sections: &["traps"],
                    enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                    default: Some("no-auth-no-privacy"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "security-name",
                    value_type: ValueKind::String,
                    in_sections: &["traps"],
                    required: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "version",
                    value_type: ValueKind::Enum,
                    in_sections: &["traps"],
                    enum_values: &["1", "2c", "3"],
                    default: Some("2c"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-password",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["traps"],
            allow_none: true,
            enum_values: &["md5", "none", "sha"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "engine-id",
            value_type: ValueKind::Unknown,
            in_sections: &["traps"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            required: true,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Integer,
            in_sections: &["traps"],
            default: Some("162"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "privacy-password",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "privacy-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["traps"],
            allow_none: true,
            enum_values: &["aes", "des", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-level",
            value_type: ValueKind::Enum,
            in_sections: &["traps"],
            enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
            default: Some("no-auth-no-privacy"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-name",
            value_type: ValueKind::String,
            in_sections: &["traps"],
            required: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "version",
            value_type: ValueKind::Enum,
            in_sections: &["traps"],
            enum_values: &["1", "2c", "3"],
            default: Some("2c"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "users",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "access",
                    value_type: ValueKind::Enum,
                    in_sections: &["users"],
                    enum_values: &["ro", "rw"],
                    default: Some("ro"),
                    usage_flags: &["read_only"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auth-password",
                    value_type: ValueKind::String,
                    in_sections: &["users"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auth-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["users"],
                    allow_none: true,
                    enum_values: &["md5", "none", "sha"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["users"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "oid-subset",
                    value_type: ValueKind::String,
                    in_sections: &["users"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "privacy-password",
                    value_type: ValueKind::String,
                    in_sections: &["users"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "privacy-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["users"],
                    allow_none: true,
                    enum_values: &["aes", "des", "none"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "security-level",
                    value_type: ValueKind::Enum,
                    in_sections: &["users"],
                    enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                    default: Some("no-auth-no-privacy"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "username",
                    value_type: ValueKind::String,
                    in_sections: &["users"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access",
            value_type: ValueKind::Enum,
            in_sections: &["users"],
            enum_values: &["ro", "rw"],
            default: Some("ro"),
            usage_flags: &["read_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-password",
            value_type: ValueKind::String,
            in_sections: &["users"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["users"],
            allow_none: true,
            enum_values: &["md5", "none", "sha"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["users"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oid-subset",
            value_type: ValueKind::String,
            in_sections: &["users"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "privacy-password",
            value_type: ValueKind::String,
            in_sections: &["users"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "privacy-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["users"],
            allow_none: true,
            enum_values: &["aes", "des", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-level",
            value_type: ValueKind::Enum,
            in_sections: &["users"],
            enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
            default: Some("no-auth-no-privacy"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "username",
            value_type: ValueKind::String,
            in_sections: &["users"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "v1-traps",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "community",
                    value_type: ValueKind::String,
                    in_sections: &["v1-traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["v1-traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["v1-traps"],
                    required: true,
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Integer,
                    in_sections: &["v1-traps"],
                    default: Some("162"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community",
            value_type: ValueKind::String,
            in_sections: &["v1-traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["v1-traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["v1-traps"],
            required: true,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Integer,
            in_sections: &["v1-traps"],
            default: Some("162"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "v2-traps",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "community",
                    value_type: ValueKind::String,
                    in_sections: &["v2-traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["v2-traps"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["v2-traps"],
                    required: true,
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Integer,
                    in_sections: &["v2-traps"],
                    default: Some("162"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "community",
            value_type: ValueKind::String,
            in_sections: &["v2-traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["v2-traps"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["v2-traps"],
            required: true,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Integer,
            in_sections: &["v2-traps"],
            default: Some("162"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
