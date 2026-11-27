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

//! BIG-IP object specs for `pem protocol`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_protocol_diameter_avp",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["protocol diameter-avp"],
        },
        header_types: &[("pem", "protocol diameter-avp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "avp-code",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-type",
                value_type: ValueKind::Integer,
                enum_values: &[
                    "enumerated",
                    "float32",
                    "float64",
                    "grouped",
                    "rat-type",
                    "time",
                    "unsigned32",
                    "unsigned64",
                ],
                default: Some("octetstring"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "length",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent-avp",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["pem_protocol_diameter_avp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vendor-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_protocol_profile_gx",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["protocol profile gx"],
        },
        header_types: &[("pem", "protocol profile gx")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cp",
                value_type: ValueKind::Reference,
                references: &[
                    "auth_source",
                    "ltm_persistence_source_addr",
                    "security_dos_auto_thresholds_top_source_ips",
                    "security_nat_destination_translation",
                    "security_nat_source_translation",
                    "sys_ipfix_destination",
                    "sys_log_config_destination_alertd",
                    "sys_log_config_destination_arcsight",
                    "sys_log_config_destination_ipfix",
                    "sys_log_config_destination_local_database",
                    "sys_log_config_destination_local_syslog",
                    "sys_log_config_destination_management_port",
                    "sys_log_config_destination_remote_high_speed_log",
                    "sys_log_config_destination_remote_syslog",
                    "sys_log_config_destination_splunk",
                    "sys_sflow_data_source_http",
                    "sys_sflow_data_source_interface",
                    "sys_sflow_data_source_system",
                    "sys_sflow_data_source_vlan",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "messages",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "avps",
                        value_type: ValueKind::List,
                        in_sections: &["messages"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages"],
                        enum_values: &["any", "in", "out"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "message-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages"],
                        enum_values: &["cca-i", "cca-u", "ccr-i", "ccr-t", "ccr-u", "raa", "rar"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "avps",
                value_type: ValueKind::List,
                in_sections: &["messages"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "diameter-avp",
                        value_type: ValueKind::Reference,
                        in_sections: &["messages", "avps"],
                        allow_none: true,
                        references: &["pem_protocol_diameter_avp"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "flag-mandatory",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "flag-protected",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "flag-vendor-specific",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "interim-message-include",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "parent-label",
                        value_type: ValueKind::String,
                        in_sections: &["messages", "avps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "reporting-message-include",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "subscriber-attr",
                        value_type: ValueKind::Reference,
                        in_sections: &["messages", "avps"],
                        allow_none: true,
                        references: &["pem_subscriber_attribute"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diameter-avp",
                value_type: ValueKind::Reference,
                in_sections: &["messages", "avps"],
                allow_none: true,
                references: &["pem_protocol_diameter_avp"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flag-mandatory",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flag-protected",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flag-vendor-specific",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interim-message-include",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent-label",
                value_type: ValueKind::String,
                in_sections: &["messages", "avps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reporting-message-include",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-attr",
                value_type: ValueKind::Reference,
                in_sections: &["messages", "avps"],
                allow_none: true,
                references: &["pem_subscriber_attribute"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                in_sections: &["messages"],
                enum_values: &["any", "in", "out"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message-type",
                value_type: ValueKind::Enum,
                in_sections: &["messages"],
                enum_values: &["cca-i", "cca-u", "ccr-i", "ccr-t", "ccr-u", "raa", "rar"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-id",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "avp",
                        value_type: ValueKind::Reference,
                        in_sections: &["subscriber-id"],
                        allow_none: true,
                        references: &["pem_protocol_diameter_avp"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["subscriber-id"],
                        enum_values: &["e164", "imsi", "nai", "private"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type-avp",
                        value_type: ValueKind::Reference,
                        in_sections: &["subscriber-id"],
                        allow_none: true,
                        references: &["pem_protocol_diameter_avp"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "avp",
                value_type: ValueKind::Reference,
                in_sections: &["subscriber-id"],
                allow_none: true,
                references: &["pem_protocol_diameter_avp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["subscriber-id"],
                enum_values: &["e164", "imsi", "nai", "private"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type-avp",
                value_type: ValueKind::Reference,
                in_sections: &["subscriber-id"],
                allow_none: true,
                references: &["pem_protocol_diameter_avp"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_protocol_profile_radius",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["protocol profile radius"],
        },
        header_types: &[("pem", "protocol profile radius")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cp",
                value_type: ValueKind::Reference,
                references: &[
                    "auth_source",
                    "ltm_persistence_source_addr",
                    "security_dos_auto_thresholds_top_source_ips",
                    "security_nat_destination_translation",
                    "security_nat_source_translation",
                    "sys_ipfix_destination",
                    "sys_log_config_destination_alertd",
                    "sys_log_config_destination_arcsight",
                    "sys_log_config_destination_ipfix",
                    "sys_log_config_destination_local_database",
                    "sys_log_config_destination_local_syslog",
                    "sys_log_config_destination_management_port",
                    "sys_log_config_destination_remote_high_speed_log",
                    "sys_log_config_destination_remote_syslog",
                    "sys_log_config_destination_splunk",
                    "sys_sflow_data_source_http",
                    "sys_sflow_data_source_interface",
                    "sys_sflow_data_source_system",
                    "sys_sflow_data_source_vlan",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "messages",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "avps",
                        value_type: ValueKind::List,
                        in_sections: &["messages"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages"],
                        enum_values: &["any", "in", "out"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "message-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages"],
                        enum_values: &[
                            "acct-req-interim-update",
                            "acct-req-start",
                            "acct-req-stop",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "avps",
                value_type: ValueKind::List,
                in_sections: &["messages"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "ingress-op",
                        value_type: ValueKind::Enum,
                        in_sections: &["messages", "avps"],
                        allow_none: true,
                        enum_values: &["none"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "radius-avp",
                        value_type: ValueKind::Reference,
                        in_sections: &["messages", "avps"],
                        allow_none: true,
                        references: &["pem_protocol_radius_avp"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "subscriber-attr",
                        value_type: ValueKind::Reference,
                        in_sections: &["messages", "avps"],
                        allow_none: true,
                        references: &["pem_subscriber_attribute"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ingress-op",
                value_type: ValueKind::Enum,
                in_sections: &["messages", "avps"],
                allow_none: true,
                enum_values: &["none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "radius-avp",
                value_type: ValueKind::Reference,
                in_sections: &["messages", "avps"],
                allow_none: true,
                references: &["pem_protocol_radius_avp"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-attr",
                value_type: ValueKind::Reference,
                in_sections: &["messages", "avps"],
                allow_none: true,
                references: &["pem_subscriber_attribute"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                in_sections: &["messages"],
                enum_values: &["any", "in", "out"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message-type",
                value_type: ValueKind::Enum,
                in_sections: &["messages"],
                enum_values: &["acct-req-interim-update", "acct-req-start", "acct-req-stop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-id",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "order",
                        value_type: ValueKind::Integer,
                        in_sections: &["subscriber-id"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "prefix",
                        value_type: ValueKind::String,
                        in_sections: &["subscriber-id"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "radius-avp",
                        value_type: ValueKind::Reference,
                        in_sections: &["subscriber-id"],
                        allow_none: true,
                        references: &["pem_protocol_radius_avp"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "suffix",
                        value_type: ValueKind::String,
                        in_sections: &["subscriber-id"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-id-type",
                value_type: ValueKind::Enum,
                enum_values: &["e164", "imsi", "nai", "private"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["subscriber-id"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix",
                value_type: ValueKind::String,
                in_sections: &["subscriber-id"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "radius-avp",
                value_type: ValueKind::Reference,
                in_sections: &["subscriber-id"],
                allow_none: true,
                references: &["pem_protocol_radius_avp"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suffix",
                value_type: ValueKind::String,
                in_sections: &["subscriber-id"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_protocol_radius_avp",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["protocol radius-avp"],
        },
        header_types: &[("pem", "protocol radius-avp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-type",
                value_type: ValueKind::Integer,
                enum_values: &[
                    "3gpp-rat-type",
                    "3gpp-user-location-info",
                    "ipaddr",
                    "ipv6addr",
                    "ipv6prefix",
                    "octet",
                    "time",
                ],
                default: Some("string"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-length",
                value_type: ValueKind::Integer,
                default: Some("253"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-length",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vendor-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vendor-type",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
