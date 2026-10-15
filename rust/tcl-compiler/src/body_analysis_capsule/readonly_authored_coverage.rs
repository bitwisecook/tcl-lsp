// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete readonly coverage over separately retained installed analysis rows.

use super::{CompilationUnit, ProcedureId};
use crate::command_binding::{RetainedSourceModuleBindings, SourceInstalledProcedureBody};
use crate::ir::{Module, Procedure, ProcedureBodyKind};
use std::collections::BTreeMap;
use tcl_registry::CommandRegistry;

/// A source coverage relation, never an installed implementation selection.
pub(super) struct ReadonlyAuthoredCoverage {
    installed_sources: BTreeMap<ProcedureId, ProcedureId>,
}

impl ReadonlyAuthoredCoverage {
    pub(super) fn capture(unit: &CompilationUnit, registry: &CommandRegistry) -> Option<Self> {
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_deref()?;
        let inventory = retained
            .positioned_for_module(module, registry)?
            .installed_procedure_body_units();
        if module.original_authored_declaration_body_units.is_empty()
            || unit.body_units.len() != module.body_units.len()
            || module
                .original_authored_declaration_body_units
                .keys()
                .chain(module.installed_procedure_body_units.keys())
                .any(|id| !unit.body_units.contains_key(id))
        {
            return None;
        }
        let mut installed_sources = BTreeMap::new();
        for (&id, function) in &unit.body_units {
            let procedure = module.body_units.get(&id)?;
            if !module.procedure_ids.owns(id)
                || function.body_id != Some(id)
                || module
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
                return None;
            }
            if let Some(card) = module.original_authored_declaration_body_units.get(&id) {
                if module.installed_procedure_body_units.contains_key(&id)
                    || unit.original_procedure_cfg_for_body(id) != Some(&function.cfg)
                    || !retained.matches_original_procedure_summary(id, procedure)
                    || !card.matches_procedure(module, procedure)
                {
                    return None;
                }
                continue;
            }
            if unit.installed_procedure_cfg_for_body(id) != Some(&function.cfg)
                || !retained.matches_installed_procedure_summary(id, procedure)
            {
                return None;
            }
            let allocation = module.installed_procedure_body_units.get(&id)?;
            let mut records = inventory
                .iter()
                .filter(|record| &record.allocation == allocation);
            let record = records.next()?;
            if records.next().is_some() || !matches_installed_row(procedure, record) {
                return None;
            }
            let authored = unique_authored_source(module, retained, record)?;
            if installed_sources
                .values()
                .any(|&existing| existing == authored)
            {
                return None;
            }
            installed_sources.insert(id, authored);
        }
        Some(Self { installed_sources })
    }

    pub(super) fn source_for(&self, installed: ProcedureId) -> Option<ProcedureId> {
        self.installed_sources.get(&installed).copied()
    }
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
    retained: &RetainedSourceModuleBindings,
    installed: &SourceInstalledProcedureBody,
) -> Option<ProcedureId> {
    let mut candidates = module
        .original_authored_declaration_body_units
        .iter()
        .filter_map(|(&id, card)| {
            let procedure = module.body_units.get(&id)?;
            (module.procedure_ids.owns(id)
                && retained.matches_original_procedure_summary(id, procedure)
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
        assert_eq!(module.body_units.len(), 4);
        let coverage = ReadonlyAuthoredCoverage::capture(&unit, context.commands()).unwrap();
        for &installed in module.installed_procedure_body_units.keys() {
            let source = coverage.source_for(installed).unwrap();
            assert_ne!(source, installed);
            assert!(
                module
                    .original_authored_declaration_body_units
                    .contains_key(&source)
            );
            assert!(module.body_units[&source].body.executed_source.is_none());
            assert!(module.body_units[&installed].body.executed_source.is_some());
        }
        assert!(module.source_metadata_input.is_none());
        assert!(module.source_entry.native_entry.is_none());
        assert_eq!(module.installed_procedure_body_units.len(), 2);
        assert_eq!(module.original_authored_declaration_body_units.len(), 2);
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
            .body_units
            .get_mut(&id)
            .unwrap()
            .params_raw
            .push_str(" foreign");
        assert!(ReadonlyAuthoredCoverage::capture(&body, context.commands()).is_none());
        let mut cfg = unit.clone();
        cfg.body_units.get_mut(&id).unwrap().body_id = None;
        assert!(ReadonlyAuthoredCoverage::capture(&cfg, context.commands()).is_none());
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
}
