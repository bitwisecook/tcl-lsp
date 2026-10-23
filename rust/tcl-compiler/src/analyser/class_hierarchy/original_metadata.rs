// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Class relation order for retained source declaration advice.
//!
//! This view joins exact declaration publications before the shared MRO walk.
//! Pure declaration orders supply no registered ancestor. A separate provider
//! view retains actual parent-horizon Registry roots. Neither supplies a
//! call-point table, maker or native delegate attachment. Missing providers and
//! ambiguous joins abstain.

mod source_method_reference;

pub use source_method_reference::{
    OriginalSourceSelfMethodReference, OriginalSourceSelfMethodReferenceObligation,
    OriginalSourceSelfPropertyReference, OriginalSourceSelfPropertyReferenceObligation,
    original_source_self_method_references, original_source_self_property_references,
};

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

/// Original own-table methods associated with this actual fresh definition.
/// Missing parent geometry and source-installer applicability remain terminal;
/// no displayed method map selects a member or repairs an unrepresented owner.
#[must_use]
pub(crate) fn original_definition_members_for_class(
    analysis: &AnalysisResult,
    class: &ClassDef,
    side: MemberSide,
) -> Option<Vec<crate::analyser::types::OriginalSourceMethodMetadata>> {
    let parents = class.original_relations.resolve(
        MemberSide::Instance,
        OriginalSourceClassRelationKind::Superclass,
        |relation| {
            let provider = relation.source_provider()?;
            (provider.matches_analysis(analysis)
                && provider.matches_relation(relation.name_input(), relation.lookup())
                && provider.matches_class_definition(class))
            .then(|| provider.clone())
        },
    )?;
    // This view is for the original fresh definition only. A later
    // configuration cannot alter an earlier factory through its final ledger.
    for method in class
        .original_members
        .declarations()
        .filter(|method| method.side() == side)
    {
        let parent = method.definition_parent()?;
        if method.declaration().source_installer().is_some()
            || !parents
                .iter()
                .any(|provider| provider.matches_definition_parent(parent))
        {
            return None;
        }
    }
    for effect in class
        .original_members
        .effects()
        .filter(|effect| effect.side() == side)
    {
        if !parents
            .iter()
            .any(|provider| provider.contains_definition_site(effect.site()))
        {
            return None;
        }
    }
    let methods = class.original_members.methods(side)?;
    Some(methods)
}

/// Registry metaclass metadata reached through closed original superclass
/// effects and actual parent-horizon providers. This does not select methods,
/// execute a constructor or prove normal completion.
#[must_use]
pub(crate) fn original_registry_metaclass_provider(
    analysis: &AnalysisResult,
    root: &ClassDef,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<crate::command_binding::OriginalSourceClassRelationProvider> {
    let mut pending = vec![(root, false)];
    let mut visited: Vec<(&ClassDef, bool)> = Vec::new();
    let mut selected = None;
    while let Some((class, finished)) = pending.pop() {
        if let Some((_, complete)) = visited
            .iter_mut()
            .find(|(previous, _)| std::ptr::eq(*previous, class))
        {
            if finished {
                *complete = true;
            } else if !*complete {
                return None;
            }
            continue;
        }
        visited.push((class, false));
        pending.push((class, true));
        let parents = class.original_relations.resolve(
            MemberSide::Instance,
            OriginalSourceClassRelationKind::Superclass,
            |relation| {
                let provider = relation.source_provider()?;
                (provider.matches_analysis(analysis)
                    && provider.matches_relation(relation.name_input(), relation.lookup())
                    && provider.matches_class_definition(class))
                .then(|| provider.clone())
            },
        )?;
        for provider in parents {
            if provider.registry_descriptor(context).is_some() {
                if selected.as_ref().is_some_and(
                    |prior: &crate::command_binding::OriginalSourceClassRelationProvider| {
                        prior.policy() != provider.policy()
                            || prior.source_slot() != provider.source_slot()
                            || !prior
                                .registry_descriptor(context)
                                .zip(provider.registry_descriptor(context))
                                .is_some_and(|(left, right)| std::ptr::eq(left, right))
                    },
                ) {
                    return None;
                }
                selected = Some(provider);
            } else {
                let declaration = provider.source_class()?;
                let parent = if analysis.allows_retained_logical_declaration_advice() {
                    declaration.logical_source_class(analysis)?
                } else {
                    declaration.source_class(analysis)?.metadata()
                };
                pending.push((parent, false));
            }
        }
    }
    selected
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

#[cfg(test)]
mod registry_provider_tests {
    use super::*;
    use crate::analyser::{Analyser, MetaclassProvenance};

    #[test]
    fn original_registry_metaclass_uses_current_relation_and_definition_owners() {
        let analysis = Analyser::new()
            .analyse("oo::class create Meta {superclass ::oo::class}", "tcl8.6")
            .clone();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let class = analysis
            .all_classes
            .values()
            .find(|class| class.name == "Meta")
            .unwrap();
        let provider = original_registry_metaclass_provider(&analysis, class, &context).unwrap();
        assert_eq!(
            provider.registry_descriptor(&context).unwrap().name,
            "oo::class"
        );

        let mut reporting_only = class.clone();
        reporting_only.name = "unrelated label".to_owned();
        reporting_only.qualified_name = "::unrelated label".to_owned();
        reporting_only.superclasses.clear();
        reporting_only.relation_lookups.clear();
        assert!(
            original_registry_metaclass_provider(&analysis, &reporting_only, &context).is_some()
        );

        let foreign = Analyser::new()
            .analyse("oo::class create Other {superclass ::oo::class}", "tcl8.6")
            .clone();
        assert!(original_registry_metaclass_provider(&foreign, class, &context).is_none());

        let mut missing_owner = class.clone();
        missing_owner.original_relations = Default::default();
        assert!(
            original_registry_metaclass_provider(&analysis, &missing_owner, &context).is_none()
        );
        let mut changed_definition = class.clone();
        changed_definition.body_span = tcl_lexer::Span::new(0, 0);
        assert!(
            original_registry_metaclass_provider(&analysis, &changed_definition, &context)
                .is_none()
        );
        let mut ambiguous = class.clone();
        ambiguous.source_name_ambiguous.observe_if(true);
        assert!(original_registry_metaclass_provider(&analysis, &ambiguous, &context).is_none());
        let mut stand_in = class.clone();
        stand_in.metaclass_provenance = MetaclassProvenance::StandIn;
        assert!(original_registry_metaclass_provider(&analysis, &stand_in, &context).is_none());
    }

    #[test]
    fn original_factory_members_require_their_actual_class_body_parent() {
        let source = "oo::class create Meta {superclass ::oo::class; self method create {name body} {next $name $body}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let class = analysis
            .all_classes
            .values()
            .find(|class| class.name == "Meta")
            .unwrap();
        let members =
            original_definition_members_for_class(&analysis, class, MemberSide::ClassObject)
                .unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].original_name_input().bytes(), b"create");
        let signature = members[0]
            .original_parameter_names_and_body(&analysis)
            .unwrap();
        assert_eq!(
            signature.parameter_names().collect::<Vec<_>>(),
            ["name", "body"]
        );

        let mut report_only = class.clone();
        report_only.class_methods.clear();
        report_only.class_exports.clear();
        report_only.class_unexports.clear();
        assert_eq!(
            original_definition_members_for_class(&analysis, &report_only, MemberSide::ClassObject)
                .unwrap(),
            members
        );

        let mut missing_parent = class.clone();
        missing_parent.original_members = Default::default();
        missing_parent
            .original_members
            .declare(members[0].clone().with_definition_parent(None).unwrap());
        assert!(
            original_definition_members_for_class(
                &analysis,
                &missing_parent,
                MemberSide::ClassObject
            )
            .is_none()
        );
        let mut outside_effect = class.clone();
        let mut site = members[0].declaration().site().clone();
        site.offset = class.name_span.start();
        outside_effect.original_members.effect(
            crate::analyser::types::OriginalSourceMemberEffect::new(
                site,
                MemberSide::ClassObject,
                crate::analyser::types::OriginalSourceMemberEffectKind::Visibility(false),
                vec![members[0].original_name_input().clone()],
                members[0].name_purpose(),
            )
            .unwrap(),
        );
        assert!(
            original_definition_members_for_class(
                &analysis,
                &outside_effect,
                MemberSide::ClassObject
            )
            .is_none()
        );

        let foreign = Analyser::new().analyse("oo::class create Other {superclass ::oo::class; self method create {x script} {next $x $script}}", "tcl8.6").clone();
        let foreign_class = foreign
            .all_classes
            .values()
            .find(|class| class.name == "Other")
            .unwrap();
        let mut foreign_parent = class.clone();
        foreign_parent.original_members = foreign_class.original_members.clone();
        assert!(
            original_definition_members_for_class(
                &analysis,
                &foreign_parent,
                MemberSide::ClassObject
            )
            .is_none()
        );
        assert!(
            members[0]
                .original_parameter_names_and_body(&foreign)
                .is_none()
        );
    }

    #[test]
    fn original_registry_metaclass_does_not_bypass_unknown_member_effects() {
        for source in [
            "oo::class create Meta {mystery; superclass ::oo::class}",
            "oo::class create Meta {superclass ::oo::class; mystery}",
            "oo::class create Meta {superclass $unknown}",
            "oo::class create Meta {superclass Missing}",
            "oo::class create Meta {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let class = analysis
                .all_classes
                .values()
                .find(|class| class.name == "Meta")
                .unwrap();
            assert!(
                original_registry_metaclass_provider(&analysis, class, &context).is_none(),
                "{source}"
            );
        }
    }
}

/// Original class/member inventory for conditional instance-side display.
/// Native and positively retained Logical member owners remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInstanceClassMetadata {
    record: SourceDeclarationMetadata<ClassDef>,
    metadata: ClassDef,
    input: crate::analyser::ResolvedAnalysisInput,
    realm: std::sync::Arc<crate::realm::CommandBindingRealm>,
    logical: Option<crate::analyser::types::LogicalSourceClassDeclaration>,
    occupied_non_classes: Vec<crate::signature_scan::scope::SignatureSourceCommand>,
}
impl OriginalInstanceClassMetadata {
    /// Capture the actual factory declaration and its retained class-source owner.
    #[must_use]
    pub fn capture(
        analysis: &AnalysisResult,
        factory: &crate::command_binding::OriginalSourceClassDeclaration,
    ) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let mut records = analysis
            .original_class_declarations()
            .filter(|record| factory.matches_source_declaration(record));
        let record = records.next()?;
        records.next().is_none().then_some(())?;
        let metadata = if input.has_logical_source_name_context() {
            factory.logical_source_class(analysis)?
        } else {
            factory.source_class(analysis)?.metadata()
        };
        let logical = if input.has_logical_source_name_context() {
            Some(analysis.logical_source_class_declaration(metadata)?.clone())
        } else {
            None
        };
        Some(Self {
            record: record.clone(),
            metadata: metadata.clone(),
            input: input.clone(),
            realm: std::sync::Arc::clone(analysis.retained_command_realm_arc()?),
            logical,
            occupied_non_classes: analysis
                .original_procedure_declarations()
                .map(|record| record.name().clone())
                .collect(),
        })
    }
    /// The retained original class metadata for readonly display.
    #[must_use]
    pub const fn metadata(&self) -> &ClassDef {
        &self.metadata
    }
    fn current(&self) -> bool {
        self.realm.matches_resolved_analysis_input(&self.input)
            && self.realm.matches_original_source_image(
                self.record.name_input().source_image(),
                self.input.lexer_config(),
            )
            && self.logical.as_ref().is_none_or(|logical| {
                logical.matches_source(self.record.name_input().source_image(), &self.input)
            })
    }
    fn own_method(
        &self,
        selector: &crate::command_binding::SourceAdviceNameInput,
    ) -> Option<Option<OriginalInstanceMethodDisplay>> {
        self.current().then_some(())?;
        let methods = self
            .metadata
            .original_members
            .methods(MemberSide::Instance)?;
        let mut matches = Vec::new();
        if let Some(logical) = &self.logical {
            selector.logical_input()?;
            for method in self.metadata.methods.values() {
                let word = self.metadata.original_members.logical_name_operand(
                    MemberSide::Instance,
                    method,
                    &self.input,
                    self.record.name_input().source_image(),
                )?;
                if tcl_syntax::word_rules::original_static_word_unicode_value(word)?.as_slice()
                    == selector.bytes()
                {
                    matches.push(OriginalInstanceMethodDisplay {
                        metadata: method.clone(),
                        native: None,
                        logical: Some(logical.member(MemberSide::Instance, method)?),
                    });
                }
            }
        } else {
            let input = selector.native_input()?;
            for method in methods {
                if method.original_name_input().policy() == input.policy()
                    && method.original_name_input().bytes() == input.bytes()
                {
                    matches.push(OriginalInstanceMethodDisplay {
                        metadata: method.metadata().clone(),
                        native: Some(method),
                        logical: None,
                    });
                }
            }
        }
        match matches.len() {
            0 => Some(None),
            1 => Some(matches.pop()),
            _ => None,
        }
    }
    fn inherited_visibility(
        &self,
        selector: &crate::command_binding::SourceAdviceNameInput,
        exported: bool,
    ) -> Option<bool> {
        self.current().then_some(())?;
        if self.logical.is_some() {
            selector.logical_input()?;
            self.metadata
                .original_members
                .methods(MemberSide::Instance)?;
            let name = std::str::from_utf8(selector.bytes()).ok()?;
            Some(
                self.metadata
                    .original_members
                    .logical_visibility_name(
                        name,
                        &self.input,
                        self.record.name_input().source_image(),
                    )
                    .map_or(exported, |visibility| {
                        visibility == tcl_registry::definer::MemberVisibility::Exported
                    }),
            )
        } else {
            self.metadata
                .original_members
                .inherited_visibility_for_input(
                    MemberSide::Instance,
                    selector.native_input()?,
                    exported,
                )
        }
    }
}

/// Exact original member selected for display, without dispatch or activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInstanceMethodDisplay {
    metadata: crate::analyser::MethodDef,
    native: Option<crate::analyser::types::OriginalSourceMethodMetadata>,
    logical: Option<crate::analyser::types::LogicalSourceMemberDeclaration>,
}
impl OriginalInstanceMethodDisplay {
    /// The exact original member metadata selected for display.
    #[must_use]
    pub const fn metadata(&self) -> &crate::analyser::MethodDef {
        &self.metadata
    }
}

/// Complete represented instance-side source order and branch visibility.
/// `Some(None)` means no represented member in this source inventory; it is
/// neither interpreter absence nor permission to restore a report-map method.
#[must_use]
pub fn original_instance_method_display(
    owners: &[OriginalInstanceClassMetadata],
    root: usize,
    selector: &crate::command_binding::SourceAdviceNameInput,
) -> Option<Option<OriginalInstanceMethodDisplay>> {
    if owners.iter().any(|owner| !owner.current()) {
        return None;
    }
    let records = owners.iter().map(|owner| &owner.record).collect::<Vec<_>>();
    let occupied = owners
        .iter()
        .flat_map(|owner| &owner.occupied_non_classes)
        .collect::<Vec<_>>();
    let order = original_instance_metadata_order_for_inventory(&records, root, &occupied)?;
    let mut supers = HashMap::new();
    let mut branches = vec![root];
    for &index in &order {
        supers.insert(
            index,
            original_direct_metadata_relations_for_records(
                &records,
                index,
                OriginalSourceClassRelationKind::Superclass,
                &occupied,
            )?,
        );
        branches.extend(original_direct_metadata_relations_for_records(
            &records,
            index,
            OriginalSourceClassRelationKind::Mixin,
            &occupied,
        )?);
    }
    branches.sort_by_key(|branch| order.iter().position(|index| index == branch));
    branches.dedup();
    let branches = branches
        .into_iter()
        .map(|branch| tcl_syntax::mro::tcloo_linearise(&branch, &supers, &HashMap::new()).ok())
        .collect::<Option<Vec<_>>>()?;
    for index in order {
        let Some(method) = owners[index].own_method(selector)? else {
            continue;
        };
        if method.metadata.visibility == "private" {
            return None;
        }
        let mut exported = method.native.as_ref().map_or(
            method.metadata.visibility == "public",
            crate::analyser::types::OriginalSourceMethodMetadata::exported,
        );
        let branch = branches.iter().find(|branch| branch.contains(&index))?;
        let position = branch.iter().position(|member| *member == index)?;
        for &ancestor in branch[..position].iter().rev() {
            exported = owners[ancestor].inherited_visibility(selector, exported)?;
        }
        return exported.then_some(Some(method));
    }
    // The source family may supply a builtin/generated member, but known
    // visibility effects still precede that independent descriptor projection.
    for branch in &branches {
        let mut visible = true;
        for &ancestor in branch.iter().rev() {
            visible = owners[ancestor].inherited_visibility(selector, visible)?;
        }
        if !visible {
            return None;
        }
    }
    Some(None)
}
