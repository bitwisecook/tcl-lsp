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
    /// Exact optional-root metadata coordinate, without written-name parsing.
    #[must_use]
    pub fn rooted_binding_label(self) -> String {
        rooted_command_descriptor_label(self.0)
    }
}

/// Add an optional metadata root marker while preserving every descriptor byte.
/// Empty labels remain empty; this grants no written lookup or registration.
#[must_use]
pub fn rooted_command_descriptor_label(label: &str) -> String {
    if label.is_empty() || tcl_syntax::naming::unroot_rooted_key(label).is_some() {
        label.to_owned()
    } else {
        tcl_syntax::naming::root_unrooted_key(label)
    }
}

/// Compare descriptor metadata with one optional root marker. Separator runs,
/// empty tails and lone colons remain exact; no live identity is established.
#[must_use]
pub fn command_descriptor_labels_match(left: &str, right: &str) -> bool {
    rooted_command_descriptor_label(left) == rooted_command_descriptor_label(right)
}

/// Index selected descriptor coordinates, declining distinct labels that collide.
/// The retained value is the original label, never a reconstructed identity.
pub fn unique_command_descriptor_coordinates<'a>(
    keys: impl IntoIterator<Item = CanonicalCommandDescriptorKey<'a>>,
) -> impl Iterator<Item = (String, CanonicalCommandDescriptorKey<'a>)> {
    use std::collections::{BTreeMap, btree_map::Entry};
    let mut entries = BTreeMap::new();
    for key in keys {
        match entries.entry(key.rooted_binding_label()) {
            Entry::Vacant(entry) => {
                entry.insert(Some(key));
            }
            Entry::Occupied(mut entry) if *entry.get() != Some(key) => {
                entry.insert(None);
            }
            Entry::Occupied(_) => {}
        }
    }
    entries
        .into_iter()
        .filter_map(|(coordinate, key)| Some((coordinate, key?)))
}

impl crate::CommandSpec {
    /// Exact label of this selected descriptor; no runtime binding is supplied.
    #[must_use]
    pub const fn descriptor_key(&self) -> CanonicalCommandDescriptorKey<'static> {
        CanonicalCommandDescriptorKey::from_registry_label(self.name)
    }
}

impl crate::CommandSpec {
    /// Exact descriptor label; no written-name or runtime binding authority.
    #[must_use]
    pub const fn descriptor_key(&self) -> CanonicalCommandDescriptorKey<'static> {
        CanonicalCommandDescriptorKey::from_registry_label(self.name)
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
    #[test]
    fn descriptor_coordinates_preserve_empty_tails_and_decline_distinct_labels() {
        let labels = [
            "",
            "::",
            "NAME::",
            "NAME:::member",
            "::::NAME",
            ":NAME",
            "collision",
            "::collision",
        ];
        let keys = labels.map(CanonicalCommandDescriptorKey::from_registry_label);
        let indexed: std::collections::BTreeMap<_, _> =
            unique_command_descriptor_coordinates(keys).collect();
        for (coordinate, label) in [
            ("", ""),
            ("::", "::"),
            ("::NAME::", "NAME::"),
            ("::NAME:::member", "NAME:::member"),
            ("::::NAME", "::::NAME"),
            (":::NAME", ":NAME"),
        ] {
            assert_eq!(indexed.get(coordinate).unwrap().label(), label);
        }
        assert!(!indexed.contains_key("::collision"));
        assert!(!command_descriptor_labels_match("::", ""));
        assert!(!command_descriptor_labels_match("::::NAME", "::NAME"));
        assert!(!command_descriptor_labels_match(
            "NAME:::member",
            "NAME::member"
        ));
        assert!(command_descriptor_labels_match("NAME::", "::NAME::"));
    }
}
