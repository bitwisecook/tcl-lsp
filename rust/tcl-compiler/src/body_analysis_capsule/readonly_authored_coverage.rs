// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly authored body units and the Installed lowerings they carry.
//!
//! C Tcl creates one command per evaluated `proc`. When a readonly authored
//! declaration body uniquely covers the Installed body of the same first
//! allocation, the two are lowerings of one analysis unit: the authored unit
//! stays in the body-unit stores and carries the Installed lowering, which
//! keeps its own body ID, function, prepared CFG and Source SSA summary.

use super::{CompilationUnit, ProcedureId};
use crate::command_binding::{
    CommandAllocation, CommandAllocationSite, RetainedSourceModuleBindings,
    SourceInstalledProcedureBody,
};
use crate::compilation_unit::FunctionUnit;
use crate::ir::{Module, Procedure, ProcedureBodyKind};
use std::collections::{BTreeMap, BTreeSet};
use tcl_registry::CommandRegistry;

/// A source coverage relation, never an installed implementation selection.
pub(super) struct ReadonlyAuthoredCoverage {
    installed_sources: BTreeMap<ProcedureId, ProcedureId>,
}

/// Which body lowerings a relation must account for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Inventory {
    /// Every body lowering is a readonly authored body or a covered Installed one.
    Complete,
    /// Other bodies remain separate units; only exact unique pairs are kept.
    Pairs,
}

/// Module-wide inputs computed once per relation, so classifying each
/// lowering costs no scan of the other bodies.
struct RelationInputs<'a> {
    retained: &'a RetainedSourceModuleBindings,
    /// Original source procedure bodies, as `original_source_procedure_bodies`.
    original: BTreeSet<ProcedureId>,
    /// Final installed source rows by full allocation.
    rows: BTreeMap<&'a CommandAllocation, Vec<&'a SourceInstalledProcedureBody>>,
    /// Readonly authored cards by declaration site.
    authored: BTreeMap<&'a CommandAllocationSite, Vec<ProcedureId>>,
}

/// What one body lowering is to the relation.
enum Lowering {
    Authored,
    Installed(ProcedureId),
    Other,
}

impl ReadonlyAuthoredCoverage {
    /// Complete readonly coverage of every body lowering, carried or not.
    pub(super) fn capture(unit: &CompilationUnit, registry: &CommandRegistry) -> Option<Self> {
        Self::relate(unit, registry, Inventory::Complete)
    }

    /// The exact pairs a build carries. Only a unit that carries nothing yet
    /// is paired; other body units keep their own analyses.
    pub(super) fn pairs(unit: &CompilationUnit, registry: &CommandRegistry) -> Option<Self> {
        let module = &unit.ir_module;
        (module.carried_installed_lowerings.is_empty()
            && !module.installed_procedure_body_units.is_empty()
            && !module.original_authored_declaration_body_units.is_empty())
        .then_some(())?;
        Self::relate(unit, registry, Inventory::Pairs)
    }

    pub(super) fn source_for(&self, installed: ProcedureId) -> Option<ProcedureId> {
        self.installed_sources.get(&installed).copied()
    }

    /// Move each paired Installed lowering and its function out of the
    /// body-unit stores and into its authored unit, under its own body ID.
    pub(super) fn carry(self, unit: &mut CompilationUnit) {
        for (installed, authored) in self.installed_sources {
            if !unit.ir_module.body_units.contains_key(&authored)
                || !unit.body_units.contains_key(&authored)
                || !unit.ir_module.body_units.contains_key(&installed)
                || !unit.body_units.contains_key(&installed)
            {
                continue;
            }
            let (Some(procedure), Some(function)) = (
                unit.ir_module.body_units.remove(&installed),
                unit.body_units.remove(&installed),
            ) else {
                continue;
            };
            unit.ir_module
                .carried_installed_lowerings
                .insert(installed, authored, procedure);
            unit.carried_installed_lowerings.insert(installed, function);
        }
    }

    fn relate(
        unit: &CompilationUnit,
        registry: &CommandRegistry,
        inventory: Inventory,
    ) -> Option<Self> {
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_deref()?;
        let rows = retained
            .positioned_for_module(module, registry)?
            .installed_procedure_body_units();
        if unit.source.as_bytes() != module.source.bytes()
            || module.original_authored_declaration_body_units.is_empty()
            || unit.body_units.len() != module.body_units.len()
            || unit.carried_installed_lowerings.len() != module.carried_installed_lowerings.len()
            || module
                .original_authored_declaration_body_units
                .keys()
                .chain(module.installed_procedure_body_units.keys())
                .any(|id| unit.body_lowering_function(*id).is_none())
        {
            return None;
        }
        let mut by_allocation: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for row in &rows {
            by_allocation.entry(&row.allocation).or_default().push(row);
        }
        let mut by_site: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (&id, card) in &module.original_authored_declaration_body_units {
            by_site.entry(card.site()).or_default().push(id);
        }
        let inputs = RelationInputs {
            retained,
            original: module
                .original_source_procedure_bodies()
                .map(|(&id, _)| id)
                .collect(),
            rows: by_allocation,
            authored: by_site,
        };
        let mut authored = BTreeSet::new();
        let mut installed_sources = BTreeMap::new();
        let mut sources = BTreeSet::new();
        let mut shared = BTreeSet::new();
        for (&id, function) in unit.body_lowering_functions() {
            match classify_lowering(unit, &inputs, id, function) {
                Some(Lowering::Authored) => {
                    authored.insert(id);
                }
                Some(Lowering::Installed(source)) => {
                    if !sources.insert(source) {
                        shared.insert(source);
                    }
                    installed_sources.insert(id, source);
                }
                Some(Lowering::Other) | None if inventory == Inventory::Pairs => {}
                Some(Lowering::Other) | None => return None,
            }
        }
        if inventory == Inventory::Complete && !shared.is_empty() {
            return None;
        }
        installed_sources
            .retain(|_, &mut source| authored.contains(&source) && !shared.contains(&source));
        if inventory == Inventory::Pairs {
            // Fail closed: carry only the sole Installed body at its declaration
            // site. Any other Installed body there, admitted or refused, could
            // be another lowering of the same evaluated `proc`.
            let mut at_site: BTreeMap<&CommandAllocationSite, usize> = BTreeMap::new();
            for allocation in module.installed_procedure_body_units.values() {
                *at_site.entry(&allocation.site).or_default() += 1;
            }
            installed_sources.retain(|installed, _| {
                module
                    .installed_procedure_body_units
                    .get(installed)
                    .is_some_and(|allocation| at_site.get(&allocation.site) == Some(&1))
            });
        }
        Some(Self { installed_sources })
    }
}

/// Classify one body lowering, or refuse it when its stores, prepared CFG,
/// retained summary or source row disagree.
fn classify_lowering(
    unit: &CompilationUnit,
    inputs: &RelationInputs<'_>,
    id: ProcedureId,
    function: &FunctionUnit,
) -> Option<Lowering> {
    let module = &unit.ir_module;
    let retained = inputs.retained;
    let procedure = module.body_lowering(&id)?;
    if !module.procedure_ids.owns(id)
        || !std::ptr::eq(unit.function_for_body(Some(id))?, function)
        || function.source_metadata_input() != module.source_metadata_input.as_ref()
        || function.body_id != Some(id)
    {
        return None;
    }
    if module
        .original_native_declaration_body_units
        .contains_key(&id)
        || module
            .original_logical_declaration_body_units
            .contains_key(&id)
        || module
            .original_hosted_declaration_body_units
            .contains_key(&id)
        || module.original_declaration_body_units.contains_key(&id)
    {
        return Some(Lowering::Other);
    }
    if let Some(card) = module.original_authored_declaration_body_units.get(&id) {
        return (!module.installed_procedure_body_units.contains_key(&id)
            && module.body_units.contains_key(&id)
            && unit.original_procedure_cfg_in(id, &inputs.original) == Some(&function.cfg)
            && retained.matches_original_procedure_summary(id, procedure)
            && card.matches_procedure(module, procedure))
        .then_some(Lowering::Authored);
    }
    let Some(allocation) = module.installed_procedure_body_units.get(&id) else {
        return Some(Lowering::Other);
    };
    // Installed lowerings retain their original full construction config;
    // only declaration units enter the separate rebased memo path.
    if function.source_lexer_config() != module.lexer_config
        || unit.installed_procedure_cfg_for_body(id) != Some(&function.cfg)
        || !retained.matches_installed_procedure_summary(id, procedure)
    {
        return None;
    }
    let [record] = inputs.rows.get(allocation)?.as_slice() else {
        return None;
    };
    if !matches_installed_row(procedure, record) {
        return None;
    }
    let source = unique_authored_source(module, inputs, record)?;
    if module
        .carried_installed_lowerings
        .unit_of(&id)
        .is_some_and(|carrier| carrier != source)
    {
        return None;
    }
    Some(Lowering::Installed(source))
}

fn matches_installed_row(procedure: &Procedure, installed: &SourceInstalledProcedureBody) -> bool {
    let Some(end) = u32::try_from(installed.source.text.len())
        .ok()
        .and_then(|length| installed.source.base().checked_add(length))
    else {
        return false;
    };
    let parameters = tcl_syntax::list::join_list(installed.parameters.iter().map(|parameter| {
        tcl_syntax::list::join_list(
            std::iter::once(parameter.name.as_str()).chain(parameter.default.as_deref()),
        )
    }));
    procedure.body_kind == ProcedureBodyKind::Installed
        && procedure.publication.is_none()
        && procedure.declaration_site.as_ref() == Some(&installed.allocation.site)
        && procedure.span == tcl_lexer::Span::new(installed.source.base(), end)
        && procedure.body_offset == installed.source.base()
        && procedure.body_source.as_deref() == installed.source.try_text().ok()
        && procedure.body.executed_source.as_deref() == Some(&installed.source)
        && procedure.body.namespace_context.as_deref() == Some(&installed.namespace_key)
        && procedure.params_raw == parameters
        && procedure.params.iter().map(String::as_str).eq(installed
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str()))
}

fn unique_authored_source(
    module: &Module,
    inputs: &RelationInputs<'_>,
    installed: &SourceInstalledProcedureBody,
) -> Option<ProcedureId> {
    // Coverage requires the card's own allocation site, so only the cards
    // declared at that site can cover this row.
    let mut candidates = inputs
        .authored
        .get(&installed.allocation.site)?
        .iter()
        .filter_map(|&id| {
            let card = module.original_authored_declaration_body_units.get(&id)?;
            let procedure = module.body_units.get(&id)?;
            (module.procedure_ids.owns(id)
                && inputs
                    .retained
                    .matches_original_procedure_summary(id, procedure)
                && card.covers_installed_analysis_source(module, procedure, installed))
            .then_some(id)
        });
    let selected = candidates.next()?;
    candidates.next().is_none().then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        CompilationUnit,
    ) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let unit = CompilationUnit::build_with_options(
            "proc kept {x} {set value $x; return $value}\nproc other {} {return OTHER}\n",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
        );
        (context, unit)
    }

    #[test]
    fn readonly_authored_coverage_preserves_both_actual_analysis_inventories() {
        let (context, unit) = fixture();
        let module = &unit.ir_module;
        assert_eq!(module.original_authored_declaration_body_units.len(), 2);
        assert_eq!(module.installed_procedure_body_units.len(), 2);
        // One analysis unit per evaluated `proc`, each carrying its Installed
        // lowering: four lowerings, two units.
        assert_eq!(module.body_units.len(), 2);
        assert_eq!(module.carried_installed_lowerings.len(), 2);
        assert_eq!(module.body_lowerings().count(), 4);
        assert_eq!(unit.body_units.len(), 2);
        assert_eq!(unit.carried_installed_lowerings.len(), 2);
        assert_eq!(unit.body_lowering_functions().count(), 4);
        let coverage = ReadonlyAuthoredCoverage::capture(&unit, context.commands()).unwrap();
        for &installed in module.installed_procedure_body_units.keys() {
            let source = coverage.source_for(installed).unwrap();
            assert_ne!(source, installed);
            assert!(
                module
                    .original_authored_declaration_body_units
                    .contains_key(&source)
            );
            assert_eq!(
                module.carried_installed_lowerings.unit_of(&installed),
                Some(source)
            );
            assert!(!module.body_units.contains_key(&installed));
            assert!(module.body_units[&source].body.executed_source.is_none());
            assert!(
                module
                    .body_lowering(&installed)
                    .unwrap()
                    .body
                    .executed_source
                    .is_some()
            );
        }
        assert!(module.source_metadata_input.is_none());
        assert!(module.source_entry.native_entry.is_none());
        assert_eq!(module.installed_procedure_body_units.len(), 2);
        assert_eq!(module.original_authored_declaration_body_units.len(), 2);
    }

    #[test]
    fn readonly_authored_coverage_keeps_installed_function_input_and_unique_store() {
        let (context, unit) = fixture();
        let registry = context.commands();
        let coverage = ReadonlyAuthoredCoverage::capture(&unit, registry).unwrap();
        let (&installed, _) = unit
            .ir_module
            .installed_procedure_body_units
            .iter()
            .next()
            .unwrap();
        let declaration = coverage.source_for(installed).unwrap();
        for id in [installed, declaration] {
            let function = unit.function_for_body(Some(id)).unwrap();
            let store = if id == installed {
                &unit.carried_installed_lowerings[&id]
            } else {
                &unit.body_units[&id]
            };
            assert!(std::ptr::eq(function, store));
            assert_eq!(function.source_metadata_input(), None);
            if id == installed {
                assert_eq!(function.source_lexer_config(), unit.ir_module.lexer_config);
                assert_eq!(
                    unit.installed_procedure_cfg_for_body(id),
                    Some(&function.cfg)
                );
                assert!(unit.original_procedure_cfg_for_body(id).is_none());
                assert!(unit.original_procedure_for_body(id).is_none());
            } else {
                assert_eq!(
                    unit.original_procedure_cfg_for_body(id),
                    Some(&function.cfg)
                );
                assert!(unit.installed_procedure_cfg_for_body(id).is_none());
            }
            let mut duplicated = unit.clone();
            let lowering = duplicated.ir_module.body_lowering(&id).unwrap().clone();
            duplicated.ir_module.procedures.insert(id, lowering);
            assert!(
                duplicated
                    .ir_module
                    .retained_source_bindings
                    .as_ref()
                    .unwrap()
                    .positioned_for_module(&duplicated.ir_module, registry)
                    .is_some()
            );
            assert!(duplicated.function_for_body(Some(id)).is_none());
            assert!(ReadonlyAuthoredCoverage::capture(&duplicated, registry).is_none());
        }
        let original_cfg = &unit.carried_installed_lowerings[&installed].cfg;
        for axis in 0..3 {
            let mut wrong = unit.clone();
            let function = wrong
                .carried_installed_lowerings
                .get_mut(&installed)
                .unwrap();
            match axis {
                0 => function.source_config.strict_quoting = !function.source_config.strict_quoting,
                1 => function.source_config.base_line += 1,
                _ => {
                    function.source_config.leading_bom = match function.source_config.leading_bom {
                        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
                    };
                }
            }
            assert_ne!(function.source_lexer_config(), unit.ir_module.lexer_config);
            assert_eq!(&function.cfg, original_cfg);
            assert!(ReadonlyAuthoredCoverage::capture(&wrong, registry).is_none());
        }
        let mut wrong = unit.clone();
        let profile = context.commands().profile().unwrap();
        wrong
            .carried_installed_lowerings
            .get_mut(&installed)
            .unwrap()
            .source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            unit.ir_module.lexer_config,
        ));
        assert_eq!(
            &wrong.carried_installed_lowerings[&installed].cfg,
            original_cfg
        );
        assert!(ReadonlyAuthoredCoverage::capture(&wrong, registry).is_none());
        let mut wrong = unit.clone();
        wrong.source.push(' ');
        assert!(ReadonlyAuthoredCoverage::capture(&wrong, registry).is_none());
        assert!(ReadonlyAuthoredCoverage::capture(&unit, registry).is_some());
        assert_eq!(unit.ir_module.body_units.len(), 2);
        assert_eq!(unit.body_units.len(), 2);
        assert_eq!(unit.ir_module.carried_installed_lowerings.len(), 2);
        assert_eq!(unit.carried_installed_lowerings.len(), 2);
    }

    #[test]
    fn readonly_authored_coverage_refuses_missing_foreign_and_mismatched_owners() {
        let (context, unit) = fixture();
        assert!(ReadonlyAuthoredCoverage::capture(&unit, context.commands()).is_some());
        let mut absent = unit.clone();
        absent
            .ir_module
            .original_authored_declaration_body_units
            .clear();
        assert!(ReadonlyAuthoredCoverage::capture(&absent, context.commands()).is_none());
        let mut wrong = unit.clone();
        let mut entries = wrong.ir_module.installed_procedure_body_units.iter();
        let (&first, _) = entries.next().unwrap();
        let (_, other) = entries.next().unwrap();
        let other = other.clone();
        wrong
            .ir_module
            .installed_procedure_body_units
            .insert(first, other);
        assert!(ReadonlyAuthoredCoverage::capture(&wrong, context.commands()).is_none());
        let mut body = unit.clone();
        let id = *body
            .ir_module
            .installed_procedure_body_units
            .keys()
            .next()
            .unwrap();
        body.ir_module
            .carried_installed_lowerings
            .get_mut(&id)
            .unwrap()
            .params_raw
            .push_str(" foreign");
        assert!(ReadonlyAuthoredCoverage::capture(&body, context.commands()).is_none());
        let mut cfg = unit.clone();
        cfg.carried_installed_lowerings
            .get_mut(&id)
            .unwrap()
            .body_id = None;
        assert!(ReadonlyAuthoredCoverage::capture(&cfg, context.commands()).is_none());
        let mut carrier = unit.clone();
        let (source, procedure) = carrier
            .ir_module
            .carried_installed_lowerings
            .remove(&id)
            .unwrap();
        let other = *carrier
            .ir_module
            .original_authored_declaration_body_units
            .keys()
            .find(|&&authored| authored != source)
            .unwrap();
        carrier
            .ir_module
            .carried_installed_lowerings
            .insert(id, other, procedure);
        assert!(ReadonlyAuthoredCoverage::capture(&carrier, context.commands()).is_none());
        let foreign = CommandRegistry::build_default();
        assert!(ReadonlyAuthoredCoverage::capture(&unit, &foreign).is_none());
        assert_eq!(
            unit.ir_module
                .original_authored_declaration_body_units
                .len(),
            2
        );
        assert_eq!(unit.ir_module.installed_procedure_body_units.len(), 2);
    }

    #[test]
    fn carried_lowerings_are_exactly_the_readonly_authored_pairs() {
        let (context, unit) = fixture();
        let registry = context.commands();
        assert!(ReadonlyAuthoredCoverage::pairs(&unit, registry).is_none());
        let mut separate = unit.clone();
        let carried: Vec<_> = separate
            .ir_module
            .carried_installed_lowerings
            .keys()
            .copied()
            .collect();
        for id in carried {
            let (_, procedure) = separate
                .ir_module
                .carried_installed_lowerings
                .remove(&id)
                .unwrap();
            let function = separate.carried_installed_lowerings.remove(&id).unwrap();
            separate.ir_module.body_units.insert(id, procedure);
            separate.body_units.insert(id, function);
        }
        assert_eq!(separate.ir_module.body_units.len(), 4);
        assert!(ReadonlyAuthoredCoverage::capture(&separate, registry).is_some());
        let mut repeated = separate.clone();
        let installed: Vec<_> = repeated
            .ir_module
            .installed_procedure_body_units
            .keys()
            .copied()
            .collect();
        let allocation = repeated.ir_module.installed_procedure_body_units[&installed[0]].clone();
        repeated
            .ir_module
            .installed_procedure_body_units
            .insert(installed[1], allocation);
        // The second body is refused, yet it still shares the first's
        // declaration site, so neither is carried.
        let blocked = ReadonlyAuthoredCoverage::pairs(&repeated, registry).unwrap();
        assert!(blocked.source_for(installed[0]).is_none());
        assert!(blocked.source_for(installed[1]).is_none());
        let pairs = ReadonlyAuthoredCoverage::pairs(&separate, registry).unwrap();
        for &installed in unit.ir_module.installed_procedure_body_units.keys() {
            assert_eq!(
                pairs.source_for(installed),
                unit.ir_module
                    .carried_installed_lowerings
                    .unit_of(&installed)
            );
        }
        pairs.carry(&mut separate);
        assert!(separate == unit);
    }
}
