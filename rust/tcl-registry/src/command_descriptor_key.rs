// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact selected Registry descriptor labels, independent of written Tcl names.

/// A pure label projected by the Registry's selected descriptor/fact producer.
/// It grants no runtime command binding, Native naming policy, invocation or
/// source-word authority. Descriptor labels are retained byte-for-byte; their
/// punctuation cannot establish a constructed root marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalCommandDescriptorKey<'a>(&'a str);
impl<'a> CanonicalCommandDescriptorKey<'a> {
    pub(crate) const fn from_registry_label(label: &'a str) -> Self {
        Self(label)
    }
    /// The selected descriptor's exact original label.
    #[must_use]
    pub const fn label(self) -> &'a str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_descriptor_key_preserves_colons_and_grants_no_written_name_recipe() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        // Pure descriptor data only; no Native binding or written-name lookup.
        for label in ["expr", "::expr", ":::::member", ":::", "A:::member"] {
            let selected = CanonicalCommandDescriptorKey::from_registry_label(label);
            assert_eq!(selected.label(), label);
        }
        let selected = CanonicalCommandDescriptorKey::from_registry_label(":::::member");
        assert_ne!(
            selected.label(),
            tcl_syntax::naming::canonical_written_command(selected.label())
        );
        assert_ne!(
            selected.label(),
            tcl_syntax::naming::unroot_rooted_key(selected.label()).unwrap()
        );
    }
}
