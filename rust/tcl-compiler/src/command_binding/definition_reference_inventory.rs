// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original definition operands survive retirement of their created class.

use super::{
    AllocationIncarnation, Arc, BTreeMap, CommandAllocation, CommandAllocationSite,
    SourceCommandBindings, SourceCommandTarget, SourceDefinitionMethodReference, SourceOriginId,
    definition_method_references::SourceDefinitionMethodReferencePhase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DefinitionMethodReferenceCoverage {
    class: SourceCommandTarget,
    creation: Option<Vec<SourceDefinitionMethodReference>>,
    observations: BTreeMap<CommandAllocationSite, Option<Vec<SourceDefinitionMethodReference>>>,
    references: Option<Vec<SourceDefinitionMethodReference>>,
}

impl DefinitionMethodReferenceCoverage {
    fn refresh(&mut self) {
        self.references = self.creation.as_ref().and_then(|creation| {
            let mut references = creation.clone();
            for observation in self.observations.values() {
                references.extend(observation.as_ref()?.iter().cloned());
            }
            Some(references)
        });
    }

    fn known_references(&self) -> impl Iterator<Item = &SourceDefinitionMethodReference> {
        self.creation.iter().flatten().chain(
            self.observations
                .values()
                .filter_map(Option::as_ref)
                .flatten(),
        )
    }
}

pub(super) type DefinitionMethodReferences =
    BTreeMap<CommandAllocation, DefinitionMethodReferenceCoverage>;

impl SourceCommandBindings {
    /// Original class allocations and their definition-name coverage.
    /// `None` is conflicting or unsupported coverage; an empty slice is a
    /// complete observation with no such operands. Neither licenses dispatch.
    pub fn definition_method_reference_inventories(
        &self,
    ) -> impl Iterator<
        Item = (
            &SourceCommandTarget,
            Option<&[SourceDefinitionMethodReference]>,
        ),
    > {
        self.definition_method_references
            .values()
            .map(|coverage| (&coverage.class, coverage.references.as_deref()))
    }

    /// Complete original operand coverage for this exact class incarnation.
    /// Later same-name classes and missing observations cannot supply it.
    #[must_use]
    pub fn definition_method_references_for_class(
        &self,
        class: &SourceCommandTarget,
    ) -> Option<&[SourceDefinitionMethodReference]> {
        let allocation = class.identity.as_ref()?.allocation.as_ref()?;
        let coverage = self.definition_method_references.get(allocation)?;
        (coverage.class == *class)
            .then_some(coverage.references.as_deref())
            .flatten()
    }

    /// One unchanged original name operand in its actual source instance.
    /// Multiple class incarnations at the same written site remain unresolved.
    #[must_use]
    pub fn definition_method_reference_at(
        &self,
        origin: &Arc<SourceOriginId>,
        operand: &crate::ir::WordExpr,
    ) -> Option<&SourceDefinitionMethodReference> {
        let mut references = self
            .definition_method_references
            .values()
            .flat_map(DefinitionMethodReferenceCoverage::known_references)
            .filter(|reference| {
                &reference.invocation().source == origin && reference.operand() == operand
            });
        let reference = references.next()?;
        references.next().is_none().then_some(reference)
    }

    pub(super) fn record_definition_method_references(
        &mut self,
        class: &SourceCommandTarget,
        phases: Option<Vec<SourceDefinitionMethodReferencePhase>>,
    ) {
        let Some(allocation) = class
            .identity
            .as_ref()
            .and_then(|id| id.allocation.as_ref())
        else {
            return;
        };
        let references = (allocation.incarnation != AllocationIncarnation::RepeatedFresh)
            .then_some(phases)
            .flatten()
            .and_then(|phases| {
                phases
                    .into_iter()
                    .map(|phase| phase.with_created_class(class))
                    .collect::<Option<Vec<_>>>()
            });
        self.definition_method_references
            .entry(allocation.clone())
            .and_modify(|retained| {
                if retained.class != *class || retained.creation != references {
                    retained.creation = None;
                }
                retained.refresh();
            })
            .or_insert_with(|| DefinitionMethodReferenceCoverage {
                class: class.clone(),
                creation: references.clone(),
                observations: BTreeMap::new(),
                references,
            });
    }

    pub(super) fn record_standalone_definition_method_references(
        &mut self,
        class: &SourceCommandTarget,
        site: CommandAllocationSite,
        phases: Option<Vec<SourceDefinitionMethodReferencePhase>>,
    ) {
        let Some(allocation) = class
            .identity
            .as_ref()
            .and_then(|id| id.allocation.as_ref())
        else {
            return;
        };
        let Some(coverage) = self.definition_method_references.get_mut(allocation) else {
            return;
        };
        let same_implementation = coverage.class.identity == class.identity
            && coverage.class.kind == class.kind
            && coverage.class.implementation_allocation == class.implementation_allocation
            && coverage.class.implementation_generation == class.implementation_generation
            && class.prepended.is_empty();
        let references = same_implementation
            .then_some(phases)
            .flatten()
            .and_then(|phases| {
                phases
                    .into_iter()
                    .map(|phase| phase.with_created_class(&coverage.class))
                    .collect()
            });
        coverage
            .observations
            .entry(site)
            .and_modify(|retained| {
                if *retained != references {
                    *retained = None;
                }
            })
            .or_insert(references);
        coverage.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> SourceCommandBindings {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = owner
            .commands()
            .profile()
            .expect("explicit Tcl 8.6 fixture");
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            owner.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
    }

    fn metadata_only(
        references: &[SourceDefinitionMethodReference],
    ) -> Vec<&SourceDefinitionMethodReference> {
        references
            .iter()
            .filter(|reference| {
                reference
                    .method_entry()
                    .is_none_or(|entry| entry.declaration() != reference.invocation())
            })
            .collect()
    }

    #[test]
    fn metadata_names_keep_their_definition_phase_and_retired_class() {
        let source = "oo::class create C {export ghost; method ghost {} {return GHOST}; method Upper {} {return FIRST}; export Upper; method Upper {} {return SECOND}; unexport Upper}; rename C retired; oo::class create C {method Upper {} {return THIRD}; export Upper}";
        let bindings = analyse(source);
        let inventories = bindings
            .definition_method_reference_inventories()
            .collect::<Vec<_>>();
        assert_eq!(inventories.len(), 2);
        let references = inventories
            .iter()
            .map(|(_, references)| references.expect("closed native metadata"))
            .collect::<Vec<_>>();
        assert_eq!(references[0].len(), 6);
        assert_eq!(references[1].len(), 2);
        let metadata = references
            .iter()
            .map(|refs| metadata_only(refs))
            .collect::<Vec<_>>();
        assert_eq!(metadata[0].len(), 3);
        assert_eq!(metadata[0][0].name(), "ghost");
        assert!(metadata[0][0].method_entry().is_none());
        for (reference, body) in [
            (metadata[0][1], "return FIRST"),
            (metadata[0][2], "return SECOND"),
            (metadata[1][0], "return THIRD"),
        ] {
            assert_eq!(
                reference
                    .method_entry()
                    .unwrap()
                    .body()
                    .text
                    .try_text()
                    .unwrap(),
                body
            );
            assert_eq!(
                reference.receiver(),
                super::super::SourceMethodReceiver::Instance
            );
            assert_eq!(
                bindings.definition_method_reference_at(
                    &reference.invocation().source,
                    reference.operand(),
                ),
                Some(reference),
            );
        }
        assert_ne!(inventories[0].0.identity, inventories[1].0.identity);
        assert_eq!(
            bindings.definition_method_references_for_class(inventories[0].0),
            Some(references[0]),
        );
    }

    #[test]
    fn method_declaration_operand_retains_its_original_private_worker() {
        let bindings = analyse("oo::class create C {method ping {} {return ORIGINAL}}");
        let (_, references) = bindings
            .definition_method_reference_inventories()
            .next()
            .unwrap();
        let references = references.expect("original native method declaration");
        assert_eq!(references.len(), 1);
        let reference = &references[0];
        let entry = reference
            .method_entry()
            .expect("the newly declared original method");
        assert_eq!(entry.declaration(), reference.invocation());
        assert_eq!(entry.name(), reference.name());
        assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
        assert_eq!(
            reference.worker().command.trim_start_matches("::"),
            "oo::define::method"
        );
        assert_eq!(
            bindings.definition_method_reference_at(
                &reference.invocation().source,
                reference.operand()
            ),
            Some(reference)
        );
    }

    #[test]
    fn metadata_coverage_keeps_empty_observations_and_declines_conflicts() {
        let mut bindings = analyse("oo::class create C {}");
        let class = bindings
            .definition_method_reference_inventories()
            .next()
            .expect("accepted actual class")
            .0
            .clone();
        assert_eq!(
            bindings.definition_method_references_for_class(&class),
            Some(&[][..])
        );
        let unchanged = bindings.clone();
        bindings.record_definition_method_references(&class, None);
        assert_ne!(bindings, unchanged);
        assert!(
            bindings
                .definition_method_references_for_class(&class)
                .is_none()
        );
        bindings.record_definition_method_references(&class, Some(Vec::new()));
        assert!(
            bindings
                .definition_method_references_for_class(&class)
                .is_none()
        );
    }

    #[test]
    fn replaced_private_metadata_worker_cannot_supply_reference_coverage() {
        let bindings = analyse(
            "proc ::oo::define::export args {return CUSTOM}; oo::class create C {method Upper {} {return OK}; export Upper}",
        );
        assert!(
            bindings
                .definition_method_reference_inventories()
                .next()
                .is_none()
        );
    }

    #[test]
    fn standalone_exports_keep_the_original_method_and_written_operand() {
        for invocation in ["oo::define C export Upper", "oo::define C {export Upper}"] {
            let source = format!(
                "oo::class create C {{method Upper {{}} {{return ORIGINAL}}}}; {invocation}"
            );
            let bindings = analyse(&source);
            let (class, references) = bindings
                .definition_method_reference_inventories()
                .next()
                .unwrap();
            let references = references.expect("original standalone worker and class");
            assert_eq!(references.len(), 2);
            let metadata = metadata_only(references);
            assert_eq!(metadata.len(), 1);
            let reference = metadata[0];
            assert_eq!(reference.name(), "Upper");
            assert_eq!(
                reference
                    .method_entry()
                    .unwrap()
                    .body()
                    .text
                    .try_text()
                    .unwrap(),
                "return ORIGINAL"
            );
            assert_eq!(reference.class().identity, class.identity);
            assert_eq!(
                bindings.definition_method_reference_at(
                    &reference.invocation().source,
                    reference.operand()
                ),
                Some(reference)
            );
        }
    }

    #[test]
    fn standalone_missing_names_do_not_borrow_later_declarations() {
        let bindings = analyse(
            "oo::class create C {method Upper {} {return ORIGINAL}}; oo::define C {export absent}",
        );
        let (_, references) = bindings
            .definition_method_reference_inventories()
            .next()
            .unwrap();
        let metadata = metadata_only(references.unwrap());
        let reference = metadata[0];
        assert_eq!(reference.name(), "absent");
        assert!(reference.method_entry().is_none());
    }

    #[test]
    fn standalone_dynamic_or_replaced_workers_withdraw_complete_coverage() {
        for invocation in [
            "set name Upper; oo::define C export $name",
            "proc ::oo::define::export args {return CUSTOM}; oo::define C {export Upper}",
            "oo::define C {unknown_definition; export Upper}",
        ] {
            let source = format!(
                "oo::class create C {{method Upper {{}} {{return ORIGINAL}}; export Upper}}; {invocation}"
            );
            let bindings = analyse(&source);
            let (_, references) = bindings
                .definition_method_reference_inventories()
                .next()
                .unwrap();
            assert!(references.is_none(), "{invocation}");
            // The independently reached creation-phase operand stays known.
            let original = bindings
                .definition_method_references
                .values()
                .next()
                .unwrap()
                .creation
                .as_ref()
                .unwrap();
            assert_eq!(original.len(), 2);
            assert_eq!(metadata_only(original).len(), 1);
            assert!(
                bindings
                    .definition_method_reference_at(
                        &original[0].invocation().source,
                        original[0].operand()
                    )
                    .is_some()
            );
        }
    }

    #[test]
    fn a_renamed_class_keeps_its_original_allocation_for_standalone_metadata() {
        let bindings = analyse(
            "oo::class create C {method Upper {} {return ORIGINAL}}; rename C saved; oo::define saved {export Upper}; oo::class create C {method Upper {} {return NEW}}",
        );
        let inventories = bindings
            .definition_method_reference_inventories()
            .collect::<Vec<_>>();
        assert_eq!(inventories.len(), 2);
        let metadata = metadata_only(inventories[0].1.unwrap());
        let reference = metadata[0];
        assert_eq!(
            reference
                .method_entry()
                .unwrap()
                .body()
                .text
                .try_text()
                .unwrap(),
            "return ORIGINAL"
        );
        assert_ne!(reference.class().identity, inventories[1].0.identity);
    }
}
