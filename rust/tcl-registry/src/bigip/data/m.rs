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
