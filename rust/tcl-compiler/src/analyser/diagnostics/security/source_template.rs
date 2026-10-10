// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Template diagnostics over selected original source and independent SSA values.

use super::w102_diagnostic;
use crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation;
use crate::analyser::state::Analyser;
use tcl_core_types::DiagCode;

impl Analyser {
    /// W102 uses the same selected-schema Structure plan as materialized
    /// source footprints. Original lexical substitutions and captured operand
    /// coordinates remain independent of any actual template value/evaluation.
    pub(in crate::analyser) fn emit_w102_subst_injection(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let registry = original.context().commands();
        let Some(metadata) =
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                registry, input,
            )
        else {
            return;
        };
        let Some((performed, index)) = original
            .with_schema(|schema| {
                let performed = schema.substitutions_performed()?;
                let index = crate::value_transfer::source_template_plan(
                    schema,
                    registry,
                    metadata,
                    original.head().config(),
                )
                .map_or_else(
                    || schema.words.arguments().exact_argv_len()?.checked_sub(1),
                    |plan| Some(plan.operand.0),
                )?;
                Some((performed, index))
            })
            .flatten()
        else {
            return;
        };
        let Some(word) = original.word(index) else {
            return;
        };
        if !super::super::usage::original_word_has_substitution(word)
            || (!performed.commands && !performed.variables)
        {
            return;
        }
        let Some(subject) = original.subject(
            crate::analyser::RegistrySourceDiagnosticKind::TemplateSubstitution,
            Some(index),
        ) else {
            return;
        };
        let advice = substitution_narrowing_switches(original, performed);
        self.result.diagnostics.push(
            w102_diagnostic(original.command(), word.span(), performed, advice)
                .with_subject(subject),
        );
    }

    /// Refine W102 only after the actual CU, function, original token vector
    /// and selected effective template operand agree. A missing/stale record
    /// cannot remove an existing source finding or resurrect a replacement.
    pub(in crate::analyser) fn emit_w102_template_plans(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let module = &cu.ir_module;
        if module.source.bytes() != self.source.as_bytes()
            || !module
                .retained_source_bindings
                .as_ref()
                .is_some_and(|owner| owner.matches_module(module, registry))
        {
            return;
        }
        for unit in std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values())
            .chain(cu.body_units.values())
        {
            if unit
                .invocation_metadata_context_for_module(registry, module)
                .is_none()
            {
                continue;
            }
            for record in &unit.sccp.template_plans {
                let Some(segment) = original_template_segment(unit, record) else {
                    continue;
                };
                let head = segment.argv[0];
                let arguments = segment.arg_tokens();
                let Some(original) = self.original_diagnostic_call_at(head, arguments) else {
                    continue;
                };
                let mut original_record = record.clone();
                original_record.span = unit.abs_span(record.span);
                self.refine_original_template_record(&original_record, &original);
            }
        }
    }
    fn refine_original_template_record(
        &mut self,
        record: &crate::sccp::TemplatePlanRecord,
        original: &OriginalDiagnosticInvocation,
    ) {
        let index = record.plan.operand.0;
        let Some(word) = original.word(index) else {
            return;
        };
        if word.span() != record.span
            && word
                .tokens()
                .first()
                .is_none_or(|token| token.span != record.span)
        {
            return;
        }
        let Some(performed) = original
            .with_schema(|schema| schema.substitutions_performed())
            .flatten()
        else {
            return;
        };
        let Some(projected) = original_template_switch_values(original, record) else {
            return;
        };
        let Some(selected) = projected
            .with_schema(|schema| schema.substitutions_performed())
            .flatten()
        else {
            return;
        };
        // The settled plan remains a separate value premise; selected
        // source effects validate its effective operand/switch shape.
        if selected != record.plan.kinds && record.switches.is_some() {
            return;
        }
        if record.switches.is_none() && performed != record.plan.kinds {
            return;
        }
        let Some(subject) = projected.subject(
            crate::analyser::RegistrySourceDiagnosticKind::TemplateSubstitution,
            Some(index),
        ) else {
            return;
        };
        self.result.diagnostics.retain(|diagnostic| {
            !(diagnostic.code == DiagCode::W102
                && diagnostic.subject().is_some_and(|subject| match subject {
                    crate::analyser::DiagnosticSubject::RegistrySource(subject) => {
                        subject.kind()
                            == crate::analyser::RegistrySourceDiagnosticKind::TemplateSubstitution
                            && subject.words() == original.words()
                            && subject.argument() == Some(index)
                    }
                    _ => false,
                }))
        });
        if record.plan.dynamic && (selected.commands || selected.variables) {
            let advice = substitution_narrowing_switches(&projected, selected);
            self.result.diagnostics.push(
                w102_diagnostic(projected.command(), word.span(), selected, advice)
                    .with_subject(subject),
            );
        }
    }
}

/// Advisory option proposals use the original selected vocabulary and
/// captured prefix. Proposed words do not acquire original argv authority.
fn substitution_narrowing_switches(
    original: &OriginalDiagnosticInvocation,
    performed: tcl_registry::substitution::SubstitutionKinds,
) -> Option<Vec<&'static str>> {
    original.with_schema(|schema| {
        let arguments = schema.words.arguments();
        let count = arguments.exact_argv_len()?;
        let (operand, switches) = (0..count)
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>()
            .split_last()
            .map(|(operand, switches)| (*operand, switches.to_vec()))?;
        let switches = switches.into_iter().collect::<Option<Vec<_>>>()?;
        let operand = operand.unwrap_or_default();
        if !schema.option_effects().complete {
            return None;
        }
        let mut chosen: Vec<&'static str> = Vec::new();
        let mut current = performed;
        for option in schema.semantics.options.available() {
            if !current.commands && !current.variables {
                break;
            }
            let mut trial = switches.clone();
            trial.extend(chosen.iter().copied());
            trial.push(option.name);
            trial.push(operand);
            let kinds = schema.authored_source_substitution_proposal(
                tcl_registry::InvocationArguments::literals(&trial),
            );
            let Some(kinds) = kinds else { continue };
            let narrows =
                (current.commands && !kinds.commands) || (current.variables && !kinds.variables);
            let widens = (!current.commands && kinds.commands)
                || (!current.variables && kinds.variables)
                || kinds.backslashes != current.backslashes;
            if narrows && !widens {
                chosen.push(option.name);
                current = kinds;
            }
        }
        (!current.commands && !current.variables).then_some(chosen)
    })?
}

/// A settled record must retain one unanimous original vector at the same
/// template word. Labels, cooked strings and offsets alone cannot select it.
fn original_template_segment(
    unit: &crate::compilation_unit::FunctionUnit,
    record: &crate::sccp::TemplatePlanRecord,
) -> Option<crate::segmenter::SegmentedCommand> {
    let mut sites = unit
        .cfg
        .command_binding_sites
        .iter()
        .filter_map(|site| site.source_tokens.as_deref())
        .chain(
            unit.cfg
                .blocks
                .values()
                .flat_map(|block| block.statements.iter())
                .filter_map(crate::ir::Statement::tokens),
        )
        .filter(|tokens| {
            tokens.synthetic.is_none()
                && tokens.argv.iter().skip(1).any(|span| *span == record.span)
        });
    let tokens = sites.next()?;
    if sites.any(|other| other != tokens) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    binding.original_lexer_config_for_tokens(tokens)?;
    let (segment, recorded) = binding.original_recorded_command()?;
    (recorded.words() == tokens.words() && recorded.argv == tokens.argv).then_some(segment)
}

/// Settled switches supplement genuine ordinary written operands. Captures
/// retain their original literal value and ordinal; disagreement refuses.
fn original_template_switch_values(
    original: &OriginalDiagnosticInvocation,
    record: &crate::sccp::TemplatePlanRecord,
) -> Option<OriginalDiagnosticInvocation> {
    let Some(switches) = &record.switches else {
        return Some(original.clone());
    };
    if switches.len() != record.plan.operand.0 {
        return None;
    }
    let mut literals = Vec::new();
    for (ordinal, value) in switches.iter().enumerate() {
        if let Some(current) = original.literal(ordinal) {
            if current != value {
                return None;
            }
        } else {
            literals.push((original.written_index(ordinal)?, value.clone()));
        }
    }
    original.with_supplemental_literals(&literals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{
        AnalysisResult, DiagnosticSubject, RegistrySourceDiagnosticKind, ResolvedAnalysisInput,
    };
    use std::sync::Arc;

    fn input(
        context: Arc<tcl_registry::model::ContextRegistry>,
        config: tcl_lexer::LexerConfig,
    ) -> ResolvedAnalysisInput {
        let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
        profile.grammar = config.grammar_over(profile.grammar);
        let profile = profile.intern();
        let input = ResolvedAnalysisInput::new(profile, profile, context, config);
        assert!(input.has_logical_source_name_context());
        input
    }
    fn current_input(dialect: &str) -> ResolvedAnalysisInput {
        let profile =
            tcl_dialect::DialectProfile::find(dialect).expect("authored template grammar");
        input(
            tcl_registry::model::ingress::resolve_environment(dialect).default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        )
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse(source, "tcl");
        analyser
    }
    fn findings(result: &AnalysisResult) -> Vec<&crate::analyser::Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W102)
            .collect()
    }
    fn last_original(analyser: &Analyser, source: &str) -> OriginalDiagnosticInvocation {
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.lexer_config(),
        )
        .pop()
        .unwrap();
        analyser
            .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
            .unwrap()
    }
    fn cu(source: &str, input: &ResolvedAnalysisInput) -> crate::compilation_unit::CompilationUnit {
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }

    #[test]
    fn original_template_diagnostics_follow_aliases_and_refuse_known_replacements() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        // Selected source advice only; no actual template value or evaluation.
        let input = current_input("tcl8.6");
        for source in [
            "::subst $tmpl",
            "rename subst Held; Held $tmpl",
            "interp alias {} render {} subst; render $tmpl",
            "interp alias {} render {} subst -nocommands; render $tmpl",
        ] {
            let analyser = analyse(source, &input);
            let warnings = findings(&analyser.result);
            assert_eq!(
                warnings.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = warnings[0].subject() else {
                panic!("typed template subject")
            };
            assert_eq!(
                subject.kind(),
                RegistrySourceDiagnosticKind::TemplateSubstitution
            );
            if source.contains("-nocommands") {
                assert!(warnings[0].message.contains("any $var"));
                assert!(!warnings[0].message.contains("[cmd]"));
                assert!(warnings[0].message.contains("Add -novariables"));
            }
        }
        for source in [
            "proc subst args {return CUSTOM}; subst $tmpl",
            "rename subst {}; subst $tmpl",
            "namespace eval custom {proc subst args {return CUSTOM}}; ::custom::subst $tmpl",
            "interp alias {} render {} subst; rename subst Held; render $tmpl",
            "interp alias {} render {} subst; proc subst args {return CUSTOM}; render $tmpl",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_template_subject_keeps_captured_switches_and_whole_unicode_word() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input("tcl8.6");
        let source = "interp alias {} render {} subst -nocommands; render \"pré $tmpl\"";
        let analyser = analyse(source, &input);
        let warnings = findings(&analyser.result);
        assert_eq!(warnings.len(), 1);
        let diagnostic = warnings[0];
        assert_eq!(&source[diagnostic.span.as_range()], "\"pré $tmpl\"");
        let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostic.subject() else {
            panic!("typed captured subject")
        };
        assert_eq!(subject.argument(), Some(1));
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(
            subject.words().arguments()[0].literal_bytes(),
            Some(b"-nocommands".as_slice())
        );
        let original = last_original(&analyser, source);
        assert_eq!(subject.words(), original.words());
        assert_eq!(subject.span(), original.word(1).unwrap().span());
        for source in [
            r"subst \$tmpl",
            "subst {pré $tmpl}",
            "subst \"pré littéral\"",
            "subst -nocommands -novariables $tmpl",
        ] {
            let analyser = analyse(source, &input);
            assert!(findings(&analyser.result).is_empty(), "{source}");
        }
    }

    #[test]
    fn original_template_advice_keeps_actual_option_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = current_input("tcl9.1");
        let source = "subst -backslashes $tmpl";
        let current_analyser = analyse(source, &current);
        assert!(findings(&current_analyser.result).is_empty());
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(Arc::clone(current.context_registry().commands())),
        );
        assert!(Arc::ptr_eq(
            older.commands(),
            current.context_registry().commands()
        ));
        let older_input = input(older, current.lexer_config());
        let older_analyser = analyse(source, &older_input);
        assert!(
            older_analyser
                .result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W004)
        );
        let positive = analyse("subst -commands $tmpl", &current);
        let warnings = findings(&positive.result);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("any [cmd]"));
        assert!(
            !warnings[0].message.contains("Add "),
            "positive and negative families stay separate"
        );
    }

    #[test]
    fn original_template_advice_refuses_missing_foreign_and_stale_source_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut descriptor = registry.get("subst").unwrap().clone();
        descriptor.name = "template_current";
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(descriptor);
        let generation = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(generation.commands())),
        );
        let supplied = input(
            Arc::clone(&generation),
            current_input("tcl8.6").lexer_config(),
        );
        let source = "template_current $tmpl";
        let analyser = analyse(source, &supplied);
        assert_eq!(findings(&analyser.result).len(), 1);
        let original = last_original(&analyser, source);
        let unavailable = analyse(source, &input(older, supplied.lexer_config()));
        assert!(findings(&unavailable.result).is_empty());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(OriginalDiagnosticInvocation::new(original.words().clone(), foreign).is_none());
        for axis in 0..4 {
            let mut missing = analyse(source, &supplied);
            missing.result.diagnostics.clear();
            match axis {
                0 => missing.result.resolved_input = None,
                1 => {
                    let mut changed = supplied.clone();
                    changed.config.strict_quoting = !changed.config.strict_quoting;
                    missing.result.resolved_input = Some(changed);
                }
                2 => missing.source = "template_current $other".to_owned(),
                _ => {
                    missing.result.resolved_input = Some(input(
                        tcl_registry::model::ingress::resolve_environment("tcl8.6")
                            .default_context_registry(),
                        supplied.lexer_config(),
                    ))
                }
            }
            missing.emit_w102_subst_injection(Some(&original));
            assert!(findings(&missing.result).is_empty(), "owner axis {axis}");
        }
    }

    #[test]
    fn original_template_refinement_requires_matching_cu_and_selected_switch_values() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Real source CU records supplement original written values only.
        let input = current_input("tcl8.6");
        let source = "proc f {x} {set opt -novariables; subst $opt $x}";
        let unit = cu(source, &input);
        let function = unit.function("::f").expect("original procedure");
        assert_eq!(
            function.sccp.template_plans.len(),
            1,
            "genuine settled record"
        );
        assert_eq!(
            function.sccp.template_plans[0].switches.as_deref(),
            Some(["-novariables".to_owned()].as_slice())
        );
        let mut analyser = analyse(source, &input);
        analyser.emit_w102_template_plans(&unit);
        let warnings = findings(&analyser.result);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("any [cmd]"));
        assert!(!warnings[0].message.contains("$var"));
        assert!(warnings[0].message.contains("Add -nocommands"));
        let Some(DiagnosticSubject::RegistrySource(subject)) = warnings[0].subject() else {
            panic!("settled typed subject")
        };
        assert_eq!(
            subject.supplemental_literals(),
            &[(0, "-novariables".to_owned())]
        );
        for axis in 0..3 {
            let mut stale = unit.clone();
            match axis {
                0 => stale.ir_module.source_metadata_input = None,
                1 => {
                    stale.ir_module.source = tcl_lexer::SourceImage::document(
                        "proc f {x} {set opt -novariables; subst $opt $y}",
                    )
                }
                _ => {
                    stale
                        .procedures
                        .values_mut()
                        .next()
                        .unwrap()
                        .source_metadata_input = None
                }
            }
            let mut analyser = analyse(source, &input);
            let before = analyser.result.diagnostics.clone();
            analyser.emit_w102_template_plans(&stale);
            assert_eq!(
                analyser.result.diagnostics, before,
                "stale refinement axis {axis}"
            );
        }
    }
}
