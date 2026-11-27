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

//! Properties `network` to `traffic-statistics` of `security log`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "network",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                in_sections: &["network"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["network"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
                in_sections: &["network"],
                references: &[
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Unknown,
                in_sections: &["network"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "filter",
        value_type: ValueKind::Unknown,
        in_sections: &["network"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "log-acl-match-accept",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-acl-match-drop",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-acl-match-reject",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-geo-always",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-ip-errors",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-tcp-errors",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-tcp-events",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-translation-fields",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-uuid-field",
                value_type: ValueKind::Enum,
                in_sections: &["network", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-acl-match-accept",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-acl-match-drop",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-acl-match-reject",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-geo-always",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-ip-errors",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-tcp-errors",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-tcp-events",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-translation-fields",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-uuid-field",
        value_type: ValueKind::Enum,
        in_sections: &["network", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["network"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["network", "format"],
                allow_none: true,
                enum_values: &[
                    "acl_policy_type",
                    "acl_rule_uuid",
                    "action",
                    "context_type",
                    "date_time",
                    "dest_ip",
                    "drop_reason",
                    "none",
                    "protocol",
                    "route_domain",
                    "sa_translation_pool",
                    "sa_translation_type",
                    "src_ip",
                    "translated_dest_ip",
                    "translated_ip_protocol",
                    "translated_route_domain",
                    "translated_src_ip",
                    "translated_vlan",
                ],
                references: &[
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["network", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["network", "format"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["network", "format"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["network", "format"],
        allow_none: true,
        enum_values: &[
            "acl_policy_type",
            "acl_rule_uuid",
            "action",
            "context_type",
            "date_time",
            "dest_ip",
            "drop_reason",
            "none",
            "protocol",
            "route_domain",
            "sa_translation_pool",
            "sa_translation_type",
            "src_ip",
            "translated_dest_ip",
            "translated_ip_protocol",
            "translated_route_domain",
            "translated_src_ip",
            "translated_vlan",
        ],
        references: &[
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["network", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["network", "format"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["network", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publisher",
        value_type: ValueKind::Reference,
        in_sections: &["network"],
        references: &[
            "security_blacklist_publisher_all_blacklist_publisher",
            "security_blacklist_publisher_blacklist_publisher_stats",
            "security_blacklist_publisher_by_addr",
            "security_blacklist_publisher_by_category",
            "security_blacklist_publisher_category",
            "security_blacklist_publisher_profile",
            "sys_icall_publisher",
            "sys_log_config_publisher",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Unknown,
        in_sections: &["network"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "acl-match-accept",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "acl-match-drop",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "acl-match-reject",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aggregate-rate",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-errors",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcp-errors",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcp-events",
                value_type: ValueKind::Integer,
                in_sections: &["network", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "acl-match-accept",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "acl-match-drop",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "acl-match-reject",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "aggregate-rate",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-errors",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tcp-errors",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tcp-events",
        value_type: ValueKind::Integer,
        in_sections: &["network", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "port-misuse",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "aggregate-rate",
                value_type: ValueKind::Integer,
                in_sections: &["port-misuse"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["port-misuse"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "aggregate-rate",
        value_type: ValueKind::Integer,
        in_sections: &["port-misuse"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["port-misuse"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-dns",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-dns"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-dns"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-dns"],
                references: &[
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-dns-dos-publisher",
        value_type: ValueKind::Reference,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "filter",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-dns"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "log-dns-drop",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-dns-filtered-drop",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-dns-malformed",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-dns-malicious",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-dns-reject",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-dns-drop",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-dns-filtered-drop",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-dns-malformed",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-dns-malicious",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-dns-reject",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-dns"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-dns", "format"],
                allow_none: true,
                enum_values: &[
                    "attack_type",
                    "date_time",
                    "dest_ip",
                    "dns_query_type",
                    "none",
                    "src_ip",
                    "vlan",
                ],
                references: &[
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["protocol-dns", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "format"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["protocol-dns", "format"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-dns", "format"],
        allow_none: true,
        enum_values: &[
            "attack_type",
            "date_time",
            "dest_ip",
            "dns_query_type",
            "none",
            "src_ip",
            "vlan",
        ],
        references: &[
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["protocol-dns", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "format"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["protocol-dns", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publisher",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-dns"],
        references: &[
            "security_blacklist_publisher_all_blacklist_publisher",
            "security_blacklist_publisher_blacklist_publisher_stats",
            "security_blacklist_publisher_by_addr",
            "security_blacklist_publisher_by_category",
            "security_blacklist_publisher_category",
            "security_blacklist_publisher_profile",
            "sys_icall_publisher",
            "sys_log_config_publisher",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-sip",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-sip"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-sip"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-sip"],
                references: &[
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-sip-dos-publisher",
        value_type: ValueKind::Reference,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "filter",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-sip"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "log-sip-drop",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sip-global-failures",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sip-malformed",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sip-redirection-responses",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sip-request-failures",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sip-server-errors",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-drop",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-global-failures",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-malformed",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-redirection-responses",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-request-failures",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sip-server-errors",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-sip"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["protocol-sip", "format"],
                allow_none: true,
                enum_values: &[
                    "attack_type",
                    "date_time",
                    "dest_ip",
                    "none",
                    "sip_callee",
                    "sip_caller",
                    "sip_method_type",
                    "src_ip",
                    "vlan",
                ],
                references: &[
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["protocol-sip", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "format"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["protocol-sip", "format"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["protocol-sip", "format"],
        allow_none: true,
        enum_values: &[
            "attack_type",
            "date_time",
            "dest_ip",
            "none",
            "sip_callee",
            "sip_caller",
            "sip_method_type",
            "src_ip",
            "vlan",
        ],
        references: &[
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["protocol-sip", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "format"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["protocol-sip", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publisher",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-sip"],
        references: &[
            "security_blacklist_publisher_all_blacklist_publisher",
            "security_blacklist_publisher_blacklist_publisher_stats",
            "security_blacklist_publisher_by_addr",
            "security_blacklist_publisher_by_category",
            "security_blacklist_publisher_category",
            "security_blacklist_publisher_profile",
            "sys_icall_publisher",
            "sys_log_config_publisher",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-transfer",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            in_sections: &["protocol-transfer"],
            references: &[
                "security_blacklist_publisher_all_blacklist_publisher",
                "security_blacklist_publisher_blacklist_publisher_stats",
                "security_blacklist_publisher_by_addr",
                "security_blacklist_publisher_by_category",
                "security_blacklist_publisher_category",
                "security_blacklist_publisher_profile",
                "sys_icall_publisher",
                "sys_log_config_publisher",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publisher",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-transfer"],
        references: &[
            "security_blacklist_publisher_all_blacklist_publisher",
            "security_blacklist_publisher_blacklist_publisher_stats",
            "security_blacklist_publisher_by_addr",
            "security_blacklist_publisher_by_category",
            "security_blacklist_publisher_category",
            "security_blacklist_publisher_profile",
            "sys_icall_publisher",
            "sys_log_config_publisher",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "traffic-statistics",
        value_type: ValueKind::List,
        block: &[
            BigipPropertySpec {
                name: "log-missed-flows",
                value_type: ValueKind::Enum,
                in_sections: &["traffic-statistics"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["traffic-statistics"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-reaped-flows",
                value_type: ValueKind::Enum,
                in_sections: &["traffic-statistics"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-sctive-flows",
                value_type: ValueKind::Enum,
                in_sections: &["traffic-statistics"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-syncookies",
                value_type: ValueKind::Enum,
                in_sections: &["traffic-statistics"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-syncookies-whitelist",
                value_type: ValueKind::Enum,
                in_sections: &["traffic-statistics"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-missed-flows",
        value_type: ValueKind::Enum,
        in_sections: &["traffic-statistics"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["traffic-statistics"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-reaped-flows",
        value_type: ValueKind::Enum,
        in_sections: &["traffic-statistics"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-sctive-flows",
        value_type: ValueKind::Enum,
        in_sections: &["traffic-statistics"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-syncookies",
        value_type: ValueKind::Enum,
        in_sections: &["traffic-statistics"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-syncookies-whitelist",
        value_type: ValueKind::Enum,
        in_sections: &["traffic-statistics"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
];
