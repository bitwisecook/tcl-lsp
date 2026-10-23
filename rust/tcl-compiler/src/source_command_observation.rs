// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative self-dispatch observation over authored source report tables.

/// Two report-table aliases of independently selected self-dispatch metadata.
/// A negative table query supplies no live command, entered frame, receiver,
/// method-table, binding absence or Native name-policy authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfDispatchSourceTableProbe {
    bare: String,
    rooted: String,
}

impl SelfDispatchSourceTableProbe {
    /// Select only descriptors that the source tables can represent without
    /// changing geometry. Ambiguous descriptor/source projections decline.
    #[must_use]
    pub fn for_selected_descriptor(
        key: tcl_registry::CanonicalCommandDescriptorKey<'_>,
    ) -> Option<Self> {
        let rooted = key.rooted_binding_label();
        if rooted.is_empty() || tcl_syntax::naming::normalise_qualified_name(key.label()) != rooted
        {
            return None;
        }
        let bare = tcl_syntax::naming::unroot_rooted_key(&rooted)?.to_owned();
        if bare.is_empty() {
            return None;
        }
        Some(Self { bare, rooted })
    }

    /// Whether either authored source report key has an observed disturbance.
    /// The caller owns each table's interpreter domain and declaration purpose.
    #[must_use]
    pub fn any_reported_key(&self, mut contains: impl FnMut(&str) -> bool) -> bool {
        contains(&self.bare) || contains(&self.rooted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_report_aliases_preserve_literal_colons_and_refuse_colliding_geometry() {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let prototype = registry.get_exact("my").unwrap().clone();
        for label in [
            "::my",
            ":my",
            "Other::my",
            "Other:::my",
            "::::my",
            "my:::member",
            "",
            "::",
        ] {
            let mut spec = prototype.clone();
            spec.name = label;
            registry.insert(spec);
        }
        for (label, bare, rooted) in [
            ("my", "my", "::my"),
            ("::my", "my", "::my"),
            (":my", ":my", ":::my"),
            ("Other::my", "Other::my", "::Other::my"),
        ] {
            let selected = registry.get_exact(label).unwrap().descriptor_key();
            let probe = SelfDispatchSourceTableProbe::for_selected_descriptor(selected).unwrap();
            assert!(probe.any_reported_key(|name| name == bare));
            assert!(probe.any_reported_key(|name| name == rooted));
            assert!(!probe.any_reported_key(|name| name == "::Unrelated::my"));
            if label != "my" && label != "::my" {
                assert!(!probe.any_reported_key(|name| name == "::my"));
            }
        }
        for label in ["Other:::my", "::::my", "my:::member", "", "::"] {
            assert!(
                SelfDispatchSourceTableProbe::for_selected_descriptor(
                    registry.get_exact(label).unwrap().descriptor_key()
                )
                .is_none()
            );
        }
    }
}
