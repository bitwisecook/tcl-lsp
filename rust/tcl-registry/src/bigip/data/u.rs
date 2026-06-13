//! Generated BIG-IP object specs. DO NOT EDIT — produced by
//! `scripts/registry-audit/gen_bigip_rust.py` from the reconciled
//! Python `OBJECT_SPECS` (canonical `origin/main` baseline).
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "util_ipsecalgdb",
        table_name: None,
        resolver_name: None,
        module: Some("util"),
        object_types: &["ipsecalgdb"],
    },
    header_types: &[("util", "ipsecalgdb")],
    properties: &[],
}];
