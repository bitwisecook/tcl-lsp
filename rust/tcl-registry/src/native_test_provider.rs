// SPDX-License-Identifier: AGPL-3.0-or-later
//! Known provider labels used by original native fixture comparisons.

pub(crate) fn profile(provider: &str) -> &'static tcl_dialect::DialectProfile {
    crate::model::ingress::resolve_known_environment(provider)
        .expect("native fixture requires a known provider environment")
        .unit_profile()
}
