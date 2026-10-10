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
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    if analysis.allows_lexical_declaration_advice() {
        let command = super::find_command_at(source, cursor, None, registry, config)?;
        let realm = analysis.retained_command_realm()?;
        let selected = match realm.binding_at(command.name(), command.span.start()) {
            tcl_compiler::realm::RealmBindingFact::Unchanged => command.name(),
            tcl_compiler::realm::RealmBindingFact::Command(name) => name,
            tcl_compiler::realm::RealmBindingFact::Rebound => return None,
        };
        if registry.get(selected)?.lowering_hook != Some(hook) {
            return None;
        }
        return Some((command, None));
    }
    let walk = FrameWalk::new(source, analysis)?;
    let command = original_command_at(&walk, source, source, 0, cursor, hook, 0)?;
    // Original conditional handler selection grants structure only. The
    // individual rewrite must consume its independently selected permission.
    Some((command, Some(obligation)))
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
        if let Some(selected) = walk.structure(source, &command) {
            if selected.facts.lowering_hook == Some(hook) {
                return Some(command);
            }
        } else {
            // An action can describe a selected original handler without
            // asserting expression evaluation or granting its replacement.
            let tokens = walk.tokens(source, &command);
            let advice =
                tcl_compiler::registry_invocation::original_registry_invocation_assistance(
                    walk.nesting,
                    None,
                    &tokens,
                )?;
            let words = advice.unanimous_command_words()?;
            if walk.nesting.get(words.command())?.lowering_hook == Some(hook) {
                return Some(command);
            }
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
