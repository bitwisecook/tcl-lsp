//! Generated BIG-IP object specs. DO NOT EDIT.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_forwarding_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["forwarding-endpoint"],
        },
        header_types: &[("pem", "forwarding-endpoint")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "fallback",
                        value_type: ValueKind::Enum,
                        in_sections: &["persistence"],
                        enum_values: &["destination-ip", "disabled", "source-ip"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hash-settings",
                        value_type: ValueKind::List,
                        in_sections: &["persistence"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["persistence"],
                        enum_values: &["destination-ip", "disabled", "hash", "source-ip"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback",
                value_type: ValueKind::Enum,
                in_sections: &["persistence"],
                enum_values: &["destination-ip", "disabled", "source-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-settings",
                value_type: ValueKind::List,
                in_sections: &["persistence"],
                block: &[
                    BigipPropertySpec {
                        name: "algorithm",
                        value_type: ValueKind::Unknown,
                        in_sections: &["persistence", "hash-settings"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "length",
                        value_type: ValueKind::Integer,
                        in_sections: &["persistence", "hash-settings"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "offset",
                        value_type: ValueKind::Integer,
                        in_sections: &["persistence", "hash-settings"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "source",
                        value_type: ValueKind::Enum,
                        in_sections: &["persistence", "hash-settings"],
                        enum_values: &["tcl-snippet", "uri"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "tcl-value",
                        value_type: ValueKind::String,
                        in_sections: &["persistence", "hash-settings"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "algorithm",
                value_type: ValueKind::Unknown,
                in_sections: &["persistence", "hash-settings"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "length",
                value_type: ValueKind::Integer,
                in_sections: &["persistence", "hash-settings"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "offset",
                value_type: ValueKind::Integer,
                in_sections: &["persistence", "hash-settings"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::Enum,
                in_sections: &["persistence", "hash-settings"],
                enum_values: &["tcl-snippet", "uri"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcl-value",
                value_type: ValueKind::String,
                in_sections: &["persistence", "hash-settings"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["persistence"],
                enum_values: &["destination-ip", "disabled", "hash", "source-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snat-pool",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Enum,
                enum_values: &["change", "preserve", "preserve-strict"],
                default: Some("preserve"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translate-address",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translate-service",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_analytics",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings analytics"],
        },
        header_types: &[("pem", "global-settings analytics")],
        properties: &[
            BigipPropertySpec {
                name: "logging",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "hsl",
                    value_type: ValueKind::Unknown,
                    in_sections: &["logging"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hsl",
                value_type: ValueKind::Unknown,
                in_sections: &["logging"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "endpoint-id",
                    value_type: ValueKind::Unknown,
                    in_sections: &["logging", "hsl"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint-id",
                value_type: ValueKind::Unknown,
                in_sections: &["logging", "hsl"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-aware",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_gx",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings gx"],
        },
        header_types: &[("pem", "global-settings gx")],
        properties: &[
            BigipPropertySpec {
                name: "gx-immediate-reporting",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gx-usage-record-merge",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_hsl_flow",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings hsl-flow"],
        },
        header_types: &[("pem", "global-settings hsl-flow")],
        properties: &[
            BigipPropertySpec {
                name: "format-version",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-on-interim",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-on-start",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_hsl_report",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings hsl-report"],
        },
        header_types: &[("pem", "global-settings hsl-report")],
        properties: &[BigipPropertySpec {
            name: "format-version",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_insert_content",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings insert-content"],
        },
        header_types: &[("pem", "global-settings insert-content")],
        properties: &[BigipPropertySpec {
            name: "max-duration",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_policy",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings policy"],
        },
        header_types: &[("pem", "global-settings policy")],
        properties: &[
            BigipPropertySpec {
                name: "mandatory-action-list",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-flow",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-rule-match-mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_quota_mgmt",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings quota-mgmt"],
        },
        header_types: &[("pem", "global-settings quota-mgmt")],
        properties: &[
            BigipPropertySpec {
                name: "default-rating-group",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-context-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_session_mgmt_attributes",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings session-mgmt-attributes"],
        },
        header_types: &[("pem", "global-settings session-mgmt-attributes")],
        properties: &[
            BigipPropertySpec {
                name: "clear-on-nas-reboot",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "create-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "delete-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flow-term-on-sess-delete",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_subscriber_activity_log",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings subscriber-activity-log"],
        },
        header_types: &[("pem", "global-settings subscriber-activity-log")],
        properties: &[
            BigipPropertySpec {
                name: "dynamic-subscriber-ids",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
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
                name: "static-subscriber-ids",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-ip-addresses",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_interception_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["interception-endpoint"],
        },
        header_types: &[("pem", "interception-endpoint")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["destination-ip", "disabled", "source-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_irule",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["irule"],
        },
        header_types: &[("pem", "irule")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_listener",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["listener"],
        },
        header_types: &[("pem", "listener")],
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
                name: "profile-spm",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profile-subscriber-mgmt",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-servers",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
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
                        shape_kind: Some(ValueKind::IpAddress),
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
                shape_kind: Some(ValueKind::IpAddress),
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
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_profile_diameter_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["profile diameter-endpoint"],
        },
        header_types: &[("pem", "profile diameter-endpoint")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["pem_profile_diameter_endpoint"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-host",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-realm",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fatal-grace-time",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "enabled",
                        value_type: ValueKind::Enum,
                        in_sections: &["fatal-grace-time"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time",
                        value_type: ValueKind::Integer,
                        in_sections: &["fatal-grace-time"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["fatal-grace-time"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time",
                value_type: ValueKind::Integer,
                in_sections: &["fatal-grace-time"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gx-session-id-prefix",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gy-session-id-prefix",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msg-max-retransmits",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msg-retransmit-delay",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin-host",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin-realm",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pem-protocol-profile-gx",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_dns_profile_report",
                    "api_protection_profile_apiprotection",
                    "apm_profile_access",
                    "apm_profile_connectivity",
                    "apm_profile_exchange",
                    "apm_profile_oauth",
                    "apm_profile_remote_desktop",
                    "apm_profile_vdi",
                    "ltm_alg_log_profile",
                    "ltm_auth_profile",
                    "ltm_dns_hpke_profile",
                    "ltm_lsn_log_profile",
                    "ltm_message_routing_diameter_profile_router",
                    "ltm_message_routing_diameter_profile_session",
                    "ltm_message_routing_mqtt_profile_router",
                    "ltm_message_routing_mqtt_profile_session",
                    "ltm_message_routing_sip_profile_router",
                    "ltm_message_routing_sip_profile_session",
                    "ltm_profile_analytics",
                    "ltm_profile_certificate_authority",
                    "ltm_profile_classification",
                    "ltm_profile_client_ldap",
                    "ltm_profile_client_ssl",
                    "ltm_profile_connector",
                    "ltm_profile_dhcpv4",
                    "ltm_profile_dhcpv6",
                    "ltm_profile_diameter",
                    "ltm_profile_dns",
                    "ltm_profile_dns_logging",
                    "ltm_profile_doh_proxy",
                    "ltm_profile_doh_server",
                    "ltm_profile_fasthttp",
                    "ltm_profile_fastl4",
                    "ltm_profile_fix",
                    "ltm_profile_ftp",
                    "ltm_profile_georedundancy",
                    "ltm_profile_gtp",
                    "ltm_profile_html",
                    "ltm_profile_http",
                    "ltm_profile_http2",
                    "ltm_profile_http3",
                    "ltm_profile_http_compression",
                    "ltm_profile_httprouter",
                    "ltm_profile_icap",
                    "ltm_profile_iiop",
                    "ltm_profile_ilx",
                    "ltm_profile_imap",
                    "ltm_profile_ipother",
                    "ltm_profile_ipsecalg",
                    "ltm_profile_json",
                    "ltm_profile_mapt",
                    "ltm_profile_mblb",
                    "ltm_profile_mqtt",
                    "ltm_profile_mr_ratelimit",
                    "ltm_profile_mr_ratelimit_action",
                    "ltm_profile_mssql",
                    "ltm_profile_netflow",
                    "ltm_profile_ntlm",
                    "ltm_profile_ocsp",
                    "ltm_profile_ocsp_stapling_params",
                    "ltm_profile_one_connect",
                    "ltm_profile_pcp",
                    "ltm_profile_pop3",
                    "ltm_profile_pptp",
                    "ltm_profile_qoe",
                    "ltm_profile_quic",
                    "ltm_profile_radius",
                    "ltm_profile_ramcache",
                    "ltm_profile_request_adapt",
                    "ltm_profile_request_log",
                    "ltm_profile_response_adapt",
                    "ltm_profile_rewrite",
                    "ltm_profile_rtsp",
                    "ltm_profile_sctp",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "ltm_profile_sip",
                    "ltm_profile_smtp",
                    "ltm_profile_smtps",
                    "ltm_profile_socks",
                    "ltm_profile_splitsessionclient",
                    "ltm_profile_splitsessionserver",
                    "ltm_profile_sse",
                    "ltm_profile_statistics",
                    "ltm_profile_stream",
                    "ltm_profile_tcp",
                    "ltm_profile_tcp_analytics",
                    "ltm_profile_tdr",
                    "ltm_profile_tftp",
                    "ltm_profile_traffic_acceleration",
                    "ltm_profile_udp",
                    "ltm_profile_wa_cache",
                    "ltm_profile_web_acceleration",
                    "ltm_profile_web_security",
                    "ltm_profile_websocket",
                    "ltm_profile_xml",
                    "net_routing_profile_bgp",
                    "pem_profile_diameter_endpoint",
                    "pem_profile_radius_aaa",
                    "pem_profile_spm",
                    "pem_profile_subscriber_mgmt",
                    "pem_protocol_profile_gx",
                    "pem_protocol_profile_radius",
                    "saas_ap_ai_profile",
                    "saas_ati_profile",
                    "saas_bd_profile",
                    "saas_csd_profile",
                    "security_anti_fraud_profile",
                    "security_blacklist_publisher_profile",
                    "security_bot_defense_profile",
                    "security_datasync_global_profile",
                    "security_datasync_local_profile",
                    "security_dos_profile",
                    "security_flowspec_route_injector_profile",
                    "security_http_profile",
                    "security_log_profile",
                    "security_protocol_inspection_profile",
                    "security_protocol_inspection_profile_status",
                    "security_scrubber_profile",
                    "security_ssh_profile",
                    "sys_fpga_turboflex_profile",
                    "sys_turboflex_profile_all",
                    "sys_turboflex_profile_config",
                    "sys_turboflex_profile_feature",
                    "vcmp_traffic_profile",
                    "wom_profile_cifs",
                    "wom_profile_isession",
                    "wom_profile_mapi",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pem-protocol-profile-gy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_dns_profile_report",
                    "api_protection_profile_apiprotection",
                    "apm_profile_access",
                    "apm_profile_connectivity",
                    "apm_profile_exchange",
                    "apm_profile_oauth",
                    "apm_profile_remote_desktop",
                    "apm_profile_vdi",
                    "ltm_alg_log_profile",
                    "ltm_auth_profile",
                    "ltm_dns_hpke_profile",
                    "ltm_lsn_log_profile",
                    "ltm_message_routing_diameter_profile_router",
                    "ltm_message_routing_diameter_profile_session",
                    "ltm_message_routing_mqtt_profile_router",
                    "ltm_message_routing_mqtt_profile_session",
                    "ltm_message_routing_sip_profile_router",
                    "ltm_message_routing_sip_profile_session",
                    "ltm_profile_analytics",
                    "ltm_profile_certificate_authority",
                    "ltm_profile_classification",
                    "ltm_profile_client_ldap",
                    "ltm_profile_client_ssl",
                    "ltm_profile_connector",
                    "ltm_profile_dhcpv4",
                    "ltm_profile_dhcpv6",
                    "ltm_profile_diameter",
                    "ltm_profile_dns",
                    "ltm_profile_dns_logging",
                    "ltm_profile_doh_proxy",
                    "ltm_profile_doh_server",
                    "ltm_profile_fasthttp",
                    "ltm_profile_fastl4",
                    "ltm_profile_fix",
                    "ltm_profile_ftp",
                    "ltm_profile_georedundancy",
                    "ltm_profile_gtp",
                    "ltm_profile_html",
                    "ltm_profile_http",
                    "ltm_profile_http2",
                    "ltm_profile_http3",
                    "ltm_profile_http_compression",
                    "ltm_profile_httprouter",
                    "ltm_profile_icap",
                    "ltm_profile_iiop",
                    "ltm_profile_ilx",
                    "ltm_profile_imap",
                    "ltm_profile_ipother",
                    "ltm_profile_ipsecalg",
                    "ltm_profile_json",
                    "ltm_profile_mapt",
                    "ltm_profile_mblb",
                    "ltm_profile_mqtt",
                    "ltm_profile_mr_ratelimit",
                    "ltm_profile_mr_ratelimit_action",
                    "ltm_profile_mssql",
                    "ltm_profile_netflow",
                    "ltm_profile_ntlm",
                    "ltm_profile_ocsp",
                    "ltm_profile_ocsp_stapling_params",
                    "ltm_profile_one_connect",
                    "ltm_profile_pcp",
                    "ltm_profile_pop3",
                    "ltm_profile_pptp",
                    "ltm_profile_qoe",
                    "ltm_profile_quic",
                    "ltm_profile_radius",
                    "ltm_profile_ramcache",
                    "ltm_profile_request_adapt",
                    "ltm_profile_request_log",
                    "ltm_profile_response_adapt",
                    "ltm_profile_rewrite",
                    "ltm_profile_rtsp",
                    "ltm_profile_sctp",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "ltm_profile_sip",
                    "ltm_profile_smtp",
                    "ltm_profile_smtps",
                    "ltm_profile_socks",
                    "ltm_profile_splitsessionclient",
                    "ltm_profile_splitsessionserver",
                    "ltm_profile_sse",
                    "ltm_profile_statistics",
                    "ltm_profile_stream",
                    "ltm_profile_tcp",
                    "ltm_profile_tcp_analytics",
                    "ltm_profile_tdr",
                    "ltm_profile_tftp",
                    "ltm_profile_traffic_acceleration",
                    "ltm_profile_udp",
                    "ltm_profile_wa_cache",
                    "ltm_profile_web_acceleration",
                    "ltm_profile_web_security",
                    "ltm_profile_websocket",
                    "ltm_profile_xml",
                    "net_routing_profile_bgp",
                    "pem_profile_diameter_endpoint",
                    "pem_profile_radius_aaa",
                    "pem_profile_spm",
                    "pem_profile_subscriber_mgmt",
                    "pem_protocol_profile_gx",
                    "pem_protocol_profile_radius",
                    "saas_ap_ai_profile",
                    "saas_ati_profile",
                    "saas_bd_profile",
                    "saas_csd_profile",
                    "security_anti_fraud_profile",
                    "security_blacklist_publisher_profile",
                    "security_bot_defense_profile",
                    "security_datasync_global_profile",
                    "security_datasync_local_profile",
                    "security_dos_profile",
                    "security_flowspec_route_injector_profile",
                    "security_http_profile",
                    "security_log_profile",
                    "security_protocol_inspection_profile",
                    "security_protocol_inspection_profile_status",
                    "security_scrubber_profile",
                    "security_ssh_profile",
                    "sys_fpga_turboflex_profile",
                    "sys_turboflex_profile_all",
                    "sys_turboflex_profile_config",
                    "sys_turboflex_profile_feature",
                    "vcmp_traffic_profile",
                    "wom_profile_cifs",
                    "wom_profile_isession",
                    "wom_profile_mapi",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "product-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sd-session-id-prefix",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "supported-apps",
                value_type: ValueKind::Enum,
                enum_values: &["Gx", "Gy", "Sd"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_profile_radius_aaa",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["profile radius-aaa"],
        },
        header_types: &[("pem", "profile radius-aaa")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["pem_profile_radius_aaa"],
                default: Some("radiusaaa"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retransmission-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shared-secret",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transaction-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_profile_spm",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["profile spm"],
        },
        header_types: &[("pem", "profile spm")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["pem_profile_spm"],
                default: Some("spm"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fast-pem",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fast-vs-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "global-policies-high-precedence",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "global-policies-low-precedence",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unknown-subscriber-policies",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_profile_subscriber_mgmt",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["profile subscriber-mgmt"],
        },
        header_types: &[("pem", "profile subscriber-mgmt")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["pem_profile_subscriber_mgmt"],
                default: Some("subscriber-mgmt"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dhcp-lease-query",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "enabled",
                        value_type: ValueKind::Enum,
                        in_sections: &["dhcp-lease-query"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "vs-name",
                        value_type: ValueKind::Reference,
                        in_sections: &["dhcp-lease-query"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["dhcp-lease-query"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vs-name",
                value_type: ValueKind::Reference,
                in_sections: &["dhcp-lease-query"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-unknown-traffic-from-server-side",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sess-creation-from-server-side",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
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
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_quota_mgmt_rating_group",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["quota-mgmt rating-group"],
        },
        header_types: &[("pem", "quota-mgmt rating-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-breach-action",
                value_type: ValueKind::Enum,
                enum_values: &["allow", "redirect", "terminate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-forwarding-endpoint",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-quota",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["default-quota"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "volume",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-quota-holding-time",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                in_sections: &["default-quota"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "consumption-time",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota", "time"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "usage-time",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota", "time"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "consumption-time",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota", "time"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "usage-time",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota", "time"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "volume",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "input-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "output-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "total-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["default-quota", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "input-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "output-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "total-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["default-quota", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-threshold",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-validity-time",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-quota-request",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["initial-quota-request"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "volume",
                        value_type: ValueKind::Unknown,
                        in_sections: &["initial-quota-request"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                in_sections: &["initial-quota-request"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "volume",
                value_type: ValueKind::Unknown,
                in_sections: &["initial-quota-request"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "input-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["initial-quota-request", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "output-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["initial-quota-request", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "total-octets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["initial-quota-request", "volume"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "input-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["initial-quota-request", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "output-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["initial-quota-request", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "total-octets",
                value_type: ValueKind::Unknown,
                in_sections: &["initial-quota-request", "volume"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rating-group-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-on-install",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_reporting_format_script",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["reporting format-script"],
        },
        header_types: &[("pem", "reporting format-script")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "definition",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_service_chain_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["service-chain-endpoint"],
        },
        header_types: &[("pem", "service-chain-endpoint")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-endpoints",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["service-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "forwarding-endpoint",
                        value_type: ValueKind::Unknown,
                        in_sections: &["service-endpoints"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "from-vlan",
                        value_type: ValueKind::Reference,
                        in_sections: &["service-endpoints"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "http-adapt-service",
                        value_type: ValueKind::Unknown,
                        in_sections: &["service-endpoints"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "icap-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["service-endpoints"],
                        allow_none: true,
                        enum_values: &["both", "none", "request", "response"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "internal-virtual",
                        value_type: ValueKind::Enum,
                        in_sections: &["service-endpoints"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "order",
                        value_type: ValueKind::Integer,
                        in_sections: &["service-endpoints"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "service-option",
                        value_type: ValueKind::Enum,
                        in_sections: &["service-endpoints"],
                        enum_values: &["mandatory", "optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "steering-policy",
                        value_type: ValueKind::Reference,
                        in_sections: &["service-endpoints"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "to-endpoint",
                        value_type: ValueKind::Reference,
                        in_sections: &["service-endpoints"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["service-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forwarding-endpoint",
                value_type: ValueKind::Unknown,
                in_sections: &["service-endpoints"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-vlan",
                value_type: ValueKind::Reference,
                in_sections: &["service-endpoints"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-adapt-service",
                value_type: ValueKind::Unknown,
                in_sections: &["service-endpoints"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icap-type",
                value_type: ValueKind::Enum,
                in_sections: &["service-endpoints"],
                allow_none: true,
                enum_values: &["both", "none", "request", "response"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-virtual",
                value_type: ValueKind::Enum,
                in_sections: &["service-endpoints"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["service-endpoints"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-option",
                value_type: ValueKind::Enum,
                in_sections: &["service-endpoints"],
                enum_values: &["mandatory", "optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "steering-policy",
                value_type: ValueKind::Reference,
                in_sections: &["service-endpoints"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "to-endpoint",
                value_type: ValueKind::Reference,
                in_sections: &["service-endpoints"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_subscriber",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["subscriber"],
        },
        header_types: &[("pem", "subscriber")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address-list",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policies",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-id-type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "dhcp",
                    "dhcp-custom",
                    "e164",
                    "imsi",
                    "mac-dhcp",
                    "nai",
                    "private",
                ],
                default: Some("imsi"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_subscriber_attribute",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["subscriber-attribute"],
        },
        header_types: &[("pem", "subscriber-attribute")],
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
                name: "export",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "import",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "well-known-attr-id",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "called-station-id",
                    "calling-station-id",
                    "imeisv",
                    "imsi",
                    "ipaddr",
                    "not-defined",
                    "subs-id",
                    "user-location-info",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
