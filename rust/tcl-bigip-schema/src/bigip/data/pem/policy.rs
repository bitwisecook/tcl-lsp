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

//! BIG-IP object specs for `pem policy`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "pem_policy",
        table_name: None,
        resolver_name: None,
        module: Some("pem"),
        object_types: &["policy"],
    },
    header_types: &[("pem", "policy")],
    properties: &[
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::List,
            references: &[
                "gtm_rule",
                "ltm_cipher_rule",
                "ltm_global_settings_rule",
                "ltm_rule",
                "ltm_rule_profiler",
                "security_firewall_matching_rule",
                "security_firewall_on_demand_rule_deploy",
                "security_firewall_rule_list",
                "security_firewall_rule_stat",
                "security_packet_filter_rule_stat",
                "sys_file_rewrite_rule",
            ],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "classification-filters",
                    value_type: ValueKind::List,
                    in_sections: &["rules"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "deprecated",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dscp-marking-downlink",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some(
                        "pass-through, indicating the DSCP code of the downlink packet will not be changed when the traffic flow matches the rule",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dscp-marking-uplink",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some(
                        "pass-through, indicating the DSCP code of the uplink packet will not be changed when the traffic flow matches the rule",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dtos-tethering",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "flow-info-filters",
                    value_type: ValueKind::List,
                    in_sections: &["rules"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "forwarding",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "gate-status",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "http-redirect",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "insert-content",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "intercept",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "l2-marking-downlink",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some(
                        "pass-through, indicating the L2 QoS Marking of the packet will not be changed when the packet matches the rule",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "l2-marking-uplink",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some(
                        "pass-through, indicating the L2 QoS Marking of the packet will not be changed when the packet matches the rule",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "modify-http-hdr",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "precedence",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "qoe-reporting",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    usage_flags: &["deprecated"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "qos-rate-pir-downlink",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "qos-rate-pir-uplink",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "quota",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ran-congestion",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "reporting",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "service-chain",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sfc-action",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcl-filter",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcp-analytics-enable",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcp-optimization-downlink",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcp-optimization-uplink",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "url-categorization-filters",
                    value_type: ValueKind::List,
                    in_sections: &["rules"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "classification-filters",
            value_type: ValueKind::List,
            in_sections: &["rules"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "classification-filters"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "application",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "classification-filters"],
                    references: &[
                        "analytics_application_security_anomalies_report",
                        "analytics_application_security_anomalies_scheduled_report",
                        "analytics_application_security_incidents_report",
                        "analytics_application_security_network_report",
                        "analytics_application_security_network_scheduled_report",
                        "analytics_application_security_report",
                        "analytics_application_security_scheduled_report",
                        "ltm_classification_application",
                        "ltm_classification_stats_application",
                        "sys_application_apl_script",
                        "sys_application_custom_stat",
                        "sys_application_service",
                        "sys_application_template",
                        "sys_disk_application_volume",
                        "wam_application",
                    ],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "category",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "classification-filters"],
                    references: &[
                        "ltm_classification_category",
                        "ltm_classification_stats_url_category",
                        "ltm_classification_url_category",
                        "security_blacklist_publisher_by_category",
                        "security_blacklist_publisher_category",
                        "security_bot_defense_anomaly_category",
                        "security_bot_defense_signature_category",
                        "security_dos_bot_signature_category",
                        "security_firewall_ipi_category_info",
                        "security_ip_intelligence_blacklist_category",
                        "security_scrubber_dwbl_scrubber_category_stats",
                        "sys_url_db_url_category",
                    ],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "classification-filters"],
                    enum_values: &["match", "nomatch"],
                    default: Some("match"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["rules", "classification-filters"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "application",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "classification-filters"],
            references: &[
                "analytics_application_security_anomalies_report",
                "analytics_application_security_anomalies_scheduled_report",
                "analytics_application_security_incidents_report",
                "analytics_application_security_network_report",
                "analytics_application_security_network_scheduled_report",
                "analytics_application_security_report",
                "analytics_application_security_scheduled_report",
                "ltm_classification_application",
                "ltm_classification_stats_application",
                "sys_application_apl_script",
                "sys_application_custom_stat",
                "sys_application_service",
                "sys_application_template",
                "sys_disk_application_volume",
                "wam_application",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "category",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "classification-filters"],
            references: &[
                "ltm_classification_category",
                "ltm_classification_stats_url_category",
                "ltm_classification_url_category",
                "security_blacklist_publisher_by_category",
                "security_blacklist_publisher_category",
                "security_bot_defense_anomaly_category",
                "security_bot_defense_signature_category",
                "security_dos_bot_signature_category",
                "security_firewall_ipi_category_info",
                "security_ip_intelligence_blacklist_category",
                "security_scrubber_dwbl_scrubber_category_stats",
                "sys_url_db_url_category",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "operation",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "classification-filters"],
            enum_values: &["match", "nomatch"],
            default: Some("match"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "deprecated",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dscp-marking-downlink",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some(
                "pass-through, indicating the DSCP code of the downlink packet will not be changed when the traffic flow matches the rule",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dscp-marking-uplink",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some(
                "pass-through, indicating the DSCP code of the uplink packet will not be changed when the traffic flow matches the rule",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dtos-tethering",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "dtos-detect",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "dtos-tethering"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "report",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "dtos-tethering"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tethering-detect",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "dtos-tethering"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dtos-detect",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "dtos-tethering"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "report",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "dtos-tethering"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "dest",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "dtos-tethering", "report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dest",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "dtos-tethering", "report"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "hsl",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "dtos-tethering", "report", "dest"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hsl",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "dtos-tethering", "report", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "format-script",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "dtos-tethering", "report", "dest", "hsl"],
                    allow_none: true,
                    references: &["pem_reporting_format_script"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "dtos-tethering", "report", "dest", "hsl"],
                    allow_none: true,
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
            name: "format-script",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "dtos-tethering", "report", "dest", "hsl"],
            allow_none: true,
            references: &["pem_reporting_format_script"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "dtos-tethering", "report", "dest", "hsl"],
            allow_none: true,
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
            name: "tethering-detect",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "dtos-tethering"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "flow-info-filters",
            value_type: ValueKind::List,
            in_sections: &["rules"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "flow-info-filters"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dscp-code",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "flow-info-filters"],
                    default: Some(
                        "disabled, indicating that the DSCP code will not be used to filter the packet in the flow-info-filter",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-ip-addr",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "flow-info-filters"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Network),
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "flow-info-filters"],
                    default: Some("any"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ecn-detection",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "flow-info-filters"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some(
                        "disabled, indicating that the ECN bits will not be used to filter the packet in the flow-info-filter",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "from-vlan",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "flow-info-filters"],
                    references: &[
                        "net_fdb_vlan",
                        "net_vlan",
                        "net_vlan_allowed",
                        "net_vlan_group",
                        "sys_sflow_data_source_vlan",
                        "sys_sflow_global_settings_vlan",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-addr-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "flow-info-filters"],
                    enum_values: &["IPv4", "IPv6", "any"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "l2-endpoint",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "flow-info-filters"],
                    enum_values: &["disabled", "vlan"],
                    default: Some(
                        "disabled, indicating that L2 endpoint is not used for matching the flows",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "flow-info-filters"],
                    enum_values: &["match", "nomatch"],
                    default: Some("match"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "proto",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "flow-info-filters"],
                    enum_values: &["any", "tcp", "udp"],
                    default: Some("any"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-ip-addr",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "flow-info-filters"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Network),
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "flow-info-filters"],
                    default: Some("any"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["rules", "flow-info-filters"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dscp-code",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "flow-info-filters"],
            default: Some(
                "disabled, indicating that the DSCP code will not be used to filter the packet in the flow-info-filter",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-ip-addr",
            value_type: ValueKind::String,
            in_sections: &["rules", "flow-info-filters"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-port",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "flow-info-filters"],
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ecn-detection",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "flow-info-filters"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some(
                "disabled, indicating that the ECN bits will not be used to filter the packet in the flow-info-filter",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "from-vlan",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "flow-info-filters"],
            references: &[
                "net_fdb_vlan",
                "net_vlan",
                "net_vlan_allowed",
                "net_vlan_group",
                "sys_sflow_data_source_vlan",
                "sys_sflow_global_settings_vlan",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-addr-type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "flow-info-filters"],
            enum_values: &["IPv4", "IPv6", "any"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l2-endpoint",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "flow-info-filters"],
            enum_values: &["disabled", "vlan"],
            default: Some(
                "disabled, indicating that L2 endpoint is not used for matching the flows",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "operation",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "flow-info-filters"],
            enum_values: &["match", "nomatch"],
            default: Some("match"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proto",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "flow-info-filters"],
            enum_values: &["any", "tcp", "udp"],
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-ip-addr",
            value_type: ValueKind::String,
            in_sections: &["rules", "flow-info-filters"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-port",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "flow-info-filters"],
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "forwarding",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "endpoint",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "forwarding"],
                    references: &["pem_forwarding_endpoint"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "fallback-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "forwarding"],
                    enum_values: &["continue", "default", "drop", "is"],
                    default: Some("drop"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "icap-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "forwarding"],
                    allow_none: true,
                    enum_values: &["both", "none", "request", "response"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "internal-virtual",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "forwarding"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "forwarding"],
                    allow_none: true,
                    enum_values: &["icap", "none", "pool", "route-to-network"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "endpoint",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "forwarding"],
            references: &["pem_forwarding_endpoint"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-action",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "forwarding"],
            enum_values: &["continue", "default", "drop", "is"],
            default: Some("drop"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icap-type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "forwarding"],
            allow_none: true,
            enum_values: &["both", "none", "request", "response"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal-virtual",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "forwarding"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "forwarding"],
            allow_none: true,
            enum_values: &["icap", "none", "pool", "route-to-network"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gate-status",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "http-redirect",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "fallback-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "http-redirect"],
                    enum_values: &["continue", "default", "drop", "is"],
                    default: Some("drop"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "redirect-url",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "http-redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-action",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "http-redirect"],
            enum_values: &["continue", "default", "drop", "is"],
            default: Some("drop"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "redirect-url",
            value_type: ValueKind::String,
            in_sections: &["rules", "http-redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "insert-content",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "duration",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "insert-content"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "frequency",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "insert-content"],
                    enum_values: &["always", "once", "once-every"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "position",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "insert-content"],
                    enum_values: &["append", "prepend"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value-content",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "insert-content"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "insert-content"],
                    enum_values: &["tcl-snippet"],
                    default: Some(
                        "string which indicates that the value-content field will be interpreted as a string",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "duration",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "insert-content"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "frequency",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "insert-content"],
            enum_values: &["always", "once", "once-every"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "position",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "insert-content"],
            enum_values: &["append", "prepend"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value-content",
            value_type: ValueKind::String,
            in_sections: &["rules", "insert-content"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value-type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "insert-content"],
            enum_values: &["tcl-snippet"],
            default: Some(
                "string which indicates that the value-content field will be interpreted as a string",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "intercept",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l2-marking-downlink",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some(
                "pass-through, indicating the L2 QoS Marking of the packet will not be changed when the packet matches the rule",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l2-marking-uplink",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some(
                "pass-through, indicating the L2 QoS Marking of the packet will not be changed when the packet matches the rule",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "modify-http-hdr",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "modify-http-hdr"],
                    allow_none: true,
                    enum_values: &["insert", "none", "remove"],
                    default: Some("match"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value-content",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "modify-http-hdr"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "modify-http-hdr"],
                    enum_values: &["tcl-snippet"],
                    default: Some(
                        "string which indicates that the value-content field will be interpreted as a string",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "operation",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "modify-http-hdr"],
            allow_none: true,
            enum_values: &["insert", "none", "remove"],
            default: Some("match"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value-content",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "modify-http-hdr"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value-type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "modify-http-hdr"],
            enum_values: &["tcl-snippet"],
            default: Some(
                "string which indicates that the value-content field will be interpreted as a string",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "precedence",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qoe-reporting",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            usage_flags: &["deprecated"],
            block: &[BigipPropertySpec {
                name: "dest",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "qoe-reporting"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dest",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "qoe-reporting"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "hsl",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "qoe-reporting", "dest"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hsl",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "qoe-reporting", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "format-script",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "qoe-reporting", "dest", "hsl"],
                    allow_none: true,
                    references: &["pem_reporting_format_script"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "qoe-reporting", "dest", "hsl"],
                    allow_none: true,
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
            name: "format-script",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "qoe-reporting", "dest", "hsl"],
            allow_none: true,
            references: &["pem_reporting_format_script"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "qoe-reporting", "dest", "hsl"],
            allow_none: true,
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
            name: "qos-rate-pir-downlink",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-rate-pir-uplink",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "quota",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "rating-group",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "quota"],
                    references: &["pem_quota_mgmt_rating_group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "reporting-level",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "quota"],
                    enum_values: &["rating-group", "service-id"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rating-group",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "quota"],
            references: &["pem_quota_mgmt_rating_group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reporting-level",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "quota"],
            enum_values: &["rating-group", "service-id"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ran-congestion",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "detect",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "ran-congestion"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "lowerthreshold-bw",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "ran-congestion"],
                    default: Some("1000kbps"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "report",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "ran-congestion"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "detect",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "ran-congestion"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lowerthreshold-bw",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "ran-congestion"],
            default: Some("1000kbps"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "report",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "ran-congestion"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "dest",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "ran-congestion", "report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dest",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "ran-congestion", "report"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "hsl",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "ran-congestion", "report", "dest"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hsl",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "ran-congestion", "report", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "format-script",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "ran-congestion", "report", "dest", "hsl"],
                    allow_none: true,
                    references: &["pem_reporting_format_script"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "ran-congestion", "report", "dest", "hsl"],
                    allow_none: true,
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
            name: "format-script",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "ran-congestion", "report", "dest", "hsl"],
            allow_none: true,
            references: &["pem_reporting_format_script"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "ran-congestion", "report", "dest", "hsl"],
            allow_none: true,
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
            name: "reporting",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "dest",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "granularity",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "reporting"],
                    enum_values: &["flow", "session", "transaction"],
                    default: Some(
                        "session which indicates the session report will be generated if this policy applies",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "reporting"],
                    default: Some("0 which indicates this feature is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "transaction",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "volume",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dest",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "gx",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hsl",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "radius-accounting",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sd",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gx",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "application-reporting",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "reporting", "dest", "gx"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "monitoring-key",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "reporting", "dest", "gx"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "application-reporting",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "reporting", "dest", "gx"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "monitoring-key",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "reporting", "dest", "gx"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hsl",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "flow-reporting-fields",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest", "hsl"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "format-script",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "reporting", "dest", "hsl"],
                    references: &["pem_reporting_format_script"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "reporting", "dest", "hsl"],
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
                    name: "session-reporting-fields",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest", "hsl"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "transaction-reporting-fields",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "dest", "hsl"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "flow-reporting-fields",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest", "hsl"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "format-script",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "reporting", "dest", "hsl"],
            references: &["pem_reporting_format_script"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "reporting", "dest", "hsl"],
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
            name: "session-reporting-fields",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest", "hsl"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transaction-reporting-fields",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest", "hsl"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "radius-accounting",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "radius-aaa-virtual",
                value_type: ValueKind::Reference,
                in_sections: &["rules", "reporting", "dest", "radius-accounting"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "radius-aaa-virtual",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "reporting", "dest", "radius-accounting"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sd",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "dest"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "application-reporting",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "reporting", "dest", "sd"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "monitoring-key",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "reporting", "dest", "sd"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "application-reporting",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "reporting", "dest", "sd"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "monitoring-key",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "reporting", "dest", "sd"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "granularity",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "reporting"],
            enum_values: &["flow", "session", "transaction"],
            default: Some(
                "session which indicates the session report will be generated if this policy applies",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "reporting"],
            default: Some("0 which indicates this feature is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transaction",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting"],
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "http",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "reporting", "transaction"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "http",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "transaction"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "hostname-len",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "reporting", "transaction", "http"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "uri-len",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "reporting", "transaction", "http"],
                    default: Some("256"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "user-agent-len",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "reporting", "transaction", "http"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hostname-len",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "reporting", "transaction", "http"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uri-len",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "reporting", "transaction", "http"],
            default: Some("256"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-agent-len",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "reporting", "transaction", "http"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "volume",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "downlink",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "volume"],
                    default: Some("0 which indicates this feature is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "total",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "volume"],
                    default: Some("0 which indicates this feature is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "uplink",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "reporting", "volume"],
                    default: Some("0 which indicates this feature is disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "downlink",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "volume"],
            default: Some("0 which indicates this feature is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "total",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "volume"],
            default: Some("0 which indicates this feature is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uplink",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "reporting", "volume"],
            default: Some("0 which indicates this feature is disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-chain",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sfc-action",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "metadata-template",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "sfc-action"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path-name",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "sfc-action"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata-template",
            value_type: ValueKind::String,
            in_sections: &["rules", "sfc-action"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path-name",
            value_type: ValueKind::String,
            in_sections: &["rules", "sfc-action"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcl-filter",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-analytics-enable",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-optimization-downlink",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-optimization-uplink",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url-categorization-filters",
            value_type: ValueKind::List,
            in_sections: &["rules"],
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "category",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules", "url-categorization-filters"],
                    references: &[
                        "ltm_classification_category",
                        "ltm_classification_stats_url_category",
                        "ltm_classification_url_category",
                        "security_blacklist_publisher_by_category",
                        "security_blacklist_publisher_category",
                        "security_bot_defense_anomaly_category",
                        "security_bot_defense_signature_category",
                        "security_dos_bot_signature_category",
                        "security_firewall_ipi_category_info",
                        "security_ip_intelligence_blacklist_category",
                        "security_scrubber_dwbl_scrubber_category_stats",
                        "sys_url_db_url_category",
                    ],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "url-categorization-filters"],
                    enum_values: &["match", "nomatch"],
                    default: Some("match"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "category",
            value_type: ValueKind::Reference,
            in_sections: &["rules", "url-categorization-filters"],
            references: &[
                "ltm_classification_category",
                "ltm_classification_stats_url_category",
                "ltm_classification_url_category",
                "security_blacklist_publisher_by_category",
                "security_blacklist_publisher_category",
                "security_bot_defense_anomaly_category",
                "security_bot_defense_signature_category",
                "security_dos_bot_signature_category",
                "security_firewall_ipi_category_info",
                "security_ip_intelligence_blacklist_category",
                "security_scrubber_dwbl_scrubber_category_stats",
                "sys_url_db_url_category",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "operation",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "url-categorization-filters"],
            enum_values: &["match", "nomatch"],
            default: Some("match"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transactional",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
