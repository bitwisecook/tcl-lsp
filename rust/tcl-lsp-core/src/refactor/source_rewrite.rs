// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source correspondence and selected-handler advice are separate from the
//! evaluation, control-flow and inserted-store permissions of a rewrite.

use super::{FrameWalk, Refactoring};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_registry::hooks::LoweringHookId;

#[derive(Clone, Copy)]
pub(super) enum RewriteObligation {
    ExpressionEvaluation,
    ControlFlowEvaluation,
    FreshStoreAndInsertion,
}

impl RewriteObligation {
    pub(super) fn refusal(self, title: &str) -> Refactoring {
        let reason = match self {
            Self::ExpressionEvaluation => {
                "missing-expression-evaluation-equivalence: Bracing requires independently equivalent word and expression evaluation"
            }
            Self::ControlFlowEvaluation => {
                "missing-control-flow-equivalence: Changing branch dispatch requires independently equivalent operand evaluation and selected replacement handlers"
            }
            Self::FreshStoreAndInsertion => {
                "missing-fresh-store-and-insertion: Dictionary conversion requires independent temporary-cell, observer, store, insertion and evaluation permissions"
            }
        };
        Refactoring {
            title: title.to_owned(),
            edits: Vec::new(),
            kind: crate::code_actions::ActionKind::RefactorRewrite,
            data_group: None,
            disabled: Some(reason.to_owned()),
        }
    }
}

pub(super) fn select(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    hook: LoweringHookId,
    obligation: RewriteObligation,
) -> Option<(SegmentedCommand, Option<RewriteObligation>)> {
    let walk = FrameWalk::new(source, analysis)?;
    let command = original_command_at(&walk, source, source, 0, cursor, hook, 0)?;
    if !walk.complete() {
        return None;
    }
    // Logical compatibility and original conditional source geometry retain
    // separate rewrite permissions after the same current source selection.
    Some((
        command,
        (!analysis.allows_lexical_declaration_advice()).then_some(obligation),
    ))
}

fn original_command_at(
    walk: &FrameWalk<'_>,
    source: &str,
    region: &str,
    offset: u32,
    cursor: u32,
    hook: LoweringHookId,
    depth: u32,
) -> Option<SegmentedCommand> {
    if super::MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
        return None;
    }
    for command in walk.segment(region, offset) {
        let (start, end) = super::command_span_offsets(source, &command);
        if cursor < start || cursor > end {
            continue;
        }
        let mut regions = walk.same_frame_regions(source, &command);
        regions.extend(walk.frame_shifted_regions(source, &command));
        for (start, end) in regions {
            let (start32, end32) = (u32::try_from(start).ok()?, u32::try_from(end).ok()?);
            if start32 <= cursor && cursor < end32 {
                return original_command_at(
                    walk,
                    source,
                    source.get(start..end)?,
                    start32,
                    cursor,
                    hook,
                    depth + 1,
                );
            }
        }
        let words = walk.source_words(source, &command)?;
        if words.origins().iter().enumerate().skip(1).any(|(index, origin)| {
            !matches!(origin, tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) if *written == index)
        }) {
            return None;
        }
        if words.with_source_schema(&walk.source_context(), |schema| {
            schema.semantics.lowering_hook == Some(hook)
        }) == Some(true)
        {
            return Some(command);
        }
    }
    None
}

#[cfg(test)]
pub(super) fn lexical_analysis(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
) -> AnalysisResult {
    let point = tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "explicit-lexical-rewrite",
        &[],
        "Logical rewrite source advice",
        point,
    )
    .intern();
    let context = crate::context_for_dialect_profile(profile)
        .with_command_store(registry.snapshot().shared_registry());
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(!analysis.has_original_vendor_source_names());
    analysis
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    // Implementation contract: naming.refactor.original-source-rewrite-permissions
    // docs/design/analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md
    #[test]
    fn original_source_rewrites_require_current_handler_and_distinct_permissions() {
        // Implementation contract: naming.refactor.original-source-rewrite-permissions
        // docs/design/analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md
        let source = "expr 1 + 2";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        let action =
            super::super::brace_expr(source, 0, &analysis).expect("selected original expr");
        assert!(action.edits.is_empty());
        assert!(
            action
                .disabled
                .unwrap()
                .starts_with("missing-expression-evaluation-equivalence:")
        );
        assert!(super::super::brace_expr("# displaced\nexpr 1 + 2", 0, &analysis).is_none());
        for (name, hook) in [
            ("expr", LoweringHookId::Expr),
            ("if", LoweringHookId::If),
            ("switch", LoweringHookId::Switch),
        ] {
            let source = format!("proc {name} {{args}} {{}}; {name} 1");
            let cursor = u32::try_from(source.rfind(name).unwrap()).unwrap();
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            assert!(
                select(
                    &source,
                    cursor,
                    &analysis,
                    hook,
                    RewriteObligation::FreshStoreAndInsertion
                )
                .is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn source_rewrites_keep_actual_availability_and_command_horizons() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("expr").unwrap().clone()
        });
        let commands = registry.snapshot().shared_registry();
        let current = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(std::sync::Arc::clone(&commands)),
        );
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(commands),
        );
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let source = "interp alias {} evaluate {} expr; evaluate 1 + 2";
        let invocation_cursor = u32::try_from(source.rfind("evaluate").unwrap()).unwrap();
        let analyse = |source: &str, context| {
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name)
        };
        let analysis = analyse(source, std::sync::Arc::clone(&current));
        assert!(
            select(
                source,
                invocation_cursor,
                &analysis,
                LoweringHookId::Expr,
                RewriteObligation::ExpressionEvaluation
            )
            .is_some()
        );
        assert!(
            select(
                source,
                invocation_cursor,
                &analyse(source, older),
                LoweringHookId::Expr,
                RewriteObligation::ExpressionEvaluation
            )
            .is_none()
        );

        let replaced = "proc expr args {}; expr 1 + 2";
        let cursor = u32::try_from(replaced.rfind("expr").unwrap()).unwrap();
        assert!(
            select(
                replaced,
                cursor,
                &analyse(replaced, std::sync::Arc::clone(&current)),
                LoweringHookId::Expr,
                RewriteObligation::ExpressionEvaluation
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            config,
        ));
        let mut stale = analysis;
        stale.body_lexer_config.as_mut().unwrap().braced_var =
            tcl_dialect::BracedVarStyle::FirstClose;
        for unavailable in [missing, foreign, stale] {
            assert!(
                select(
                    source,
                    invocation_cursor,
                    &unavailable,
                    LoweringHookId::Expr,
                    RewriteObligation::ExpressionEvaluation
                )
                .is_none()
            );
        }
    }

    #[test]
    fn logical_source_rewrite_keeps_captured_values_outside_written_clause_positions() {
        // naming.refactor.original-source-rewrite-permissions
        // docs/design/analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md
        let plain = "interp alias {} evaluate {} expr\nevaluate 1 + 2";
        let analysis = Analyser::new().analyse(plain, "tcl");
        let cursor = u32::try_from(plain.rfind("evaluate").unwrap()).unwrap();
        let (_, obligation) = select(
            plain,
            cursor,
            &analysis,
            LoweringHookId::Expr,
            RewriteObligation::ExpressionEvaluation,
        )
        .expect("written operands of selected original alias");
        assert!(obligation.is_none());
        let captured = "interp alias {} evaluate {} expr 1\nevaluate + 2";
        let analysis = Analyser::new().analyse(captured, "tcl");
        let cursor = u32::try_from(captured.rfind("evaluate").unwrap()).unwrap();
        assert!(
            select(
                captured,
                cursor,
                &analysis,
                LoweringHookId::Expr,
                RewriteObligation::ExpressionEvaluation
            )
            .is_none()
        );
    }

    // Implementation contract: naming.refactor.original-source-rewrite-permissions
    // docs/design/analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md
    #[test]
    fn original_dictionary_conversion_does_not_insert_into_an_existing_or_observed_cell() {
        // Implementation contract: naming.refactor.original-source-rewrite-permissions
        // docs/design/analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md
        for prefix in [
            "set result_map kept; ",
            "set result_map kept; trace add variable result_map read observer; ",
        ] {
            let source = format!(
                "{prefix}switch -exact -- $subject {{A {{return first}} B {{return second}}}}"
            );
            let cursor = u32::try_from(source.find("switch").unwrap()).unwrap();
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            if let Some(action) = super::super::switch_to_dict(
                &source,
                cursor,
                &analysis,
                &tcl_lexer::LineIndex::new(&source),
            ) {
                assert!(action.edits.is_empty());
                assert!(
                    action
                        .disabled
                        .unwrap()
                        .starts_with("missing-fresh-store-and-insertion:")
                );
            } else {
                assert!(
                    select(
                        &source,
                        cursor,
                        &analysis,
                        LoweringHookId::Switch,
                        RewriteObligation::FreshStoreAndInsertion
                    )
                    .is_none()
                );
            }
        }
    }
}
