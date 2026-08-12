//! Generated BIG-IP object specs. DO NOT EDIT.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_datacenter",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["datacenter"],
        },
        header_types: &[("gtm", "datacenter")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "contact",
                value_type: ValueKind::Reference,
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
                name: "location",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-fallback",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "any-available",
                    "inside-datacenter",
                    "none",
                    "outside-datacenter",
                    "pool",
                ],
                default: Some("any-available"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["gtm_prober_pool"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-preference",
                value_type: ValueKind::Enum,
                enum_values: &["inside-datacenter", "outside-datacenter", "pool"],
                default: Some("inside-datacenter"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_distributed_app",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["distributed-app"],
        },
        header_types: &[("gtm", "distributed-app")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dependency-level",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["datacenter", "link", "none", "server", "wideip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disabled-contexts",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wideips",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_general",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings general"],
        },
        header_types: &[("gtm", "global-settings general")],
        properties: &[
            BigipPropertySpec {
                name: "allow-nxdomain-override",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discovery",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discovery-interval",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "automatic-configuration-save-timeout",
                value_type: ValueKind::Integer,
                default: Some("15 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-ldns-servers",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-name-check",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["allow-underscore", "none"],
                default: Some("allow-underscore"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drain-persistent-requests",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-status",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gtm-sets-recursion",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heartbeat-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-ltm-rate-limit-modes",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-cipher-list",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-crl-validation-depth",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["device", "full"],
                default: Some("full"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-minimum-tls-version",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-becoming-active",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-expiring",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-file-update",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-expired-crls",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-not-yet-active-crls",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-revoked-certs",
                value_type: ValueKind::Enum,
                enum_values: &["always", "existing", "never"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor-disabled-objects",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nethsm-timeout",
                value_type: ValueKind::Integer,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsec3-types-bitmap-strict",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-leader",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-wildcard-rrs",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("disable"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                default: Some("32"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-group-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-time-tolerance",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-timeout",
                value_type: ValueKind::Integer,
                default: Some("180"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronize-zone-files",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronize-zone-files-timeout",
                value_type: ValueKind::Integer,
                default: Some("300"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-allow-zero-scores",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtuals-depend-on-server-state",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wideip-zone-nameserver",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_load_balancing",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings load-balancing"],
        },
        header_types: &[("gtm", "global-settings load-balancing")],
        properties: &[
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-path-ttl",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "respect-fallback-dependency",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-longest-match",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-vs-availability",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_metrics",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings metrics"],
        },
        header_types: &[("gtm", "global-settings metrics")],
        properties: &[
            BigipPropertySpec {
                name: "default-probe-limit",
                value_type: ValueKind::Integer,
                default: Some("12"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-packet-length",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-sample-count",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-timeout",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-ttl",
                value_type: ValueKind::Integer,
                default: Some("604800"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactive-ldns-ttl",
                value_type: ValueKind::Integer,
                default: Some("2419200 (28 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactive-paths-ttl",
                value_type: ValueKind::Integer,
                default: Some("604800 (7 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ldns-update-interval",
                value_type: ValueKind::Integer,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-synchronous-monitor-requests",
                value_type: ValueKind::Integer,
                default: Some("20"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics-caching",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(604800f64),
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics-collection-protocols",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-ttl",
                value_type: ValueKind::Integer,
                default: Some("2400"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "paths-retry",
                value_type: ValueKind::Integer,
                default: Some("120"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_metrics_exclusions",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings metrics-exclusions"],
        },
        header_types: &[("gtm", "global-settings metrics-exclusions")],
        properties: &[BigipPropertySpec {
            name: "addresses",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_link",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["link"],
        },
        header_types: &[("gtm", "link")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cost-segments",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "datacenter",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-name",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "duplex-billing",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-inbound-bps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-inbound-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-outbound-bps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-outbound-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-total-bps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-total-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-ratio",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor",
                value_type: ValueKind::Unknown,
                repeated: true,
                references: &[
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
                ],
                shape_kind: Some(ValueKind::Object),
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prepaid-segment",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "router-addresses",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-provider",
                value_type: ValueKind::Reference,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translation",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uplink-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weighting",
                value_type: ValueKind::Enum,
                enum_values: &["price", "ratio"],
                default: Some("ratio"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_listener",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["listener"],
        },
        header_types: &[("gtm", "listener")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "advertise",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                name: "auto-lasthop",
                value_type: ValueKind::Enum,
                enum_values: &["default", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-persistence",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Enum,
                enum_values: &["tcp", "udp"],
                default: Some("udp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-hop-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::Unknown,
                required: true,
                shape_kind: Some(ValueKind::Object),
                default: Some("255"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("53 if no port number is specified"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profiles",
                value_type: ValueKind::List,
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
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "context",
                    value_type: ValueKind::Enum,
                    in_sections: &["profiles"],
                    enum_values: &["all", "clientside", "serverside"],
                    default: Some("all for both sides"),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "context",
                value_type: ValueKind::Enum,
                in_sections: &["profiles"],
                enum_values: &["all", "clientside", "serverside"],
                default: Some("all for both sides"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                repeated: true,
                references: &["gtm_rule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address-translation",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "pool",
                        value_type: ValueKind::Reference,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        references: &[
                            "gtm_pool_a",
                            "gtm_pool_aaaa",
                            "gtm_pool_cname",
                            "gtm_pool_https",
                            "gtm_pool_mx",
                            "gtm_pool_naptr",
                            "gtm_pool_srv",
                            "gtm_pool_svcb",
                        ],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        enum_values: &["automap", "none", "snat"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                in_sections: &["source-address-translation"],
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["source-address-translation"],
                allow_none: true,
                enum_values: &["automap", "none", "snat"],
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
                name: "translate-port",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_listener_doh_proxy",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["listener-doh-proxy"],
        },
        header_types: &[("gtm", "listener-doh-proxy")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "advertise",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                name: "auto-lasthop",
                value_type: ValueKind::Enum,
                enum_values: &["default", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-persistence",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-hop-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::Unknown,
                required: true,
                shape_kind: Some(ValueKind::Object),
                default: Some("255"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("443 if no port number is specified"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profiles",
                value_type: ValueKind::List,
                required: true,
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
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "context",
                    value_type: ValueKind::Enum,
                    in_sections: &["profiles"],
                    enum_values: &["all", "clientside", "serverside"],
                    default: Some("all for both sides"),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "context",
                value_type: ValueKind::Enum,
                in_sections: &["profiles"],
                enum_values: &["all", "clientside", "serverside"],
                default: Some("all for both sides"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                repeated: true,
                references: &["gtm_rule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address-translation",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "pool",
                        value_type: ValueKind::Reference,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        references: &[
                            "gtm_pool_a",
                            "gtm_pool_aaaa",
                            "gtm_pool_cname",
                            "gtm_pool_https",
                            "gtm_pool_mx",
                            "gtm_pool_naptr",
                            "gtm_pool_srv",
                            "gtm_pool_svcb",
                        ],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        enum_values: &["automap", "none", "snat"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                in_sections: &["source-address-translation"],
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["source-address-translation"],
                allow_none: true,
                enum_values: &["automap", "none", "snat"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Enum,
                enum_values: &["change", "preserve"],
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
                name: "translate-port",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_listener_doh_server",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["listener-doh-server"],
        },
        header_types: &[("gtm", "listener-doh-server")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "advertise",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                name: "auto-lasthop",
                value_type: ValueKind::Enum,
                enum_values: &["default", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-persistence",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-hop-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::Unknown,
                required: true,
                shape_kind: Some(ValueKind::Object),
                default: Some("255"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("443 if no port number is specified"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profiles",
                value_type: ValueKind::List,
                required: true,
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
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "context",
                    value_type: ValueKind::Enum,
                    in_sections: &["profiles"],
                    enum_values: &["all", "clientside", "serverside"],
                    default: Some("all for both sides"),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "context",
                value_type: ValueKind::Enum,
                in_sections: &["profiles"],
                enum_values: &["all", "clientside", "serverside"],
                default: Some("all for both sides"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                repeated: true,
                references: &["gtm_rule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address-translation",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "pool",
                        value_type: ValueKind::Reference,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        references: &[
                            "gtm_pool_a",
                            "gtm_pool_aaaa",
                            "gtm_pool_cname",
                            "gtm_pool_https",
                            "gtm_pool_mx",
                            "gtm_pool_naptr",
                            "gtm_pool_srv",
                            "gtm_pool_svcb",
                        ],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        enum_values: &["automap", "none", "snat"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                in_sections: &["source-address-translation"],
                allow_none: true,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["source-address-translation"],
                allow_none: true,
                enum_values: &["automap", "none", "snat"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Enum,
                enum_values: &["change", "preserve"],
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
                name: "translate-port",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans-enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_bigip",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor bigip"],
        },
        header_types: &[("gtm", "monitor bigip")],
        properties: &[
            BigipPropertySpec {
                name: "aggregate-dynamic-ratios",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "average-members",
                    "average-nodes",
                    "none",
                    "sum-members",
                    "sum-nodes",
                ],
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
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_bigip"],
                default: Some("bigip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "non-default",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("90 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_bigip_link",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor bigip-link"],
        },
        header_types: &[("gtm", "monitor bigip-link")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_bigip_link"],
                default: Some("bigip_link"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_external",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor external"],
        },
        header_types: &[("gtm", "monitor external")],
        properties: &[
            BigipPropertySpec {
                name: "args",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_external"],
                default: Some("external"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "run",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_firepass",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor firepass"],
        },
        header_types: &[("gtm", "monitor firepass")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cipherlist",
                value_type: ValueKind::Unknown,
                default: Some("HIGH:!ADH"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "concurrency-limit",
                value_type: ValueKind::Integer,
                default: Some("95"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_firepass"],
                default: Some("firepass_gtm"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-load-average",
                value_type: ValueKind::Unknown,
                default: Some("12"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("90 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                default: Some("gtmuser"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_ftp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor ftp"],
        },
        header_types: &[("gtm", "monitor ftp")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_ftp"],
                default: Some("ftp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filename",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["passive"],
                default: Some("passive"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_gateway_icmp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor gateway-icmp"],
        },
        header_types: &[("gtm", "monitor gateway-icmp")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_gateway_icmp"],
                default: Some("gateway_icmp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-attempts",
                value_type: ValueKind::Integer,
                default: Some("3 attempts"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("1 second"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
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
            kind: "gtm_monitor_gtp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor gtp"],
        },
        header_types: &[("gtm", "monitor gtp")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_gtp"],
                default: Some("gtp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-attempts",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol-version",
                value_type: ValueKind::Integer,
                default: Some("version 1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_http",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor http"],
        },
        header_types: &[("gtm", "monitor http")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_http"],
                default: Some("http"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-status-code",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "disabled, which specifies that the monitor does not operate in reverse mode",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_https",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor https"],
        },
        header_types: &[("gtm", "monitor https")],
        properties: &[
            BigipPropertySpec {
                name: "cert",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cipherlist",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compatibility",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_https"],
                default: Some("https"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-status-code",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "disabled, which specifies that the monitor does not operate in reverse mode",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                default: Some("GET /, which retrieves a default HTML file for a web site"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sni-server-name",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_imap",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor imap"],
        },
        header_types: &[("gtm", "monitor imap")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_imap"],
                default: Some("imap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "folder",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["sys_folder"],
                default: Some("INBOX"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor ldap"],
        },
        header_types: &[("gtm", "monitor ldap")],
        properties: &[
            BigipPropertySpec {
                name: "base",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chase-referrals",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_ldap"],
                default: Some("ldap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mandatory-attributes",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "ssl", "tls"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_mssql",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor mssql"],
        },
        header_types: &[("gtm", "monitor mssql")],
        properties: &[
            BigipPropertySpec {
                name: "count",
                value_type: ValueKind::Enum,
                enum_values: &["0", "1"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "database",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["sys_log_config_destination_local_database"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_mssql"],
                default: Some("mssql"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-column",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-row",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("91 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_mysql",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor mysql"],
        },
        header_types: &[("gtm", "monitor mysql")],
        properties: &[
            BigipPropertySpec {
                name: "count",
                value_type: ValueKind::Enum,
                enum_values: &["0", "1"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "database",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["sys_log_config_destination_local_database"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_mysql"],
                default: Some("mysql"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-column",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-row",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("91 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_nntp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor nntp"],
        },
        header_types: &[("gtm", "monitor nntp")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_nntp"],
                default: Some("nntp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "newsgroup",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_oracle",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor oracle"],
        },
        header_types: &[("gtm", "monitor oracle")],
        properties: &[
            BigipPropertySpec {
                name: "count",
                value_type: ValueKind::Enum,
                enum_values: &["0", "1"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "database",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["sys_log_config_destination_local_database"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_oracle"],
                default: Some("oracle"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-column",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-row",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("91 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_pop3",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor pop3"],
        },
        header_types: &[("gtm", "monitor pop3")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_pop3"],
                default: Some("pop3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_postgresql",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor postgresql"],
        },
        header_types: &[("gtm", "monitor postgresql")],
        properties: &[
            BigipPropertySpec {
                name: "count",
                value_type: ValueKind::Enum,
                enum_values: &["0", "1"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "database",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["sys_log_config_destination_local_database"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_postgresql"],
                default: Some("postgresql"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-column",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv-row",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("91 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_radius",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor radius"],
        },
        header_types: &[("gtm", "monitor radius")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_radius"],
                default: Some("radius"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nas-ip-address",
                value_type: ValueKind::String,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_radius_accounting",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor radius-accounting"],
        },
        header_types: &[("gtm", "monitor radius-accounting")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-until-up",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["gtm_monitor_radius_accounting"],
                default: Some("radius"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nas-ip-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-until-up",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_real_server",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor real-server"],
        },
        header_types: &[("gtm", "monitor real-server")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_real_server"],
                default: Some("real_server"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("ServerBandwidth:1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_scripted",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor scripted"],
        },
        header_types: &[("gtm", "monitor scripted")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_scripted"],
                default: Some("scripted"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filename",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_sip",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor sip"],
        },
        header_types: &[("gtm", "monitor sip")],
        properties: &[
            BigipPropertySpec {
                name: "cert",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cipherlist",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compatibility",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_sip"],
                default: Some("sip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["any", "none", "status"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter-neg",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["any", "none", "status"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["sips", "tcp", "tls", "udp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_smtp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor smtp"],
        },
        header_types: &[("gtm", "monitor smtp")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_smtp"],
                default: Some("smtp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "apm_policy_agent_route_domain_selection",
                    "cm_trust_domain",
                    "net_route_domain",
                    "security_firewall_user_domain",
                    "wam_domain_list",
                    "wam_resource_domain_list",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_snmp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor snmp"],
        },
        header_types: &[("gtm", "monitor snmp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_routing_community_list"],
                default: Some("public"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_snmp"],
                default: Some("snmp_gtm"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("90 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("161"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("v1"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_snmp_link",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor snmp-link"],
        },
        header_types: &[("gtm", "monitor snmp-link")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_routing_community_list"],
                default: Some("public"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_snmp_link"],
                default: Some("snmp_link"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("161"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_soap",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor soap"],
        },
        header_types: &[("gtm", "monitor soap")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_soap"],
                default: Some("soap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expect-fault",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "namespace",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parameter-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("bool"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parameter-type",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parameter-value",
                value_type: ValueKind::Integer,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "return-type",
                value_type: ValueKind::String,
                default: Some("bool"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "return-value",
                value_type: ValueKind::Integer,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "soap-action",
                value_type: ValueKind::String,
                default: Some("the empty string"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-path",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_tcp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor tcp"],
        },
        header_types: &[("gtm", "monitor tcp")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_tcp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "disabled, which specifies that the monitor does not operate in reverse mode",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
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
            kind: "gtm_monitor_tcp_half_open",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor tcp-half-open"],
        },
        header_types: &[("gtm", "monitor tcp-half-open")],
        properties: &[
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_tcp_half_open"],
                default: Some("tcp_half_open"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-attempts",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
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
            kind: "gtm_monitor_udp",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor udp"],
        },
        header_types: &[("gtm", "monitor udp")],
        properties: &[
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_udp"],
                default: Some("udp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-attempts",
                value_type: ValueKind::Integer,
                default: Some("3 attempts"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-interval",
                value_type: ValueKind::Integer,
                default: Some("1 second"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "disabled, which specifies that the monitor does not operate in reverse mode",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                default: Some("\"default send string\""),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
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
            kind: "gtm_monitor_wap",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor wap"],
        },
        header_types: &[("gtm", "monitor wap")],
        properties: &[
            BigipPropertySpec {
                name: "accounting-node",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "accounting-port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "call-id",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-until-up",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_wap"],
                default: Some("wap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::Endpoint),
                default: Some("*:*"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "framed-address",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recv",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server-id",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-id",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("31 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_monitor_wmi",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["monitor wmi"],
        },
        header_types: &[("gtm", "monitor wmi")],
        properties: &[
            BigipPropertySpec {
                name: "command",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["gtm_monitor_wmi"],
                default: Some("wmi"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-down-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("LoadPercentage, DiskUsage, PhysicalMemoryUsage:1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "probe-timeout",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("120 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                default: Some("/scripts/f5Isapi"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_pool_a",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool a"],
        },
        header_types: &[("gtm", "pool a")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "fallback-ip",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "depends-on",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-mode",
                value_type: ValueKind::Enum,
                enum_values: &["number", "off", "percent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-value",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor",
                value_type: ValueKind::Unknown,
                repeated: true,
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
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_aaaa",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool aaaa"],
        },
        header_types: &[("gtm", "pool aaaa")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "fallback-ip",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "depends-on",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor",
                value_type: ValueKind::Unknown,
                repeated: true,
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
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_cname",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool cname"],
        },
        header_types: &[("gtm", "pool cname")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-target",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_https",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool https"],
        },
        header_types: &[("gtm", "pool https")],
        properties: &[
            BigipPropertySpec {
                name: "alpn",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "fallback-ip",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hpke-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-mode",
                value_type: ValueKind::Enum,
                enum_values: &["number", "off", "percent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-value",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_mx",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool mx"],
        },
        header_types: &[("gtm", "pool mx")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_naptr",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool naptr"],
        },
        header_types: &[("gtm", "pool naptr")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flags",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preference",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_pool_srv",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool srv"],
        },
        header_types: &[("gtm", "pool srv")],
        properties: &[
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ratio",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_pool_svcb",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["pool svcb"],
        },
        header_types: &[("gtm", "pool svcb")],
        properties: &[
            BigipPropertySpec {
                name: "alpn",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alternate-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "drop-packet",
                    "fallback-ip",
                    "none",
                    "packet-rate",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fallback-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "none",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("return-to-dns"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hpke-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "completion-rate",
                    "cpu",
                    "drop-packet",
                    "fallback-ip",
                    "fewest-hops",
                    "kilobytes-per-second",
                    "least-connections",
                    "lowest-round-trip-time",
                    "packet-rate",
                    "quality-of-service",
                    "ratio",
                    "return-to-dns",
                    "round-robin",
                    "static-persistence",
                    "topology",
                    "virtual-server-capacity",
                    "virtual-server-score",
                ],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-resume",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-answers-returned",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "member-order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-mode",
                value_type: ValueKind::Enum,
                enum_values: &["number", "off", "percent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-members-up-value",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hit-ratio",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-hops",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-kilobytes-second",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-lcs",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-packet-rate",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-rtt",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-topology",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-capacity",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qos-vs-score",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(4294967295f64),
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-member-availability",
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
            kind: "gtm_prober_pool",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["prober-pool"],
        },
        header_types: &[("gtm", "prober-pool")],
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
                name: "load-balancing-mode",
                value_type: ValueKind::Enum,
                enum_values: &["round-robin"],
                default: Some("global-availability"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_region",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["region"],
        },
        header_types: &[("gtm", "region")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continent",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "Africa",
                    "Antarctica",
                    "Asia",
                    "Australia",
                    "Europe",
                    "unknown",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "country",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "datacenter",
                value_type: ValueKind::Reference,
                references: &["gtm_datacenter"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geoip-isp",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "isp",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "AOL",
                    "BeijingCNC",
                    "CNC",
                    "ChinaTelecom",
                    "Comcast",
                    "Earthlink",
                    "ShanghaiCNC",
                    "ShanghaiTelecom",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "not",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "continent",
                    "country",
                    "datacenter",
                    "isp",
                    "pool",
                    "subnet",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "region-members",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "region-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Reference,
                references: &[
                    "security_firewall_current_state",
                    "sys_mcp_state",
                    "sys_state_mirroring",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subnet",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_rule",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["rule"],
        },
        header_types: &[("gtm", "rule")],
        properties: &[
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_server",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["server"],
        },
        header_types: &[("gtm", "server")],
        properties: &[
            BigipPropertySpec {
                name: "addresses",
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
                name: "datacenter",
                value_type: ValueKind::Reference,
                required: true,
                references: &["gtm_datacenter"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "depends-on",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "devices",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "explicit-link-name",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expose-route-domains",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iq-allow-path",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iq-allow-service-check",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iq-allow-snmp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-cipher-list",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-minimum-tls-version",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-cpu-usage",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-cpu-usage-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-bps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-connections-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-max-pps-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-mem-avail",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit-mem-avail-status",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-discovery",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ltm-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor",
                value_type: ValueKind::Unknown,
                repeated: true,
                references: &[
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
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-fallback",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "any-available",
                    "inherit",
                    "inside-datacenter",
                    "none",
                    "outside-datacenter",
                    "pool",
                ],
                default: Some("inherit"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["gtm_prober_pool"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prober-preference",
                value_type: ValueKind::Enum,
                enum_values: &["inherit", "inside-datacenter", "outside-datacenter", "pool"],
                default: Some("inherit"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "product",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translation",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translation-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "translation-port",
                value_type: ValueKind::Reference,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-server-discovery",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-servers",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_topology",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["topology"],
        },
        header_types: &[("gtm", "topology")],
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
                name: "order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_a",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip a"],
        },
        header_types: &[("gtm", "wideip a")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_a"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_a"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_aaaa",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip aaaa"],
        },
        header_types: &[("gtm", "wideip aaaa")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_aaaa"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_aaaa"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_cname",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip cname"],
        },
        header_types: &[("gtm", "wideip cname")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_cname"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_cname"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_https",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip https"],
        },
        header_types: &[("gtm", "wideip https")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_https"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_https"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_mx",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip mx"],
        },
        header_types: &[("gtm", "wideip mx")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_mx"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_mx"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_naptr",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip naptr"],
        },
        header_types: &[("gtm", "wideip naptr")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_naptr"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_naptr"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_srv",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip srv"],
        },
        header_types: &[("gtm", "wideip srv")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_srv"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_srv"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_svcb",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip svcb"],
        },
        header_types: &[("gtm", "wideip svcb")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
                default: Some("none"),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_svcb"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_svcb"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
