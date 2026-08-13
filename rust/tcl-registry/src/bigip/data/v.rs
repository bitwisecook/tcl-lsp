//! Generated BIG-IP object specs. DO NOT EDIT.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_guest",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["guest"],
        },
        header_types: &[("vcmp", "guest")],
        properties: &[
            BigipPropertySpec {
                name: "allowed-slots",
                value_type: ValueKind::List,
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
                name: "boot-priority",
                value_type: ValueKind::Integer,
                default: Some("65535"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capabilities",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cores-per-slot",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-hotfix",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-image",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-gw",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-ip",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-network",
                value_type: ValueKind::Enum,
                enum_values: &["bridged", "isolated"],
                default: Some("Bridged"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-slots",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "slots",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["configured", "deployed", "provisioned"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-profile",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-disk",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_traffic_profile",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["traffic-profile"],
        },
        header_types: &[("vcmp", "traffic-profile")],
        properties: &[BigipPropertySpec {
            name: "color-policer",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_virtual_disk",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["virtual-disk"],
        },
        header_types: &[("vcmp", "virtual-disk")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_virtual_disk_template",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["virtual-disk-template"],
        },
        header_types: &[("vcmp", "virtual-disk-template")],
        properties: &[],
    },
];
