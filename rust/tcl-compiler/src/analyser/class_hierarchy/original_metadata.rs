// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Class relation order for retained source declaration advice.
//!
//! This view joins exact declaration publications before the shared MRO walk.
//! It supplies no call-point table, registered ancestor, maker or native
//! delegate attachment. Missing source providers and ambiguous joins abstain.

use crate::analyser::types::{MemberSide, OriginalSourceClassRelationKind};
use crate::analyser::{AnalysisResult, ClassDef};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use std::collections::HashMap;

/// Complete source-declaration instance relation order for one retained class.
/// Every reachable superclass and mixin must resolve to one exact source
/// record. A registered or external provider has no source node here, so its
/// absence withdraws this advisory order rather than supplying a native root.
/// Class-object makers, object mixins and delegate attachments are separate.
#[must_use]
pub fn original_instance_metadata_order<'a>(
    analysis: &'a AnalysisResult,
    root: &SourceDeclarationMetadata<ClassDef>,
) -> Option<Vec<&'a SourceDeclarationMetadata<ClassDef>>> {
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let mut roots = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.declaration_site() == root.declaration_site());
    let (root_index, record) = roots.next()?;
    if roots.next().is_some() || **record != *root {
        return None;
    }
    let occupied = analysis
        .original_procedure_declarations()
        .map(crate::signature_scan::original_name::SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let order = original_instance_metadata_order_for_inventory(&records, root_index, &occupied)?;
    Some(order.into_iter().map(|index| records[index]).collect())
}

/// Source-declaration relation order in a caller-retained record inventory.
/// Indices preserve independent document owners even when their original
/// declaration sites or images compare equal. Provider joins must still select
/// exactly one publication; equal-byte records from distinct owners are not
/// collapsed. This supplies declaration advice, not dispatch or publication.
#[must_use]
pub fn original_instance_metadata_order_for_records(
    records: &[&SourceDeclarationMetadata<ClassDef>],
    root_index: usize,
) -> Option<Vec<usize>> {
    original_instance_metadata_order_for_inventory(records, root_index, &[])
}

/// Direct source relation targets selected at the first occupied publication.
/// Non-class declarations block a later class candidate. Independent records
/// with equal publications remain ambiguous. This is declaration advice only.
#[must_use]
pub fn original_direct_metadata_relations_for_records(
    records: &[&SourceDeclarationMetadata<ClassDef>],
    root_index: usize,
    kind: OriginalSourceClassRelationKind,
    occupied_non_classes: &[&crate::signature_scan::scope::SignatureSourceCommand],
) -> Option<Vec<usize>> {
    records
        .get(root_index)?
        .metadata()
        .original_relations
        .resolve(MemberSide::Instance, kind, |relation| {
            let selected = relation.lookup().first_matching_publications(
                records
                    .iter()
                    .enumerate()
                    .map(|(index, record)| (record.name(), Some(index)))
                    .chain(occupied_non_classes.iter().map(|name| (*name, None))),
            );
            match selected.as_slice() {
                [Some(index)] => Some(*index),
                _ => None,
            }
        })
}

/// Source instance order with independently retained occupied command headers.
/// Every relation uses the same first-publication rule before the shared MRO
/// walk. No registered hierarchy or current method dispatch is inferred.
#[must_use]
pub fn original_instance_metadata_order_for_inventory(
    records: &[&SourceDeclarationMetadata<ClassDef>],
    root_index: usize,
    occupied_non_classes: &[&crate::signature_scan::scope::SignatureSourceCommand],
) -> Option<Vec<usize>> {
    records.get(root_index)?;
    let mut supers = HashMap::new();
    let mut mixins = HashMap::new();
    let mut pending = vec![root_index];
    while let Some(index) = pending.pop() {
        if supers.contains_key(&index) {
            continue;
        }
        let parents = original_direct_metadata_relations_for_records(
            records,
            index,
            OriginalSourceClassRelationKind::Superclass,
            occupied_non_classes,
        )?;
        let mixed = original_direct_metadata_relations_for_records(
            records,
            index,
            OriginalSourceClassRelationKind::Mixin,
            occupied_non_classes,
        )?;
        pending.extend(parents.iter().chain(&mixed).copied());
        supers.insert(index, parents);
        mixins.insert(index, mixed);
    }
    tcl_syntax::mro::tcloo_linearise(&root_index, &supers, &mixins).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn original_metadata_order_joins_opaque_parents_after_ui_maps_are_cleared() {
        let mut analysis = Analyser::new().analyse(
            "oo::class create B\\uD800 {}\noo::class create B\\uD801 {}\noo::class create C {superclass B\\uD800}\n",
            "tcl8.6").clone();
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        let root = analysis
            .original_class_declarations()
            .find(|record| record.name_input().bytes() == b"C")
            .unwrap();
        let order = original_instance_metadata_order(&analysis, root).unwrap();
        assert_eq!(
            order
                .iter()
                .map(|record| record.name_input().bytes())
                .collect::<Vec<_>>(),
            vec![b"C".as_slice(), b"B\xed\xa0\x80".as_slice()]
        );
    }

    #[test]
    fn original_metadata_order_refuses_missing_computed_and_cyclic_providers() {
        for source in [
            "oo::class create C {superclass Missing}",
            "oo::class create C {superclass $unknown}",
            "oo::class create B {superclass C}\noo::class create C {superclass B}",
            "oo::class create C {superclass ::oo::object}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
            let root = analysis
                .original_class_declarations()
                .find(|record| record.name_input().bytes() == b"C")
                .unwrap();
            assert!(
                original_instance_metadata_order(&analysis, root).is_none(),
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod record_owner_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn original_metadata_record_order_preserves_equal_source_owner_indices() {
        let first = Analyser::new().analyse("oo::class create C {}", "tcl8.6");
        let second = Analyser::new().analyse("oo::class create C {}", "tcl8.6");
        let left = first.original_class_declarations().next().unwrap();
        let right = second.original_class_declarations().next().unwrap();
        assert_eq!(left.declaration_site(), right.declaration_site());
        let records = [left, right];
        assert_eq!(
            original_instance_metadata_order_for_records(&records, 0),
            Some(vec![0])
        );
        assert_eq!(
            original_instance_metadata_order_for_records(&records, 1),
            Some(vec![1])
        );
        assert!(original_instance_metadata_order_for_records(&records, 2).is_none());
    }

    #[test]
    fn original_metadata_record_order_refuses_duplicate_provider_publications() {
        let source = "oo::class create Base {}\noo::class create Child {superclass Base}\n";
        let first = Analyser::new().analyse(source, "tcl8.6");
        let second = Analyser::new().analyse(source, "tcl8.6");
        let base = first
            .original_class_declarations()
            .find(|record| record.name_input().bytes() == b"Base")
            .unwrap();
        let duplicate = second
            .original_class_declarations()
            .find(|record| record.name_input().bytes() == b"Base")
            .unwrap();
        let child = first
            .original_class_declarations()
            .find(|record| record.name_input().bytes() == b"Child")
            .unwrap();
        assert_eq!(
            original_instance_metadata_order_for_records(&[base, child], 1),
            Some(vec![1, 0])
        );
        assert!(
            original_instance_metadata_order_for_records(&[base, duplicate, child], 2).is_none()
        );
    }
}
