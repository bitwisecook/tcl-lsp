//! Generated BIG-IP object specs. DO NOT EDIT.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_address_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["address-list"],
        },
        header_types: &[("net", "address-list")],
        properties: &[
            BigipPropertySpec {
                name: "addresses",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
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
            kind: "net_arp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["arp"],
        },
        header_types: &[("net", "arp")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address",
                value_type: ValueKind::String,
                repeated: true,
                shape_kind: Some(ValueKind::IpAddress),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac-address",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc policy"],
        },
        header_types: &[("net", "bwc policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "categories",
                value_type: ValueKind::List,
                required: true,
                block: &[
                    BigipPropertySpec {
                        name: "ip-tos",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        enum_values: &["pass-through"],
                        default: Some("pass-through, which indicates, do not modify UDP packets"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "link-qos",
                        value_type: ValueKind::Enum,
                        in_sections: &["categories"],
                        enum_values: &["pass-through"],
                        default: Some("pass-through, which indicates, do not modify UDP packets"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-cat-rate",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-cat-rate-percentage",
                        value_type: ValueKind::Integer,
                        in_sections: &["categories"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "traffic-priority-map",
                        value_type: ValueKind::String,
                        in_sections: &["categories"],
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tos",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-qos",
                value_type: ValueKind::Enum,
                in_sections: &["categories"],
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-cat-rate",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-cat-rate-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["categories"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-priority-map",
                value_type: ValueKind::String,
                in_sections: &["categories"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Unknown,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tos",
                value_type: ValueKind::Integer,
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-qos",
                value_type: ValueKind::Enum,
                enum_values: &["pass-through"],
                default: Some("pass-through, which indicates, do not modify UDP packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-period",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-user-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-user-rate-pps",
                value_type: ValueKind::Integer,
                default: Some("0 (not configured)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measure",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-priority-map",
                value_type: ValueKind::String,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_priority_group",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc priority-group"],
        },
        header_types: &[("net", "bwc priority-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority-classes",
                value_type: ValueKind::List,
                block: &[
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["priority-classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight-percentage",
                        value_type: ValueKind::Integer,
                        in_sections: &["priority-classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_bwc_traffic_group",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["bwc traffic-group"],
        },
        header_types: &[("net", "bwc traffic-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
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
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority-classes",
                value_type: ValueKind::List,
                block: &[BigipPropertySpec {
                    name: "weight-percentage",
                    value_type: ValueKind::Integer,
                    in_sections: &["priority-classes"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["priority-classes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_cmetrics",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["cmetrics"],
        },
        header_types: &[("net", "cmetrics")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_cos_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["cos global-settings"],
        },
        header_types: &[("net", "cos global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "default-map-8021p",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-map-dscp",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-traffic-priority",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "feature-disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "feature-enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "precedence",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_cos_map_8021p",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["cos map-8021p"],
        },
        header_types: &[("net", "cos map-8021p")],
        properties: &[
            BigipPropertySpec {
                name: "traffic-priority",
                value_type: ValueKind::Reference,
                references: &["net_cos_traffic_priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_cos_map_dscp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["cos map-dscp"],
        },
        header_types: &[("net", "cos map-dscp")],
        properties: &[
            BigipPropertySpec {
                name: "traffic-priority",
                value_type: ValueKind::Reference,
                references: &["net_cos_traffic_priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_cos_traffic_priority",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["cos traffic-priority"],
        },
        header_types: &[("net", "cos traffic-priority")],
        properties: &[
            BigipPropertySpec {
                name: "buffer",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_dag_globals",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["dag-globals"],
        },
        header_types: &[("net", "dag-globals")],
        properties: &[
            BigipPropertySpec {
                name: "dag-ipv6-prefix-len",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icmp-hash",
                value_type: ValueKind::Enum,
                enum_values: &["icmp", "ipicmp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icmp-monitor-priority",
                value_type: ValueKind::Enum,
                enum_values: &["high", "normal"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "round-robin-mode",
                value_type: ValueKind::Enum,
                enum_values: &["local"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_dns_resolver",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["dns-resolver"],
        },
        header_types: &[("net", "dns-resolver")],
        properties: &[
            BigipPropertySpec {
                name: "answer-default-zones",
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
                name: "cache-size",
                value_type: ValueKind::Integer,
                default: Some("5767168"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "nameservers",
                    value_type: ValueKind::List,
                    in_sections: &["forward-zones"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameservers",
                value_type: ValueKind::List,
                in_sections: &["forward-zones"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-min-rtt",
                value_type: ValueKind::Integer,
                default: Some("50 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-ttl",
                value_type: ValueKind::Integer,
                default: Some("900 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "outbound-msg-retry",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefetch",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "randomize-query-name-case",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                default: Some("the default route domain"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv4",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv6",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-tcp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-udp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_fdb_tunnel",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["fdb tunnel"],
        },
        header_types: &[("net", "fdb tunnel")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "records",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoint",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoints",
                        value_type: ValueKind::List,
                        in_sections: &["records"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "replicators",
                        value_type: ValueKind::List,
                        in_sections: &["records"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["records"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["records"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::String,
                in_sections: &["records"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoints",
                value_type: ValueKind::List,
                in_sections: &["records"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "replicators",
                value_type: ValueKind::List,
                in_sections: &["records"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_fdb_vlan",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["fdb vlan"],
        },
        header_types: &[("net", "fdb vlan")],
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
                name: "interface",
                value_type: ValueKind::Reference,
                references: &[
                    "net_interface",
                    "net_interface_cos",
                    "net_interface_ddm",
                    "sys_sflow_data_source_interface",
                    "sys_sflow_global_settings_interface",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "records",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trunk",
                value_type: ValueKind::Reference,
                references: &["net_trunk"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_interface",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["interface"],
        },
        header_types: &[("net", "interface")],
        properties: &[
            BigipPropertySpec {
                name: "bundle",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bundle-speed",
                value_type: ValueKind::Enum,
                enum_values: &["100G", "40G"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flow-control",
                value_type: ValueKind::Unknown,
                default: Some("tx-rx"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "force-gigabit-fiber",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-error-correction",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lacp-port-priority",
                value_type: ValueKind::Integer,
                default: Some("33768"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-traps-enabled",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lldp-admin",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "rxonly", "txonly", "txrx"],
                default: Some("txonly"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lldp-tlvmap",
                value_type: ValueKind::Integer,
                default: Some("130943"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "media",
                value_type: ValueKind::Enum,
                enum_values: &["auto", "no-phy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "media-fixed",
                value_type: ValueKind::Enum,
                enum_values: &["auto", "no-phy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "media-sfp",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["auto", "no-phy", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-mgmt",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-fwd-mode",
                value_type: ValueKind::Enum,
                enum_values: &["l3", "passive", "virtual-wire"],
                default: Some("l3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefer-port",
                value_type: ValueKind::Enum,
                enum_values: &["fixed", "sfp"],
                default: Some("sfp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qinq-ethertype",
                value_type: ValueKind::String,
                default: Some("set to 0x8100"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sflow",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "poll-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["sflow"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "poll-interval-global",
                        value_type: ValueKind::Enum,
                        in_sections: &["sflow"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("yes"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                in_sections: &["sflow"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval-global",
                value_type: ValueKind::Enum,
                in_sections: &["sflow"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "span-mode",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp-auto-edge-port",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp-edge-port",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp-link-type",
                value_type: ValueKind::Enum,
                enum_values: &["auto", "p2p", "shared"],
                default: Some("auto"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp-reset",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ike_daemon",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ike-daemon"],
        },
        header_types: &[("net", "ipsec ike-daemon")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "isakmp-natt-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "isakmp-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "debug2", "error", "info", "notify", "warning"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "natt-keep-alive",
                value_type: ValueKind::Unknown,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ike_peer",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ike-peer"],
        },
        header_types: &[("net", "ipsec ike-peer")],
        properties: &[
            BigipPropertySpec {
                name: "address-list",
                value_type: ValueKind::String,
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
                name: "ca-cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "crl-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug-payloads",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dpd-delay",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-policy",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on", "unique"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-macro",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-dhcp",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-dns",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-dhcp",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-dns",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["aggressive", "main"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-key-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-key-passphrase",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-id-type",
                value_type: ValueKind::Enum,
                enum_values: &["asn1dn", "fqdn", "keyid-tag", "user-fqdn"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-id-value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nat-traversal",
                value_type: ValueKind::Enum,
                enum_values: &["force", "off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-cert-validator",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-ha-reauth",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-jitter-percent",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-reauth-fail-open",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passive",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-dynamic-ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-cert-file",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-cert-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["certfile", "none"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-id-type",
                value_type: ValueKind::Enum,
                enum_values: &["asn1dn", "fqdn", "keyid-tag", "user-fqdn"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-id-value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-auth-method",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "dss",
                    "ecdsa-256",
                    "ecdsa-384",
                    "ecdsa-521",
                    "pre-shared-key",
                    "rsa-signature",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "3des",
                    "aes",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "blowfish",
                    "camellia",
                    "cast128",
                    "des",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-hash-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "md5",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-perfect-forward-secrecy",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "ecp256", "ecp384", "ecp521", "modp1024", "modp1536", "modp2048", "modp3072",
                    "modp4096", "modp6144", "modp768", "modp8192",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preshared-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preshared-key-encrypted",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prf",
                value_type: ValueKind::Enum,
                enum_values: &["sha1", "sha256", "sha384", "sha512"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-support",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "replay-window-size",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-selector",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_traffic_selector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-cert",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("v1"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ipsec_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ipsec-policy"],
        },
        header_types: &[("net", "ipsec ipsec-policy")],
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
                name: "ike-phase2-auth-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ],
                default: Some("aes-gcm128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "3des",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "aes128",
                    "aes192",
                    "aes256",
                    "null",
                ],
                default: Some("aes-gcm128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-lifetime",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-lifetime-kilobytes",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-perfect-forward-secrecy",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "modp1024", "modp1536", "modp2048", "modp3072", "modp4096", "modp6144",
                    "modp768", "modp8192",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipcomp",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["deflate", "none", "null"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["interface", "tunnel"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-local-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_manual_security_association",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec manual-security-association"],
        },
        header_types: &[("net", "ipsec manual-security-association")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-algorithm",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["3des", "aes128", "aes192", "aes256", "null"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipsec-policy",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_ipsec_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "spi",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_traffic_selector",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec traffic-selector"],
        },
        header_types: &[("net", "ipsec traffic-selector")],
        properties: &[
            BigipPropertySpec {
                name: "action",
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                enum_values: &["both", "in", "out"],
                default: Some("both"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipsec-policy",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_ipsec_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipv6_subscriber_prefix_length",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipv6-subscriber-prefix-length"],
        },
        header_types: &[("net", "ipv6-subscriber-prefix-length")],
        properties: &[BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_lacp_globals",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["lacp-globals"],
        },
        header_types: &[("net", "lacp-globals")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_lldp_globals",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["lldp-globals"],
        },
        header_types: &[("net", "lldp-globals")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_multicast_globals",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["multicast-globals"],
        },
        header_types: &[("net", "multicast-globals")],
        properties: &[
            BigipPropertySpec {
                name: "max-pending-packets",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-pending-routes",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-lookup-timeout",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ndp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ndp"],
        },
        header_types: &[("net", "ndp")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac-address",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_packet_filter",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["packet-filter"],
        },
        header_types: &[("net", "packet-filter")],
        properties: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                enum_values: &["accept", "continue", "discard", "reject"],
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
                name: "logging",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-class",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rule",
                value_type: ValueKind::Unknown,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                references: &["net_vlan", "net_vlan_allowed", "net_vlan_group"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_packet_filter_trusted",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["packet-filter-trusted"],
        },
        header_types: &[("net", "packet-filter-trusted")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-addresses",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac-addresses",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_port_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["port-list"],
        },
        header_types: &[("net", "port-list")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ports",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_port_mirror",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["port-mirror"],
        },
        header_types: &[("net", "port-mirror")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interfaces",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_class",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping class"],
        },
        header_types: &[("net", "rate-shaping class")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling",
                value_type: ValueKind::Integer,
                default: Some("the value of the rate option"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling-percentage",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which indicates that the class uses the value of the ceiling option",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                enum_values: &["any", "to-client", "to-server"],
                default: Some("any"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-policy",
                value_type: ValueKind::Reference,
                references: &["net_rate_shaping_drop_policy"],
                default: Some("tail, which is the simplest drop policy"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-burst",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "queue",
                value_type: ValueKind::Enum,
                enum_values: &["pfifp", "sfq"],
                references: &[
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ],
                default: Some("sfq"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-percentage",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which specifies that the system uses the value of the rate option",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shaping-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_rate_shaping_shaping_policy"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_color_policer",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping color-policer"],
        },
        header_types: &[("net", "rate-shaping color-policer")],
        properties: &[
            BigipPropertySpec {
                name: "action",
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
                name: "committed-burst-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "committed-information-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "excess-burst-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_drop_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping drop-policy"],
        },
        header_types: &[("net", "rate-shaping drop-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "average-packet-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-max-active",
                value_type: ValueKind::Integer,
                default: Some("0 (zero),which disables active flow limitation"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-max-drop",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-min-drop",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inverse-weight",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-probability",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "red-hard-limit",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["fred", "red", "tail"],
                default: Some("tail"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_queue",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping queue"],
        },
        header_types: &[("net", "rate-shaping queue")],
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
                name: "pfifo-max-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pfifo-min-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-bucket-count",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-bucket-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-perturbation",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["pfifo", "sfq"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_shaping_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping shaping-policy"],
        },
        header_types: &[("net", "rate-shaping shaping-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling-percentage",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_rate_shaping_drop_policy"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-burst",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "queue",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-percentage",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_route",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["route"],
        },
        header_types: &[("net", "route")],
        properties: &[
            BigipPropertySpec {
                name: "blackhole",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gw",
                value_type: ValueKind::String,
                references: &["net_self"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interface",
                value_type: ValueKind::Reference,
                references: &["net_interface", "net_interface_cos", "net_interface_ddm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
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
            kind: "net_route_domain",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["route-domain"],
        },
        header_types: &[("net", "route-domain")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bwc-policy",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connection-limit",
                value_type: ValueKind::Integer,
                default: Some("0, unlimited"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flow-eviction-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-context-stat",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-enforced-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-enforced-policy-rules",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-staged-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-staged-policy-rules",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-intelligence-categories",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("None"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-misuse",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "routing-protocol",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-nat-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-nat-rules",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_router_advertisement",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["router-advertisement"],
        },
        header_types: &[("net", "router-advertisement")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "autonomous",
                value_type: ValueKind::Unknown,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "current-hop-limit",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-interval",
                value_type: ValueKind::Integer,
                default: Some("600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-interval",
                value_type: ValueKind::Integer,
                default: Some("200"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-other-config",
                value_type: ValueKind::Unknown,
                default: Some("0 zero"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "on-link",
                value_type: ValueKind::Unknown,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preferred-lifetime",
                value_type: ValueKind::Integer,
                default: Some("604800"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-length",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefixes",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reachable-time",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retransmit-timer",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "router-lifetime",
                value_type: ValueKind::Integer,
                default: Some("1800"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unmanaged",
                value_type: ValueKind::Unknown,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "valid-lifetime",
                value_type: ValueKind::Integer,
                default: Some("2592000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                references: &["net_vlan", "net_vlan_allowed", "net_vlan_group"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_access_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing access-list"],
        },
        header_types: &[("net", "routing access-list")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "destination",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "exact-match",
                        value_type: ValueKind::Enum,
                        in_sections: &["entries"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "source",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exact-match",
                value_type: ValueKind::Enum,
                in_sections: &["entries"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_bfd",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing bfd"],
        },
        header_types: &[("net", "routing bfd")],
        properties: &[
            BigipPropertySpec {
                name: "gtsm",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gtsm-ttl",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multihop-peer",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "interval",
                        value_type: ValueKind::Boolean,
                        in_sections: &["multihop-peer"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minrx",
                        value_type: ValueKind::Boolean,
                        in_sections: &["multihop-peer"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "multiplier",
                        value_type: ValueKind::Boolean,
                        in_sections: &["multihop-peer"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Boolean,
                in_sections: &["multihop-peer"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minrx",
                value_type: ValueKind::Boolean,
                in_sections: &["multihop-peer"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier",
                value_type: ValueKind::Boolean,
                in_sections: &["multihop-peer"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "notification",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "slow-timer",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                references: &["net_vlan"],
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "enabled",
                        value_type: ValueKind::Enum,
                        in_sections: &["vlan"],
                        enum_values: &["false", "true"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "interval",
                        value_type: ValueKind::Boolean,
                        in_sections: &["vlan"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minrx",
                        value_type: ValueKind::Boolean,
                        in_sections: &["vlan"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "multiplier",
                        value_type: ValueKind::Boolean,
                        in_sections: &["vlan"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["vlan"],
                enum_values: &["false", "true"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Boolean,
                in_sections: &["vlan"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minrx",
                value_type: ValueKind::Boolean,
                in_sections: &["vlan"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier",
                value_type: ValueKind::Boolean,
                in_sections: &["vlan"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_bgp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing bgp"],
        },
        header_types: &[("net", "routing bgp")],
        properties: &[
            BigipPropertySpec {
                name: "address-family",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "activate",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aggregate-address",
                        value_type: ValueKind::Reference,
                        in_sections: &["address-family"],
                        shape_kind: Some(ValueKind::List),
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-as-in",
                        value_type: ValueKind::Boolean,
                        in_sections: &["address-family"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "as-override",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute-unchanged",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "auto-summary",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "capability",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "default-originate",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "distance",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "distribute-list",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "filter-list",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "maximum-prefix",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "network-synchronization",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "next-hop-self",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "prefix-list",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remove-private-as",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "route-map",
                        value_type: ValueKind::String,
                        in_sections: &["address-family"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "route-reflector-client",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "route-server-client",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "send-community",
                        value_type: ValueKind::Boolean,
                        in_sections: &["address-family"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "soft-reconfiguration-inbound",
                        value_type: ValueKind::Enum,
                        in_sections: &["address-family"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "unsuppress-map",
                        value_type: ValueKind::Boolean,
                        in_sections: &["address-family"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight",
                        value_type: ValueKind::Boolean,
                        in_sections: &["address-family"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "activate",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aggregate-address",
                value_type: ValueKind::Reference,
                in_sections: &["address-family"],
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-as-in",
                value_type: ValueKind::Boolean,
                in_sections: &["address-family"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-override",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-unchanged",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-summary",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capability",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-originate",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distance",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distribute-list",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter-list",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-prefix",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network-synchronization",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "next-hop-self",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-list",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-private-as",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::String,
                in_sections: &["address-family"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-reflector-client",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-server-client",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-community",
                value_type: ValueKind::Boolean,
                in_sections: &["address-family"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "soft-reconfiguration-inbound",
                value_type: ValueKind::Enum,
                in_sections: &["address-family"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unsuppress-map",
                value_type: ValueKind::Boolean,
                in_sections: &["address-family"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Boolean,
                in_sections: &["address-family"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-set",
                value_type: ValueKind::Enum,
                in_sections: &["aggregate-address"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "summary-only",
                value_type: ValueKind::Enum,
                in_sections: &["aggregate-address"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-infinite-hold-time",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "always-compare-med",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-path",
                value_type: ValueKind::Enum,
                in_sections: &["attribute-unchanged"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "med",
                value_type: ValueKind::Enum,
                in_sections: &["attribute-unchanged"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "next-hop",
                value_type: ValueKind::Enum,
                in_sections: &["attribute-unchanged"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bestpath",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "as-path-ignore",
                        value_type: ValueKind::Enum,
                        in_sections: &["bestpath"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "compare-confed-aspath",
                        value_type: ValueKind::Enum,
                        in_sections: &["bestpath"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "compare-originator-id",
                        value_type: ValueKind::Enum,
                        in_sections: &["bestpath"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "compare-routerid",
                        value_type: ValueKind::Enum,
                        in_sections: &["bestpath"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "med",
                        value_type: ValueKind::String,
                        in_sections: &["bestpath"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "tie-break-on-age",
                        value_type: ValueKind::Enum,
                        in_sections: &["bestpath"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-path-ignore",
                value_type: ValueKind::Enum,
                in_sections: &["bestpath"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compare-confed-aspath",
                value_type: ValueKind::Enum,
                in_sections: &["bestpath"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compare-originator-id",
                value_type: ValueKind::Enum,
                in_sections: &["bestpath"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compare-routerid",
                value_type: ValueKind::Enum,
                in_sections: &["bestpath"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "med",
                value_type: ValueKind::String,
                in_sections: &["bestpath"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tie-break-on-age",
                value_type: ValueKind::Enum,
                in_sections: &["bestpath"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override",
                value_type: ValueKind::Enum,
                in_sections: &["capability-negotiate"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["capability-negotiate"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict-match",
                value_type: ValueKind::Enum,
                in_sections: &["capability-negotiate"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dynamic",
                value_type: ValueKind::Enum,
                in_sections: &["capability"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-restart",
                value_type: ValueKind::Enum,
                in_sections: &["capability"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "orf",
                value_type: ValueKind::String,
                in_sections: &["capability"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-refresh",
                value_type: ValueKind::Enum,
                in_sections: &["capability"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-to-client-reflection",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cluster-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "confederation",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "identifier",
                        value_type: ValueKind::Integer,
                        in_sections: &["confederation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "peers",
                        value_type: ValueKind::Boolean,
                        in_sections: &["confederation"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identifier",
                value_type: ValueKind::Integer,
                in_sections: &["confederation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers",
                value_type: ValueKind::Boolean,
                in_sections: &["confederation"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dampening",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "reachability-half-life",
                        value_type: ValueKind::Integer,
                        in_sections: &["dampening"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "reuse",
                        value_type: ValueKind::Integer,
                        in_sections: &["dampening"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "route-map",
                        value_type: ValueKind::Boolean,
                        in_sections: &["dampening"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "state",
                        value_type: ValueKind::Enum,
                        in_sections: &["dampening"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "suppress",
                        value_type: ValueKind::Integer,
                        in_sections: &["dampening"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "suppress-max",
                        value_type: ValueKind::Integer,
                        in_sections: &["dampening"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "unreachability-half-life",
                        value_type: ValueKind::Integer,
                        in_sections: &["dampening"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reachability-half-life",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::Boolean,
                in_sections: &["dampening"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["dampening"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress-max",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unreachability-half-life",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-local-preference",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::Boolean,
                in_sections: &["default-originate"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["default-originate"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deterministic-med",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distance",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "access-list",
                        value_type: ValueKind::Boolean,
                        in_sections: &["distance"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "distance",
                        value_type: ValueKind::Integer,
                        in_sections: &["distance"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "external",
                        value_type: ValueKind::Integer,
                        in_sections: &["distance"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "internal",
                        value_type: ValueKind::Integer,
                        in_sections: &["distance"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local",
                        value_type: ValueKind::Integer,
                        in_sections: &["distance"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-list",
                value_type: ValueKind::Boolean,
                in_sections: &["distance"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distance",
                value_type: ValueKind::Integer,
                in_sections: &["distance"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external",
                value_type: ValueKind::Integer,
                in_sections: &["distance"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal",
                value_type: ValueKind::Integer,
                in_sections: &["distance"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local",
                value_type: ValueKind::Integer,
                in_sections: &["distance"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "in",
                value_type: ValueKind::Boolean,
                in_sections: &["distribute-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "out",
                value_type: ValueKind::Boolean,
                in_sections: &["distribute-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-first-as",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fast-external-failover",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "in",
                value_type: ValueKind::Boolean,
                in_sections: &["filter-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "out",
                value_type: ValueKind::Boolean,
                in_sections: &["filter-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-restart",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "graceful-reset",
                        value_type: ValueKind::Enum,
                        in_sections: &["graceful-restart"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "restart-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["graceful-restart"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "stalepath-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["graceful-restart"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-reset",
                value_type: ValueKind::Enum,
                in_sections: &["graceful-restart"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart-time",
                value_type: ValueKind::Integer,
                in_sections: &["graceful-restart"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stalepath-time",
                value_type: ValueKind::Integer,
                in_sections: &["graceful-restart"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-shutdown",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "capable",
                        value_type: ValueKind::Enum,
                        in_sections: &["graceful-shutdown"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-preference",
                        value_type: ValueKind::Integer,
                        in_sections: &["graceful-shutdown"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["graceful-shutdown"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "timer",
                        value_type: ValueKind::Integer,
                        in_sections: &["graceful-shutdown"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capable",
                value_type: ValueKind::Enum,
                in_sections: &["graceful-shutdown"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-preference",
                value_type: ValueKind::Integer,
                in_sections: &["graceful-shutdown"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                in_sections: &["graceful-shutdown"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timer",
                value_type: ValueKind::Integer,
                in_sections: &["graceful-shutdown"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hold-time",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keep-alive",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-as",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-neighbor-changes",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold",
                value_type: ValueKind::Boolean,
                in_sections: &["maximum-prefix"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Integer,
                in_sections: &["maximum-prefix"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warning-only",
                value_type: ValueKind::Enum,
                in_sections: &["maximum-prefix"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "confed",
                value_type: ValueKind::Enum,
                in_sections: &["med"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "missing-as-worst",
                value_type: ValueKind::Enum,
                in_sections: &["med"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-recv-med",
                value_type: ValueKind::Enum,
                in_sections: &["med"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-send-med",
                value_type: ValueKind::Enum,
                in_sections: &["med"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "neighbor",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "address-family",
                        value_type: ValueKind::Reference,
                        in_sections: &["neighbor"],
                        shape_kind: Some(ValueKind::List),
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "advertisement-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-infinite-hold-time",
                        value_type: ValueKind::Enum,
                        in_sections: &["neighbor"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "as-origination-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "capability",
                        value_type: ValueKind::String,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "capability-negotiate",
                        value_type: ValueKind::String,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "collide-established",
                        value_type: ValueKind::Enum,
                        in_sections: &["neighbor"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "connect-timer",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::Boolean,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ebgp-multihop",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enabled",
                        value_type: ValueKind::Enum,
                        in_sections: &["neighbor"],
                        enum_values: &["false", "true"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-multihop",
                        value_type: ValueKind::Enum,
                        in_sections: &["neighbor"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "fall-over",
                        value_type: ValueKind::Boolean,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "graceful-shutdown",
                        value_type: ValueKind::String,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hold-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "keep-alive",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-as",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "passive",
                        value_type: ValueKind::Enum,
                        in_sections: &["neighbor"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "password",
                        value_type: ValueKind::Boolean,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "peer-group",
                        value_type: ValueKind::Boolean,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remote-as",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "restart-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "update-source",
                        value_type: ValueKind::Boolean,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "version",
                        value_type: ValueKind::Integer,
                        in_sections: &["neighbor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "vlan",
                        value_type: ValueKind::Reference,
                        in_sections: &["neighbor"],
                        allow_none: true,
                        references: &["net_vlan"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-family",
                value_type: ValueKind::Reference,
                in_sections: &["neighbor"],
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "advertisement-interval",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-infinite-hold-time",
                value_type: ValueKind::Enum,
                in_sections: &["neighbor"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-origination-interval",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capability",
                value_type: ValueKind::String,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capability-negotiate",
                value_type: ValueKind::String,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collide-established",
                value_type: ValueKind::Enum,
                in_sections: &["neighbor"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connect-timer",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                in_sections: &["neighbor"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ebgp-multihop",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["neighbor"],
                enum_values: &["false", "true"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-multihop",
                value_type: ValueKind::Enum,
                in_sections: &["neighbor"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fall-over",
                value_type: ValueKind::Boolean,
                in_sections: &["neighbor"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-shutdown",
                value_type: ValueKind::String,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hold-time",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keep-alive",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-as",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passive",
                value_type: ValueKind::Enum,
                in_sections: &["neighbor"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Boolean,
                in_sections: &["neighbor"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-group",
                value_type: ValueKind::Boolean,
                in_sections: &["neighbor"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-as",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart-time",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-source",
                value_type: ValueKind::Boolean,
                in_sections: &["neighbor"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                in_sections: &["neighbor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                in_sections: &["neighbor"],
                allow_none: true,
                references: &["net_vlan"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "backdoor",
                        value_type: ValueKind::Enum,
                        in_sections: &["network"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "route-map",
                        value_type: ValueKind::Boolean,
                        in_sections: &["network"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "backdoor",
                value_type: ValueKind::Enum,
                in_sections: &["network"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::Boolean,
                in_sections: &["network"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-list",
                value_type: ValueKind::Boolean,
                in_sections: &["orf"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-group",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "address-family",
                        value_type: ValueKind::Reference,
                        in_sections: &["peer-group"],
                        shape_kind: Some(ValueKind::List),
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "advertisement-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-infinite-hold-time",
                        value_type: ValueKind::Enum,
                        in_sections: &["peer-group"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "as-origination-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "capability",
                        value_type: ValueKind::String,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "capability-negotiate",
                        value_type: ValueKind::String,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "collide-established",
                        value_type: ValueKind::Enum,
                        in_sections: &["peer-group"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "connect-timer",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::Boolean,
                        in_sections: &["peer-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ebgp-multihop",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enabled",
                        value_type: ValueKind::Enum,
                        in_sections: &["peer-group"],
                        enum_values: &["false", "true"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-multihop",
                        value_type: ValueKind::Enum,
                        in_sections: &["peer-group"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "fall-over",
                        value_type: ValueKind::Boolean,
                        in_sections: &["peer-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "graceful-shutdown",
                        value_type: ValueKind::String,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hold-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "keep-alive",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-as",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "passive",
                        value_type: ValueKind::Enum,
                        in_sections: &["peer-group"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "password",
                        value_type: ValueKind::Boolean,
                        in_sections: &["peer-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remote-as",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "restart-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "update-source",
                        value_type: ValueKind::Boolean,
                        in_sections: &["peer-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "version",
                        value_type: ValueKind::Integer,
                        in_sections: &["peer-group"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-family",
                value_type: ValueKind::Reference,
                in_sections: &["peer-group"],
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "advertisement-interval",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-infinite-hold-time",
                value_type: ValueKind::Enum,
                in_sections: &["peer-group"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-origination-interval",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capability",
                value_type: ValueKind::String,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capability-negotiate",
                value_type: ValueKind::String,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collide-established",
                value_type: ValueKind::Enum,
                in_sections: &["peer-group"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connect-timer",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                in_sections: &["peer-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ebgp-multihop",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                in_sections: &["peer-group"],
                enum_values: &["false", "true"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-multihop",
                value_type: ValueKind::Enum,
                in_sections: &["peer-group"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fall-over",
                value_type: ValueKind::Boolean,
                in_sections: &["peer-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "graceful-shutdown",
                value_type: ValueKind::String,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hold-time",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keep-alive",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-as",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passive",
                value_type: ValueKind::Enum,
                in_sections: &["peer-group"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Boolean,
                in_sections: &["peer-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-as",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart-time",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-source",
                value_type: ValueKind::Boolean,
                in_sections: &["peer-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                in_sections: &["peer-group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "in",
                value_type: ValueKind::Boolean,
                in_sections: &["prefix-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "out",
                value_type: ValueKind::Boolean,
                in_sections: &["prefix-list"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profile",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redistribute",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "route-map",
                    value_type: ValueKind::Boolean,
                    in_sections: &["redistribute"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::Boolean,
                in_sections: &["redistribute"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "in",
                value_type: ValueKind::Boolean,
                in_sections: &["route-map"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "out",
                value_type: ValueKind::Boolean,
                in_sections: &["route-map"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "router-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scan-time",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-delay",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_community_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing community-list"],
        },
        header_types: &[("net", "routing community-list")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "community",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_debug",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing debug"],
        },
        header_types: &[("net", "routing debug")],
        properties: &[
            BigipPropertySpec {
                name: "bfd",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "all",
                    "event",
                    "ipc-error",
                    "ipc-event",
                    "nsm",
                    "packet",
                    "session",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bgp",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "all",
                    "bfd",
                    "dampening",
                    "events",
                    "filters",
                    "fsm",
                    "keepalives",
                    "nht",
                    "nsm",
                    "updates",
                    "updates-in",
                    "updates-out",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "all",
                    "bfd",
                    "events",
                    "ha",
                    "ha-all",
                    "kernel",
                    "packet",
                    "packet-detail",
                    "packet-recv",
                    "packet-send",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_extcommunity_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing extcommunity-list"],
        },
        header_types: &[("net", "routing extcommunity-list")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rt",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "soo",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rt",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "soo",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_prefix_list",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing prefix-list"],
        },
        header_types: &[("net", "routing prefix-list")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "prefix",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "prefix-len-range",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-len-range",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_profile_bgp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing profile bgp"],
        },
        header_types: &[("net", "routing profile bgp")],
        properties: &[
            BigipPropertySpec {
                name: "adj-out",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aggregate-nexthop-check",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-local-count",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bgp-multiple-instance",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Boolean,
                allow_none: true,
                references: &["net_routing_profile_bgp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "extended-asn-cap",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-paths",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "ebgp",
                        value_type: ValueKind::Integer,
                        in_sections: &["max-paths"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ibgp",
                        value_type: ValueKind::Integer,
                        in_sections: &["max-paths"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ebgp",
                value_type: ValueKind::Integer,
                in_sections: &["max-paths"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ibgp",
                value_type: ValueKind::Integer,
                in_sections: &["max-paths"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nexthop-trigger",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "delay",
                        value_type: ValueKind::Integer,
                        in_sections: &["nexthop-trigger"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "state",
                        value_type: ValueKind::Enum,
                        in_sections: &["nexthop-trigger"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "delay",
                value_type: ValueKind::Integer,
                in_sections: &["nexthop-trigger"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["nexthop-trigger"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rfc1771",
                value_type: ValueKind::String,
                block: &[
                    BigipPropertySpec {
                        name: "path-select",
                        value_type: ValueKind::Enum,
                        in_sections: &["rfc1771"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "strict",
                        value_type: ValueKind::Enum,
                        in_sections: &["rfc1771"],
                        enum_values: &["disabled", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-select",
                value_type: ValueKind::Enum,
                in_sections: &["rfc1771"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict",
                value_type: ValueKind::Enum,
                in_sections: &["rfc1771"],
                enum_values: &["disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "router-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_routing_route_map",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["routing route-map"],
        },
        header_types: &[("net", "routing route-map")],
        properties: &[
            BigipPropertySpec {
                name: "access-list",
                value_type: ValueKind::Boolean,
                in_sections: &["address"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-list",
                value_type: ValueKind::Boolean,
                in_sections: &["address"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                in_sections: &["aggregator"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as",
                value_type: ValueKind::Integer,
                in_sections: &["aggregator"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "additive",
                value_type: ValueKind::Boolean,
                in_sections: &["community"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exact-match",
                value_type: ValueKind::Boolean,
                in_sections: &["community"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exact-set",
                value_type: ValueKind::Boolean,
                in_sections: &["community"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Integer,
                in_sections: &["community"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reachability-half-life",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress-max",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unreachability-half-life",
                value_type: ValueKind::Integer,
                in_sections: &["dampening"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Boolean,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::Reference,
                shape_kind: Some(ValueKind::List),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Boolean,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "match",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "set",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Boolean,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "set",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exact-match",
                value_type: ValueKind::Boolean,
                in_sections: &["extcommunity"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rt",
                value_type: ValueKind::Boolean,
                in_sections: &["extcommunity"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "soo",
                value_type: ValueKind::Boolean,
                in_sections: &["extcommunity"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "next-hop",
                value_type: ValueKind::String,
                in_sections: &["ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                in_sections: &["ipv4"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "next-hop",
                value_type: ValueKind::String,
                in_sections: &["ipv4"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer",
                value_type: ValueKind::String,
                in_sections: &["ipv4"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                in_sections: &["ipv6"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "next-hop",
                value_type: ValueKind::String,
                in_sections: &["ipv6"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer",
                value_type: ValueKind::String,
                in_sections: &["ipv6"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-path",
                value_type: ValueKind::Boolean,
                in_sections: &["match"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::String,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "extcommunity",
                value_type: ValueKind::String,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4",
                value_type: ValueKind::String,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6",
                value_type: ValueKind::String,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metric",
                value_type: ValueKind::Integer,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin",
                value_type: ValueKind::Boolean,
                in_sections: &["match"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-type",
                value_type: ValueKind::Boolean,
                in_sections: &["match"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tag",
                value_type: ValueKind::Integer,
                in_sections: &["match"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                in_sections: &["match"],
                allow_none: true,
                references: &["net_vlan"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Boolean,
                in_sections: &["metric"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Boolean,
                in_sections: &["metric"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-list",
                value_type: ValueKind::Boolean,
                in_sections: &["next-hop"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                in_sections: &["next-hop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local",
                value_type: ValueKind::String,
                in_sections: &["next-hop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefix-list",
                value_type: ValueKind::Boolean,
                in_sections: &["next-hop"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-list",
                value_type: ValueKind::Boolean,
                in_sections: &["peer"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aggregator",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "as-path-prepend",
                value_type: ValueKind::Integer,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "atomic-aggregate",
                value_type: ValueKind::Boolean,
                in_sections: &["set"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dampening",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "extcommunity",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "level",
                value_type: ValueKind::Boolean,
                in_sections: &["set"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-preference",
                value_type: ValueKind::Integer,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metric",
                value_type: ValueKind::String,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin",
                value_type: ValueKind::Boolean,
                in_sections: &["set"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "originator-id",
                value_type: ValueKind::Integer,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tag",
                value_type: ValueKind::Integer,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                in_sections: &["set"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_self",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["self"],
        },
        header_types: &[("net", "self")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-source",
                value_type: ValueKind::Enum,
                enum_values: &["from-management", "from-user"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-service",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "default", "none"],
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
                name: "fw-context-stat",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-enforced-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-enforced-policy-rules",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-staged-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fw-staged-policy-rules",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::String,
                allow_none: true,
                references: &["cm_traffic_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                required: true,
                references: &["net_vlan", "net_vlan_allowed", "net_vlan_group"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_self_allow",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["self-allow"],
        },
        header_types: &[("net", "self-allow")],
        properties: &[BigipPropertySpec {
            name: "defaults",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "none"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_service_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["service-policy"],
        },
        header_types: &[("net", "service-policy")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-misuse-policy",
                value_type: ValueKind::Reference,
                references: &["security_firewall_port_misuse_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timer-policy",
                value_type: ValueKind::Reference,
                references: &["net_timer_policy"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_sfc_chain",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["sfc chain"],
        },
        header_types: &[("net", "sfc chain")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hopkey",
                value_type: ValueKind::Enum,
                enum_values: &["interface", "service-index"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-index",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_sfc_sf",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["sfc sf"],
        },
        header_types: &[("net", "sfc sf")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "egress-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ingress-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address",
                value_type: ValueKind::String,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsh-aware",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-name",
                value_type: ValueKind::Reference,
                allow_none: true,
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
                name: "virtual-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "analytics_virtual_report",
                    "analytics_virtual_scheduled_report",
                    "ltm_monitor_virtual_location",
                    "ltm_virtual",
                    "ltm_virtual_address",
                    "security_dos_virtual",
                    "security_protocol_inspection_virtual_servers",
                    "vcmp_virtual_disk",
                    "vcmp_virtual_disk_template",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_stp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["stp"],
        },
        header_types: &[("net", "stp")],
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
                name: "instance-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interfaces",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["interfaces"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "external-path-cost",
                        value_type: ValueKind::Integer,
                        in_sections: &["interfaces"],
                        default: Some("20000"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "internal-path-cost",
                        value_type: ValueKind::Integer,
                        in_sections: &["interfaces"],
                        default: Some("20000"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "priority",
                        value_type: ValueKind::Integer,
                        in_sections: &["interfaces"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["interfaces"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-path-cost",
                value_type: ValueKind::Integer,
                in_sections: &["interfaces"],
                default: Some("20000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-path-cost",
                value_type: ValueKind::Integer,
                in_sections: &["interfaces"],
                default: Some("20000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                in_sections: &["interfaces"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trunks",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["trunks"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "external-path-cost",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        default: Some("20000"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "internal-path-cost",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        default: Some("20000"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "priority",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["trunks"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-path-cost",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                default: Some("20000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-path-cost",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                default: Some("20000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_stp_globals",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["stp-globals"],
        },
        header_types: &[("net", "stp-globals")],
        properties: &[
            BigipPropertySpec {
                name: "config-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "config-revision",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fwd-delay",
                value_type: ValueKind::Integer,
                default: Some("15 seconds, and the valid range is 4 to 30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hello-time",
                value_type: ValueKind::Integer,
                default: Some("2 seconds, and the valid range is 1 - 10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-age",
                value_type: ValueKind::Integer,
                default: Some("20 seconds, and the valid range is 6-40 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-hops",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "mstp", "passthru", "rstp", "stp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transmit-hold",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_timer_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["timer-policy"],
        },
        header_types: &[("net", "timer-policy")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
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
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["rules"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "destination-ports",
                        value_type: ValueKind::List,
                        in_sections: &["rules"],
                        allow_none: true,
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ip-protocol",
                        value_type: ValueKind::Reference,
                        in_sections: &["rules"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "timers",
                        value_type: ValueKind::List,
                        in_sections: &["rules"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["rules"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-ports",
                value_type: ValueKind::List,
                in_sections: &["rules"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Reference,
                in_sections: &["rules"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timers",
                value_type: ValueKind::List,
                in_sections: &["rules"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "timers"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "timers"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_trunk",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["trunk"],
        },
        header_types: &[("net", "trunk")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bandwidth",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distribution-hash",
                value_type: ValueKind::Enum,
                enum_values: &["dst-mac", "src-dst-mac"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interfaces",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lacp",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lacp-mode",
                value_type: ValueKind::Enum,
                enum_values: &["active", "passive"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lacp-timeout",
                value_type: ValueKind::Enum,
                enum_values: &["long", "short"],
                default: Some("long"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "link-select-policy",
                value_type: ValueKind::Enum,
                enum_values: &["auto", "maximum-bandwidth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac-address",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "qinq-ethertype",
                value_type: ValueKind::String,
                default: Some("set to 0x8100"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stp-reset",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_etherip",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels etherip"],
        },
        header_types: &[("net", "tunnels etherip")],
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
                references: &["net_tunnels_etherip"],
                default: Some("etherip"),
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
            kind: "net_tunnels_fec",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels fec"],
        },
        header_types: &[("net", "tunnels fec")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("1500 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-max-packets",
                value_type: ValueKind::Integer,
                default: Some("512"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-queues",
                value_type: ValueKind::Integer,
                default: Some("32"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["net_tunnels_fec"],
                default: Some("fec"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encode-max-delay",
                value_type: ValueKind::Integer,
                default: Some("500 microseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keepalive-interval",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lzo",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repair-adaptive",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repair-packets",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-adaptive",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-packets",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "udp-port",
                value_type: ValueKind::Integer,
                default: Some("8288"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_geneve",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels geneve"],
        },
        header_types: &[("net", "tunnels geneve")],
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
                allow_none: true,
                references: &["net_tunnels_geneve"],
                default: Some("geneve"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multicast", "multipoint", "none"],
                default: Some("multipoint"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("6081"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_gre",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels gre"],
        },
        header_types: &[("net", "tunnels gre")],
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
                references: &["net_tunnels_gre"],
                default: Some("gre"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encapsulation",
                value_type: ValueKind::Enum,
                enum_values: &["nvgre", "standard"],
                default: Some("standard"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multipoint", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tx-csum",
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
            kind: "net_tunnels_ipip",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ipip"],
        },
        header_types: &[("net", "tunnels ipip")],
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
                references: &["net_tunnels_ipip"],
                default: Some("ipip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ds-lite",
                value_type: ValueKind::Unknown,
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proto",
                value_type: ValueKind::Enum,
                enum_values: &["IPv4", "IPv6"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("IPv4"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_ipsec",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ipsec"],
        },
        header_types: &[("net", "tunnels ipsec")],
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
                allow_none: true,
                references: &["net_tunnels_ipsec"],
                default: Some("ipsec"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-selector",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_traffic_selector"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_lw4o6",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels lw4o6"],
        },
        header_types: &[("net", "tunnels lw4o6")],
        properties: &[
            BigipPropertySpec {
                name: "all-protocols-pass",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
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
                allow_none: true,
                references: &["net_tunnels_lw4o6"],
                default: Some("lw4o6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lwtbl-file",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "psid-length",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_map",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels map"],
        },
        header_types: &[("net", "tunnels map")],
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
                allow_none: true,
                references: &["net_tunnels_map"],
                default: Some("map"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ea-bits-length",
                value_type: ValueKind::Integer,
                default: Some("32 (IPv4 prefix 24 bits + PSID 8 bits)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-offset",
                value_type: ValueKind::Integer,
                default: Some("6"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_ppp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ppp"],
        },
        header_types: &[("net", "tunnels ppp")],
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
                allow_none: true,
                references: &["net_tunnels_ppp"],
                default: Some("ppp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lcp-echo-failure",
                value_type: ValueKind::Integer,
                default: Some("4"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lcp-echo-interval",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vj",
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
            kind: "net_tunnels_tcp_forward",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels tcp-forward"],
        },
        header_types: &[("net", "tunnels tcp-forward")],
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
                allow_none: true,
                references: &["net_tunnels_tcp_forward"],
                default: Some("tcp-forward"),
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
            kind: "net_tunnels_tunnel",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels tunnel"],
        },
        header_types: &[("net", "tunnels tunnel")],
        properties: &[
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
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["bidirectional", "inbound", "outbound"],
                default: Some("bidirectional"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profile",
                value_type: ValueKind::Reference,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secondary-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tos",
                value_type: ValueKind::Integer,
                default: Some("preserve"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["cm_traffic_group"],
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
                name: "use-pmtu",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_v6rd",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels v6rd"],
        },
        header_types: &[("net", "tunnels v6rd")],
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
                allow_none: true,
                references: &["net_tunnels_v6rd"],
                default: Some("v6rd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4prefixlen",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v6rdprefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v6rdprefixlen",
                value_type: ValueKind::Integer,
                default: Some("56"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_vxlan",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels vxlan"],
        },
        header_types: &[("net", "tunnels vxlan")],
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
                allow_none: true,
                references: &["net_tunnels_vxlan"],
                default: Some("vxlan"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encapsulation-type",
                value_type: ValueKind::Enum,
                enum_values: &["vxlan", "vxlan-gpe"],
                default: Some("vxlan"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multicast", "multipoint", "none", "replicator"],
                default: Some("multicast"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("4789"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_wccp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels wccp"],
        },
        header_types: &[("net", "tunnels wccp")],
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
                references: &["net_tunnels_wccp"],
                default: Some("wccpgre"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wccp-version",
                value_type: ValueKind::Enum,
                enum_values: &["1", "2"],
                default: Some("2"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_vlan",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["vlan"],
        },
        header_types: &[("net", "vlan")],
        properties: &[
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
                name: "cmp-hash",
                value_type: ValueKind::Enum,
                enum_values: &["default", "dst-ip", "src-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-tag",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dag-adjustment",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["bit-roll", "nibble-roll", "none", "xor-5mid-xor-5low"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dag-round-robin",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dag-tunnel",
                value_type: ValueKind::Enum,
                enum_values: &["inner", "outer"],
                default: Some("outer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failsafe",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failsafe-action",
                value_type: ValueKind::Enum,
                enum_values: &["failover", "failover-restart-tm", "reboot", "restart-all"],
                default: Some("failover-restart-tm"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failsafe-timeout",
                value_type: ValueKind::Integer,
                default: Some("90 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fwd-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["l3", "none", "passive", "virtual-wire"],
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hardware-syncookie",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interfaces",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-prefix-len",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "learning",
                value_type: ValueKind::Enum,
                enum_values: &["disable-drop", "disable-forward", "enable-forward"],
                default: Some("enable-forward"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Integer,
                default: Some("1500"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nti",
                value_type: ValueKind::Integer,
                default: Some("4096"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sflow",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "poll-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["sflow"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "poll-interval-global",
                        value_type: ValueKind::Enum,
                        in_sections: &["sflow"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("yes"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sampling-rate",
                        value_type: ValueKind::Integer,
                        in_sections: &["sflow"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sampling-rate-global",
                        value_type: ValueKind::Enum,
                        in_sections: &["sflow"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("yes"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                in_sections: &["sflow"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval-global",
                value_type: ValueKind::Enum,
                in_sections: &["sflow"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate",
                value_type: ValueKind::Integer,
                in_sections: &["sflow"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate-global",
                value_type: ValueKind::Enum,
                in_sections: &["sflow"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-checking",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "syn-flood-rate-limit",
                value_type: ValueKind::Integer,
                default: Some("set at 1000 packets per second"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "syncache-threshold",
                value_type: ValueKind::Integer,
                default: Some("set to 6000 packets"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tag",
                value_type: ValueKind::Enum,
                enum_values: &["4096"],
                default: Some(
                    "to not use this option, and the system assigns a tag number between 1 to 4094",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tag-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["customer", "double", "none", "service"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_vlan_group",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["vlan-group"],
        },
        header_types: &[("net", "vlan-group")],
        properties: &[
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
                name: "bridge-in-standby",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bridge-multicast",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bridge-traffic",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "migration-keepalive",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["opaque", "translucent", "transparent", "virtual-wire"],
                default: Some("translucent"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-excludes",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_wccp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["wccp"],
        },
        header_types: &[("net", "wccp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-timeout",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "services",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "alt-hash-fields",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        allow_none: true,
                        enum_values: &["dest-ip", "none", "src-ip"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["services"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hash-fields",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        allow_none: true,
                        enum_values: &["dest-ip", "none", "src-ip"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "password",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        allow_none: true,
                        enum_values: &["dest", "none", "source"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ports",
                        value_type: ValueKind::Integer,
                        in_sections: &["services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "priority",
                        value_type: ValueKind::Integer,
                        in_sections: &["services"],
                        default: Some("100"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "protocol",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        enum_values: &["tcp", "udp"],
                        default: Some("tcp"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirection-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        enum_values: &["gre", "l2"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "return-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        enum_values: &["gre", "l2"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "routers",
                        value_type: ValueKind::List,
                        in_sections: &["services"],
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "traffic-assign",
                        value_type: ValueKind::Enum,
                        in_sections: &["services"],
                        enum_values: &["hash", "mask"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "tunnel-local-address",
                        value_type: ValueKind::String,
                        in_sections: &["services"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "tunnel-remote-addresses",
                        value_type: ValueKind::List,
                        in_sections: &["services"],
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight",
                        value_type: ValueKind::Integer,
                        in_sections: &["services"],
                        default: Some("50"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alt-hash-fields",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                allow_none: true,
                enum_values: &["dest-ip", "none", "src-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["services"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-fields",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                allow_none: true,
                enum_values: &["dest-ip", "none", "src-ip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-type",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                allow_none: true,
                enum_values: &["dest", "none", "source"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ports",
                value_type: ValueKind::Integer,
                in_sections: &["services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                in_sections: &["services"],
                default: Some("100"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                enum_values: &["tcp", "udp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirection-method",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                enum_values: &["gre", "l2"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "return-method",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                enum_values: &["gre", "l2"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "routers",
                value_type: ValueKind::List,
                in_sections: &["services"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-assign",
                value_type: ValueKind::Enum,
                in_sections: &["services"],
                enum_values: &["hash", "mask"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-local-address",
                value_type: ValueKind::String,
                in_sections: &["services"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-remote-addresses",
                value_type: ValueKind::List,
                in_sections: &["services"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                in_sections: &["services"],
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
