// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Separate complete-report comparison against actual issued body pairs.
use super::{ObjectFactOwner, ObjectHandleFacts};
use crate::analyser::ResolvedAnalysisInput;
use crate::analyser::authenticated_report::body_correspondence::ReportBodyCorrespondence;
use std::collections::{HashMap, HashSet};
use tcl_core_types::BodyFrameId;
use tcl_lexer::SourceImage;
use tcl_registry::RegistrySemanticKey;

fn frame_valid(map: &ReportBodyCorrespondence<'_>, frame: BodyFrameId, left: bool) -> bool {
    match frame {
        BodyFrameId::Top(id) => map.contains_module(id, left),
        BodyFrameId::Body(id) => map.contains_body(id, left),
    }
}
fn frame_pair(map: &ReportBodyCorrespondence<'_>, a: BodyFrameId, b: BodyFrameId) -> bool {
    match (a, b) {
        (BodyFrameId::Top(a), BodyFrameId::Top(b)) => map.modules(a, b),
        (BodyFrameId::Body(a), BodyFrameId::Body(b)) => map.bodies(a, b),
        _ => false,
    }
}
fn owner_valid(map: &ReportBodyCorrespondence<'_>, owner: &ObjectFactOwner, left: bool) -> bool {
    match owner {
        ObjectFactOwner::Frame(frame) => frame_valid(map, *frame, left),
        ObjectFactOwner::DefinitionScript { definition, .. } => {
            map.contains_definition(*definition, left)
        }
        ObjectFactOwner::OriginalClass { .. } => true,
    }
}
fn owner_pair(
    map: &ReportBodyCorrespondence<'_>,
    a: &ObjectFactOwner,
    b: &ObjectFactOwner,
) -> bool {
    match (a, b) {
        (ObjectFactOwner::Frame(a), ObjectFactOwner::Frame(b)) => frame_pair(map, *a, *b),
        (
            ObjectFactOwner::DefinitionScript {
                definition: a,
                receiver: ar,
            },
            ObjectFactOwner::DefinitionScript {
                definition: b,
                receiver: br,
            },
        ) => ar == br && map.bodies(*a, *b),
        (
            ObjectFactOwner::OriginalClass {
                allocation: a,
                receiver: ar,
            },
            ObjectFactOwner::OriginalClass {
                allocation: b,
                receiver: br,
            },
        ) => ar == br && a == b,
        _ => false,
    }
}
fn owner_map_equal<V: PartialEq>(
    map: &ReportBodyCorrespondence<'_>,
    a: &HashMap<ObjectFactOwner, V>,
    b: &HashMap<ObjectFactOwner, V>,
) -> bool {
    a.len() == b.len()
        && a.iter().all(|(owner, value)| {
            let mut matches = b.iter().filter(|(other, _)| owner_pair(map, owner, other));
            matches.next().is_some_and(|(_, other)| value == other) && matches.next().is_none()
        })
}
fn owner_set_equal(
    map: &ReportBodyCorrespondence<'_>,
    a: &HashSet<ObjectFactOwner>,
    b: &HashSet<ObjectFactOwner>,
) -> bool {
    a.len() == b.len()
        && a.iter().all(|owner| {
            b.iter()
                .filter(|other| owner_pair(map, owner, other))
                .count()
                == 1
        })
}

impl ObjectHandleFacts {
    fn retain_report_class_owners(
        &self,
        other: &Self,
        map: &ReportBodyCorrespondence<'_>,
        original_classes: &mut [HashSet<ObjectFactOwner>; 2],
    ) -> Option<()> {
        for (facts, left) in [(self, true), (other, false)] {
            let inventory = &mut original_classes[usize::from(!left)];
            for owner in facts.by_scope.keys().map(|(owner, _)| owner).chain(
                facts
                    .owner_spans
                    .iter()
                    .filter_map(|row| row.class_owner.as_ref()),
            ) {
                if matches!(owner, ObjectFactOwner::OriginalClass { .. }) {
                    inventory.insert(owner.clone());
                }
            }
            if facts
                .by_scope
                .keys()
                .any(|(owner, _)| !owner_valid(map, owner, left))
                || facts.owner_spans.iter().any(|row| {
                    !frame_valid(map, row.frame, left)
                        || row
                            .class_owner
                            .as_ref()
                            .is_some_and(|owner| !owner_valid(map, owner, left))
                })
                || facts
                    .returns_object
                    .keys()
                    .any(|id| !map.contains_body(*id, left))
            {
                return None;
            }
        }
        Some(())
    }

    fn reported_object_facets_equal(
        &self,
        other: &Self,
        map: &ReportBodyCorrespondence<'_>,
    ) -> (bool, bool, bool) {
        let classes_equal = self.by_scope.len() == other.by_scope.len()
            && self.by_scope.iter().all(|((owner, name), value)| {
                let mut matches =
                    other
                        .by_scope
                        .iter()
                        .filter(|((candidate, candidate_name), _)| {
                            name == candidate_name && owner_pair(map, owner, candidate)
                        });
                matches.next().is_some_and(|(_, other)| value == other) && matches.next().is_none()
            });
        let scopes_equal = self.owner_spans.len() == other.owner_spans.len()
            && self
                .owner_spans
                .iter()
                .zip(&other.owner_spans)
                .all(|(a, b)| {
                    a.span == b.span
                        && a.unit == b.unit
                        && a.class == b.class
                        && frame_pair(map, a.frame, b.frame)
                        && match (&a.class_owner, &b.class_owner) {
                            (None, None) => true,
                            (Some(a), Some(b)) => owner_pair(map, a, b),
                            _ => false,
                        }
                });
        let returns_equal = self.returns_object.len() == other.returns_object.len()
            && self.returns_object.iter().all(|(id, value)| {
                let mut matches = other
                    .returns_object
                    .iter()
                    .filter(|(other, _)| map.bodies(*id, **other));
                matches.next().is_some_and(|(_, other)| value == other) && matches.next().is_none()
            });
        (classes_equal, scopes_equal, returns_equal)
    }

    /// Ordinary Eq, old eligibility and physical result/read facets stay exact.
    pub(crate) fn authenticated_full_report_eq(
        &self,
        other: &Self,
        map: &ReportBodyCorrespondence<'_>,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
        registry: &RegistrySemanticKey,
        output: &crate::analyser::authenticated_report::ReportOutputCorrespondence<'_>,
    ) -> Option<bool> {
        // Exhaustive inventory: any new field requires a deliberate policy.
        let Self {
            report_source: _,
            logical_source_candidates: _,
            any_scope: _,
            by_scope: _,
            proven_reads: _,
            physical_type_reads: _,
            owner_spans: _,
            collections: _,
            returns_object: _,
            normal_procedure_results: _,
            global_object_cells: _,
        } = self;
        // Physical result/read facets retain their complete strict owners. A
        // distinct nested issuer is unsupported here, never a report Difference.
        if self.proven_reads != other.proven_reads
            || self.physical_type_reads != other.physical_type_reads
            || self.normal_procedure_results != other.normal_procedure_results
        {
            return None;
        }
        let mut original_classes = [HashSet::new(), HashSet::new()];
        self.retain_report_class_owners(other, map, &mut original_classes)?;
        let candidate_pair = match (
            &self.logical_source_candidates,
            &other.logical_source_candidates,
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                let (left_output, right_output) = output.analyses();
                let a = a.authenticated_report_parts(self, left_output, image, input, registry)?;
                let b =
                    b.authenticated_report_parts(other, right_output, image, input, registry)?;
                for (parts, left) in [(&a, true), (&b, false)] {
                    let inventory = &mut original_classes[usize::from(!left)];
                    for owner in parts
                        .instance_variables
                        .keys()
                        .chain(parts.original_classes.iter())
                        .chain(parts.definition_classes.iter())
                    {
                        if matches!(owner, ObjectFactOwner::OriginalClass { .. }) {
                            inventory.insert(owner.clone());
                        }
                    }
                    if parts
                        .instance_variables
                        .keys()
                        .chain(parts.original_classes.iter())
                        .chain(parts.definition_classes.iter())
                        .any(|owner| !owner_valid(map, owner, left))
                        || parts
                            .frames
                            .iter()
                            .any(|frame| !frame_valid(map, *frame, left))
                    {
                        return None;
                    }
                }
                owner_map_equal(map, a.instance_variables, b.instance_variables)
                    && owner_set_equal(map, a.original_classes, b.original_classes)
                    && owner_set_equal(map, a.definition_classes, b.definition_classes)
                    && a.frames.len() == b.frames.len()
                    && a.frames
                        .iter()
                        .all(|a| b.frames.iter().filter(|b| frame_pair(map, *a, **b)).count() == 1)
            }
            _ => return None,
        };
        // These actual allocation owners have no alternate projection. Require
        // a complete exact correspondence before reporting advisory differences.
        if original_classes[0] != original_classes[1] {
            return None;
        }
        let (classes_equal, scopes_equal, returns_equal) =
            self.reported_object_facets_equal(other, map);
        Some(
            candidate_pair
                && classes_equal
                && scopes_equal
                && returns_equal
                && self.any_scope == other.any_scope
                && self.collections == other.collections
                && self.global_object_cells == other.global_object_cells
                && self.proven_reads == other.proven_reads
                && self.physical_type_reads == other.physical_type_reads
                && self.normal_procedure_results == other.normal_procedure_results,
        )
    }
}
