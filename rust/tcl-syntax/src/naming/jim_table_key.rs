// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim command-table spelling is independent of its root-stripped comparison key.

use super::{NativeNameProjection, NativeNameProtocol, NativeNamePurpose};
use tcl_core_types::NameBytes;

/// Retained publication spelling and comparison units for one Jim table entry.
/// This value describes a table key; it does not identify an original object header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeJimCommandTableKey {
    report: NameBytes,
    comparison: NameBytes,
}

impl NativeJimCommandTableKey {
    /// Retain a genuine purpose-selected publication or rename destination.
    #[must_use]
    pub fn from_projection(projection: &NativeNameProjection<'_>) -> Option<Self> {
        if !matches!(
            projection.purpose(),
            NativeNamePurpose::CommandPublication
                | NativeNamePurpose::AliasPublication
                | NativeNamePurpose::ChildAliasPublication
                | NativeNamePurpose::RenameDestination
        ) {
            return None;
        }
        Some(Self {
            report: projection.selected().into(),
            comparison: projection.jim_flat_key()?.into(),
        })
    }

    /// Retain an already constructed actual Jim comparison key. Written ingress
    /// must use the selected publication projection instead.
    #[must_use]
    pub fn from_comparison_key(protocol: NativeNameProtocol, key: &[u8]) -> Option<Self> {
        if !protocol.is_jim084() {
            return None;
        }
        let selected = protocol
            .alias_publication_input(super::NativeNameContext::root(), key)
            .ok()?;
        let key = Self::from_projection(&selected)?;
        (key.report == key.comparison).then_some(key)
    }

    #[must_use]
    pub fn report_bytes(&self) -> &[u8] {
        self.report.as_bytes()
    }
    #[must_use]
    pub fn comparison_bytes(&self) -> &[u8] {
        self.comparison.as_bytes()
    }

    /// Jim replaces the command value of an occupied hash entry, retaining its
    /// original key. A distinct comparison slot receives the incoming key.
    #[must_use]
    pub fn retain_for_replacement(self, occupied: Option<&Self>) -> Self {
        occupied
            .filter(|key| key.comparison == self.comparison)
            .cloned()
            .unwrap_or(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_jim_table_key_retains_publication_spelling_across_replacement() {
        // naming.jim.original-command-table-key-publication
        // docs/design/analysis/name-resolution-proofs/jim-original-command-table-key-publication.md
        let protocol = NativeNameProtocol::Jim084;
        let project = |name: &[u8]| {
            NativeJimCommandTableKey::from_projection(
                &protocol
                    .command_publication_input(super::super::NativeNameContext::root(), name)
                    .unwrap(),
            )
            .unwrap()
        };
        let first = project(b"::name");
        assert_eq!(first.report_bytes(), b"::name");
        assert_eq!(first.comparison_bytes(), b"name");
        let replaced = project(b"name").retain_for_replacement(Some(&first));
        assert_eq!(replaced, first);
        let destination = protocol
            .rename_destination_input(super::super::NativeNameContext::root(), b":::moved")
            .unwrap();
        let moved = NativeJimCommandTableKey::from_projection(&destination).unwrap();
        assert_eq!(moved.report_bytes(), b":::moved");
        assert_eq!(moved.comparison_bytes(), b"moved");
        assert_eq!(
            project(b"moved").retain_for_replacement(Some(&moved)),
            moved
        );
        assert_eq!(project(b"moved").report_bytes(), b"moved");
    }
    #[test]
    fn original_jim_table_key_rejects_lookup_and_other_engine_projections() {
        // Native proof: naming.jim.original-command-table-key-publication
        // docs/design/analysis/name-resolution-proofs/jim-original-command-table-key-publication.md
        let c = NativeNameProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_6);
        assert!(
            NativeJimCommandTableKey::from_projection(
                &c.command_publication_input(super::super::NativeNameContext::root(), b"::name")
                    .unwrap()
            )
            .is_none()
        );
        let jim = NativeNameProtocol::Jim084;
        assert!(
            NativeJimCommandTableKey::from_projection(
                &jim.command_lookup_input(super::super::NativeNameContext::root(), b"::name")
                    .unwrap()
            )
            .is_none()
        );
        assert!(NativeJimCommandTableKey::from_comparison_key(jim, b"::written").is_none());
        // This pure projection retains supplied units; public NUL/Unicode key reporting is not asserted here.
        let original = b"name\0\xff";
        let alias = jim
            .alias_publication_input(super::super::NativeNameContext::root(), original)
            .unwrap();
        let key = NativeJimCommandTableKey::from_projection(&alias).unwrap();
        assert_eq!(key.report_bytes(), original);
        assert_eq!(key.comparison_bytes(), original);
    }
}
