//! Generated BIG-IP object specs. DO NOT EDIT.
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "mgmt_shared_settings_api_status_availability",
            table_name: None,
            resolver_name: None,
            module: Some("mgmt"),
            object_types: &["shared settings api-status availability"],
        },
        header_types: &[("mgmt", "shared settings api-status availability")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "mgmt_shared_settings_api_status_log_resource",
            table_name: None,
            resolver_name: None,
            module: Some("mgmt"),
            object_types: &["shared settings api-status log resource"],
        },
        header_types: &[("mgmt", "shared settings api-status log resource")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "mgmt_shared_settings_api_status_log_resource_property",
            table_name: None,
            resolver_name: None,
            module: Some("mgmt"),
            object_types: &["shared settings api-status log resource-property"],
        },
        header_types: &[("mgmt", "shared settings api-status log resource-property")],
        properties: &[],
    },
];
