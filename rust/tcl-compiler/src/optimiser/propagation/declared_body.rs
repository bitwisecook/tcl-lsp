// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Closed retained calls in original declaration syntax.

use super::{CompilationUnit, DiagCode, Optimisation, PassContext};
use crate::ir::{CommandTokens, Procedure, ProcedureId, Statement, WordExpr};
use crate::registry_invocation::InvocationMetadataContext;
use tcl_registry::CommandRegistry;

struct CurrentSourceUnit<'a> {
    unit: &'a CompilationUnit,
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
}

struct OriginalBodyCallSource<'a> {
    unit: &'a CompilationUnit,
    procedure: &'a Procedure,
    card: &'a crate::command_binding::OriginalNativeProcedureBody,
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
}

impl<'a> CurrentSourceUnit<'a> {
    fn capture(
        unit: &'a CompilationUnit,
        registry: &'a CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        let prepared = unit.original_optimiser_prepared_unit(registry, dialect)?;
        if !prepared.is_whole_source_unit() || unit.ir_module.lexer_config != config {
            return None;
        }
        Some(Self {
            unit,
            registry,
            metadata: InvocationMetadataContext::for_module(registry, &unit.ir_module)?,
        })
    }

    fn body(&self, body: ProcedureId) -> Option<OriginalBodyCallSource<'a>> {
        let unit = self.unit;
        let module = &unit.ir_module;
        let procedure = module.body_units.get(&body)?;
        let card = module.original_native_declaration_body_units.get(&body)?;
        if procedure.body.is_authored_source()
            || !module.procedure_ids.owns(body)
            || unit.function_for_body(Some(body))?.body_id != Some(body)
            || !card.matches_procedure(module, procedure)
        {
            return None;
        }
        Some(OriginalBodyCallSource {
            unit,
            procedure,
            card,
            registry: self.registry,
            metadata: self.metadata,
        })
    }
}

impl OriginalBodyCallSource<'_> {
    fn tokens<'s>(&self, index: usize, statement: &'s Statement) -> Option<&'s CommandTokens> {
        self.procedure
            .body
            .statements
            .get(index)
            .is_some_and(|original| std::ptr::eq(original, statement))
            .then_some(())?;
        let tokens = statement.tokens()?;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || !tokens
                .source_binding
                .as_ref()?
                .original_native_source_owner_for_card(
                    tokens,
                    self.metadata.source_analysis_input()?,
                    self.registry,
                    self.card,
                )
        {
            return None;
        }
        Some(tokens)
    }
}

pub(super) fn run(ctx: &mut PassContext<'_>, unit: &CompilationUnit) {
    let Some(registry) = ctx.registry else {
        return;
    };
    if ctx.source != unit.source {
        return;
    }
    let Some(current) = CurrentSourceUnit::capture(unit, registry, ctx.dialect, ctx.lexer_config())
    else {
        return;
    };
    for (body, _) in unit.ir_module.original_source_procedure_bodies() {
        let Some(source) = current.body(*body) else {
            continue;
        };
        source.report(ctx);
    }
}

impl OriginalBodyCallSource<'_> {
    fn report(&self, ctx: &mut PassContext<'_>) {
        let unit = self.unit;
        let registry = self.registry;
        for (index, statement) in self.procedure.body.statements.iter().enumerate() {
            let Some(tokens) = self.tokens(index, statement) else {
                continue;
            };
            let Some(calls) = super::retained_substitution_calls(tokens, &unit.ir_module, registry)
            else {
                continue;
            };
            for word in tokens.words() {
                let WordExpr::CommandSubstitution { source: word, .. } = word else {
                    continue;
                };
                let Some(call) = super::retained_call_at(&calls, word.span.start()) else {
                    continue;
                };
                if !call.tokens.as_ref().is_some_and(literal_call_words) {
                    continue;
                }
                // This existing evaluator still requires the actual registry target,
                // handler, closed original arguments and declared footprint policy.
                let Some(value) = super::fold_retained_builtin(
                    registry,
                    call,
                    &calls,
                    None,
                    Some(self.metadata),
                    ctx.dialect,
                ) else {
                    continue;
                };
                let Some(replacement) = super::render_propagation_word(&value, ctx.lexer_config())
                else {
                    continue;
                };
                ctx.report(Optimisation::new(
                    DiagCode::O129,
                    "Fold constant builtin command substitution",
                    call.span,
                    replacement,
                ));
            }
        }
    }
}

fn literal_call_words(tokens: &CommandTokens) -> bool {
    tokens.words().iter().all(|word| match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Template { parts, .. } => parts
            .iter()
            .all(|part| matches!(part, crate::ir::WordPart::Text { .. })),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_pipeline_folds_only_the_owned_closed_original_substitution() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let source = "proc p {} {set known [string length abcde]; return $known}\n";
        let optimisations =
            crate::optimiser::manager::optimise_raw(source, current.commands(), Some("tcl8.6"));
        let folds = optimisations
            .iter()
            .filter(|optimisation| {
                optimisation.code == DiagCode::O129 && optimisation.replacement == "5"
            })
            .collect::<Vec<_>>();
        assert_eq!(folds.len(), 1, "{optimisations:?}");
        assert_eq!(
            source.get(folds[0].span.as_range()),
            Some("[string length abcde]")
        );
        assert!(!folds[0].hint_only);
        for source in [
            "proc p {} {set known [missing-command abcde]}\n",
            "proc p {value} {set known [string length $value]}\n",
        ] {
            let optimisations =
                crate::optimiser::manager::optimise_raw(source, current.commands(), Some("tcl8.6"));
            assert!(
                !optimisations
                    .iter()
                    .any(|optimisation| optimisation.code == DiagCode::O129),
                "{optimisations:?}"
            );
        }
    }

    #[test]
    fn raw_interprocedural_preparation_transports_only_a_current_source_owner() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = current.commands();
        let dialect = Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let text = "proc p {} {set known [string length abcde]}\n";
        let mut unit = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let origin = unit.original_optimiser_source_origin().unwrap().clone();
        assert!(unit.prepare_raw_optimiser_interprocedural(registry, dialect));
        assert_eq!(unit.original_optimiser_source_origin(), Some(&origin));
        assert!(
            unit.original_optimiser_prepared_unit(registry, dialect)
                .is_some()
        );
        let mut stale = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        stale.interproc = Some(crate::interprocedural::InterproceduralAnalysis::default());
        assert!(
            stale
                .original_optimiser_prepared_unit(registry, dialect)
                .is_none()
        );
        assert!(stale.prepare_raw_optimiser_interprocedural(registry, dialect));
        assert!(
            stale
                .original_optimiser_prepared_unit(registry, dialect)
                .is_none()
        );
        let mut foreign_dialect =
            CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        foreign_dialect.prepare_raw_optimiser_interprocedural(
            registry,
            Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap()),
        );
        assert!(foreign_dialect.original_optimiser_source_origin().is_none());
        let mut foreign_registry =
            CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        foreign_registry
            .prepare_raw_optimiser_interprocedural(&CommandRegistry::build_default(), dialect);
        assert!(
            foreign_registry
                .original_optimiser_source_origin()
                .is_none()
        );
        let mut missing = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        missing.ir_module.source_metadata_input = None;
        assert!(!missing.prepare_raw_optimiser_interprocedural(registry, dialect));
        assert!(missing.original_optimiser_source_origin().is_none());
    }

    #[test]
    fn original_body_source_refuses_foreign_unit_body_registry_and_tokens() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = current.commands();
        let text = "proc p {} {set known [string length abcde]; return $known}\n";
        let unit = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let (body, _) = unit
            .ir_module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let dialect = Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let config = unit.ir_module.lexer_config;
        let current = CurrentSourceUnit::capture(&unit, registry, dialect, config).unwrap();
        let source = current.body(*body).unwrap();
        assert!(!source.procedure.body.is_authored_source());
        assert!(source.procedure.body.executed_source.is_none());
        assert!(source.procedure.body.native_compilation_admission.is_none());
        let original = &source.procedure.body.statements[0];
        assert!(source.tokens(0, original).is_some());
        assert!(source.tokens(usize::MAX, original).is_none());
        assert!(source.tokens(0, &original.clone()).is_none());
        let other = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let other_body = *other
            .ir_module
            .original_native_declaration_body_units
            .keys()
            .next()
            .unwrap();
        assert!(current.body(other_body).is_none());
        assert!(
            CurrentSourceUnit::capture(
                &unit,
                registry,
                Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap()),
                config
            )
            .is_none()
        );
        let foreign = tcl_registry::CommandRegistry::build_default();
        assert!(CurrentSourceUnit::capture(&unit, &foreign, dialect, config).is_none());
        let mut changed = unit.clone();
        changed.source.push(' ');
        assert!(CurrentSourceUnit::capture(&changed, registry, dialect, config).is_none());
    }
}
