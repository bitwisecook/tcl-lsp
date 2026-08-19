//! BIG-IP object specs, bucketed by the first letter of `kind`.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). There is
//! no live generator to regenerate these from; edit them by hand, in the
//! same shape the other kind-letter buckets already use.
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the bucket matching its first letter,
//! and every `references` target either names a real kind or is on the
//! documented known-gap list.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_admin_partitions",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["admin-partitions"],
        },
        header_types: &[("cli", "admin-partitions")],
        properties: &[BigipPropertySpec {
            name: "update-partition",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_alias_private",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["alias private"],
        },
        header_types: &[("cli", "alias private")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command",
                value_type: ValueKind::Unknown,
                repeated: true,
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
            kind: "cli_alias_shared",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["alias shared"],
        },
        header_types: &[("cli", "alias shared")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command",
                value_type: ValueKind::Unknown,
                repeated: true,
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
            kind: "cli_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["global-settings"],
        },
        header_types: &[("cli", "global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "audit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
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
                enum_values: &["disabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scf-backup-number",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service",
                value_type: ValueKind::Enum,
                enum_values: &["number"],
                references: &[
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "apm_aaa_f5_service_connector",
                    "apm_saml_artifact_resolution_service",
                    "apm_saml_attribute_consuming_service",
                    "net_service_policy",
                    "pem_service_chain_endpoint",
                    "security_bot_defense_micro_service",
                    "security_protocol_inspection_service",
                    "sys_application_service",
                    "sys_service",
                ],
                default: Some("name"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_preference",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["preference"],
        },
        header_types: &[("cli", "preference")],
        properties: &[
            BigipPropertySpec {
                name: "alias-path",
                value_type: ValueKind::Unknown,
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
                name: "confirm-edit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "display-threshold",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "editor",
                value_type: ValueKind::Enum,
                enum_values: &["nano", "vi"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fully-qualified-host",
                value_type: ValueKind::Unknown,
                default: Some("to not display this information"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-date-time",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-file-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-size",
                value_type: ValueKind::Integer,
                default: Some("500"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keymap",
                value_type: ValueKind::Enum,
                enum_values: &["default", "emacs", "vi"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "list-all-properties",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mcp-state",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("to not display this information"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pager",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prompt",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-aliases",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stat-units",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "default", "exa", "gig", "kil", "meg", "peta", "raw", "tera", "yotta", "zetta",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress-warnings",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "config-version", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "table-indent-width",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcl-syntax-highlighting",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warn",
                value_type: ValueKind::Enum,
                enum_values: &["bell", "disabled", "visual-bell"],
                default: Some("bell"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_script",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["script"],
        },
        header_types: &[("cli", "script")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_transaction",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["transaction"],
        },
        header_types: &[("cli", "transaction")],
        properties: &[BigipPropertySpec {
            name: "submit",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_version",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["version"],
        },
        header_types: &[("cli", "version")],
        properties: &[BigipPropertySpec {
            name: "active",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_device",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["device"],
        },
        header_types: &[("cm", "device")],
        properties: &[
            BigipPropertySpec {
                name: "comment",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "configsync-ip",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "contact",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ha-capacity",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mgmt-unicast-mode",
                value_type: ValueKind::Enum,
                enum_values: &["both", "ipv4", "ipv6"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror-ip",
                value_type: ValueKind::Enum,
                enum_values: &["any6"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror-secondary-ip",
                value_type: ValueKind::Enum,
                enum_values: &["any6"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multicast-interface",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multicast-ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multicast-port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unicast-address",
                value_type: ValueKind::List,
                allow_none: true,
                block: &[
                    BigipPropertySpec {
                        name: "effective-ip",
                        value_type: ValueKind::String,
                        in_sections: &["unicast-address"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "effective-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["unicast-address"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ip",
                        value_type: ValueKind::String,
                        in_sections: &["unicast-address"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["unicast-address"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "effective-ip",
                value_type: ValueKind::String,
                in_sections: &["unicast-address"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "effective-port",
                value_type: ValueKind::Unknown,
                in_sections: &["unicast-address"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                in_sections: &["unicast-address"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                in_sections: &["unicast-address"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_device_group",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["device-group"],
        },
        header_types: &[("cm", "device-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "asm-sync",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-sync",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clear-incremental-config-sync-cache",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "devices",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "full-load-on-sync",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "incremental-config-sync-size-max",
                value_type: ValueKind::Integer,
                default: Some("1024 KB"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network-failover",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-on-auto-sync",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["sync-failover", "sync-only"],
                default: Some("sync-only"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_ha_group",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["ha-group"],
        },
        header_types: &[("cm", "ha-group")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_key",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["key"],
        },
        header_types: &[("cm", "key")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_traffic_group",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["traffic-group"],
        },
        header_types: &[("cm", "traffic-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-failback-enabled",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-failback-time",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failover-method",
                value_type: ValueKind::Enum,
                enum_values: &["ha-order", "ha-score"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ha-group",
                value_type: ValueKind::String,
                references: &["cm_ha_group"],
                usage_flags: &["deprecated", "not_synced"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ha-load-factor",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ha-order",
                value_type: ValueKind::Unknown,
                repeated: true,
                references: &["cm_device"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor",
                value_type: ValueKind::Unknown,
                references: &[
                    "analytics_system_monitor_report",
                    "gtm_monitor_bigip",
                    "gtm_monitor_bigip_link",
                    "gtm_monitor_external",
                    "gtm_monitor_firepass",
                    "gtm_monitor_ftp",
                    "gtm_monitor_gateway_icmp",
                    "gtm_monitor_gtp",
                    "gtm_monitor_http",
                    "gtm_monitor_https",
                    "gtm_monitor_imap",
                    "gtm_monitor_ldap",
                    "gtm_monitor_mssql",
                    "gtm_monitor_mysql",
                    "gtm_monitor_nntp",
                    "gtm_monitor_none",
                    "gtm_monitor_oracle",
                    "gtm_monitor_pop3",
                    "gtm_monitor_postgresql",
                    "gtm_monitor_radius",
                    "gtm_monitor_radius_accounting",
                    "gtm_monitor_real_server",
                    "gtm_monitor_scripted",
                    "gtm_monitor_sip",
                    "gtm_monitor_smtp",
                    "gtm_monitor_snmp",
                    "gtm_monitor_snmp_link",
                    "gtm_monitor_soap",
                    "gtm_monitor_tcp",
                    "gtm_monitor_tcp_half_open",
                    "gtm_monitor_udp",
                    "gtm_monitor_wap",
                    "gtm_monitor_wmi",
                    "ltm_default_node_monitor",
                    "ltm_monitor_diameter",
                    "ltm_monitor_dns",
                    "ltm_monitor_external",
                    "ltm_monitor_firepass",
                    "ltm_monitor_ftp",
                    "ltm_monitor_gateway_icmp",
                    "ltm_monitor_http",
                    "ltm_monitor_http2",
                    "ltm_monitor_https",
                    "ltm_monitor_icmp",
                    "ltm_monitor_imap",
                    "ltm_monitor_inband",
                    "ltm_monitor_ldap",
                    "ltm_monitor_module_score",
                    "ltm_monitor_mqtt",
                    "ltm_monitor_mssql",
                    "ltm_monitor_mysql",
                    "ltm_monitor_nntp",
                    "ltm_monitor_none",
                    "ltm_monitor_oracle",
                    "ltm_monitor_pop3",
                    "ltm_monitor_postgresql",
                    "ltm_monitor_radius",
                    "ltm_monitor_radius_accounting",
                    "ltm_monitor_real_server",
                    "ltm_monitor_rpc",
                    "ltm_monitor_sasp",
                    "ltm_monitor_scripted",
                    "ltm_monitor_sip",
                    "ltm_monitor_smb",
                    "ltm_monitor_smtp",
                    "ltm_monitor_snmp_dca",
                    "ltm_monitor_snmp_dca_base",
                    "ltm_monitor_soap",
                    "ltm_monitor_tcp",
                    "ltm_monitor_tcp_echo",
                    "ltm_monitor_tcp_half_open",
                    "ltm_monitor_udp",
                    "ltm_monitor_virtual_location",
                    "ltm_monitor_wap",
                    "ltm_monitor_wmi",
                    "sys_file_external_monitor",
                    "util_test_monitor",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cm_trust_domain",
            table_name: None,
            resolver_name: None,
            module: Some("cm"),
            object_types: &["trust-domain"],
        },
        header_types: &[("cm", "trust-domain")],
        properties: &[
            BigipPropertySpec {
                name: "add-device",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "device-ip",
                        value_type: ValueKind::String,
                        in_sections: &["add-device"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "device-name",
                        value_type: ValueKind::String,
                        in_sections: &["add-device"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "device-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["add-device"],
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "password",
                        value_type: ValueKind::String,
                        in_sections: &["add-device"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sha1-fingerprint",
                        value_type: ValueKind::String,
                        in_sections: &["add-device"],
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "username",
                        value_type: ValueKind::String,
                        in_sections: &["add-device"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-ip",
                value_type: ValueKind::String,
                in_sections: &["add-device"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-name",
                value_type: ValueKind::String,
                in_sections: &["add-device"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-port",
                value_type: ValueKind::Unknown,
                in_sections: &["add-device"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                in_sections: &["add-device"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sha1-fingerprint",
                value_type: ValueKind::String,
                in_sections: &["add-device"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                in_sections: &["add-device"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-devices",
                value_type: ValueKind::List,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deprecated",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "devices",
                value_type: ValueKind::List,
                list_operators: &["delete"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "md5-fingerprint",
                value_type: ValueKind::String,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "non-ca-devices",
                value_type: ValueKind::List,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-device",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serial",
                value_type: ValueKind::String,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sha1-fingerprint",
                value_type: ValueKind::String,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
