// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original definition-member reference syntax used solely for rename hazards.

use std::collections::{BTreeSet, VecDeque};
use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_compiler::registry_invocation::{OriginalSourceScriptBody, source_structure};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::model::ContextRegistry;

struct MemberReferenceScan<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    class: &'a ClassDef,
    method: &'a str,
    context: std::sync::Arc<ContextRegistry>,
}

pub(super) fn spans(
    source: &str,
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class: &ClassDef,
    method: &str,
) -> Vec<Span> {
    inventory(source, analysis, dialect, class, method)
        .map(|(spans, _)| spans)
        .unwrap_or_default()
}

pub(super) fn checked_spans(
    source: &str,
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class: &ClassDef,
    method: &str,
) -> Option<Vec<Span>> {
    let (spans, complete) = inventory(source, analysis, dialect, class, method)?;
    complete.then_some(spans)
}

fn inventory(
    source: &str,
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class: &ClassDef,
    method: &str,
) -> Option<(Vec<Span>, bool)> {
    // naming.core.original-member-reference-hazard
    // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
    let Some(input) = analysis.resolved_input.as_ref() else {
        return None;
    };
    let config = input.lexer_config();
    if analysis
        .resolved_profile()
        .is_none_or(|profile| profile.name != dialect.name)
        || analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&SourceImage::document(source), config)
        || analysis
            .retained_command_realm()
            .is_none_or(|realm| !realm.matches_resolved_analysis_input(input))
    {
        return None;
    }
    let scan = MemberReferenceScan {
        source,
        analysis,
        class,
        method,
        context: input.context_registry(),
    };
    Some(scan.collect(config))
}

impl MemberReferenceScan<'_> {
    fn class_matches(
        &self,
        declaration: &tcl_compiler::command_binding::OriginalSourceClassDeclaration,
    ) -> bool {
        if self.analysis.allows_retained_logical_declaration_advice() {
            declaration.logical_source_class(self.analysis) == Some(self.class)
        } else {
            declaration
                .source_class(self.analysis)
                .is_some_and(|record| record.metadata() == self.class)
        }
    }

    fn names_class(&self, offset: u32) -> Option<bool> {
        if let Some(class) =
            source_structure::source_class_declaration_at(self.source, self.analysis, offset)
        {
            return Some(self.class_matches(&class));
        }
        source_structure::source_configured_class_at(self.source, self.analysis, offset)
            .map(|target| self.class_matches(target.class_declaration()))
    }

    fn collect(&self, config: LexerConfig) -> (Vec<Span>, bool) {
        let Some(structure) = crate::source_structure::SourceStructure::capture(
            self.source,
            Some(self.analysis),
            config,
        ) else {
            return (Vec::new(), false);
        };
        let mut pending = VecDeque::new();
        let mut complete = true;
        let mut selected_root = false;
        for command in &structure.commands {
            let Some(head) = command.argv.first() else {
                continue;
            };
            let Some(words) =
                source_structure::source_registry_words(self.source, self.analysis, command)
            else {
                complete = false;
                continue;
            };
            let target = self.names_class(head.span.start());
            if words
                .source_script_bodies_for_syntax_coverage(&self.context)
                .is_none()
            {
                complete = false;
            }
            for body in words.source_script_bodies(&self.context) {
                if let Some(parent) = body.definition_parent_for(&self.context, None) {
                    match target {
                        Some(true) => {
                            selected_root = true;
                            pending.push_back((parent, body.content_span(), 0));
                        }
                        Some(false) => {}
                        None => complete = false,
                    }
                } else if target == Some(true) {
                    complete = false;
                }
            }
        }
        let mut visited = BTreeSet::new();
        let mut spans = BTreeSet::new();
        while let Some((parent, region, depth)) = pending.pop_front() {
            if super::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
                complete = false;
                continue;
            }
            if !visited.insert((region.start(), region.end())) {
                continue;
            }
            complete &= self.collect_region(&parent, region, depth, &mut pending, &mut spans);
        }
        (
            spans
                .into_iter()
                .map(|(start, end)| Span::new(start, end))
                .collect(),
            complete && selected_root,
        )
    }

    fn collect_region(
        &self,
        parent: &OriginalSourceScriptBody,
        region: Span,
        depth: u32,
        pending: &mut VecDeque<(OriginalSourceScriptBody, Span, u32)>,
        spans: &mut BTreeSet<(u32, u32)>,
    ) -> bool {
        let Some(members) = parent.definition_member_region(&self.context, region) else {
            return false;
        };
        // Preserve partial positive spans below, but only the genuine whole
        // parent lexical plan can close omitted or malformed source regions.
        let mut complete = members.complete_original_commands().is_some();
        for command in members.original_commands() {
            for reference in members.references(command).into_iter().flatten() {
                if reference.kind() == tcl_registry::definer::MemberRefKind::Method
                    && reference.matches_reported_name(self.method)
                    && let Ok(span) = reference.original_word().content_span()
                {
                    spans.insert((span.start(), span.end()));
                }
            }
            if members.member_keyword(command).is_some()
                && members.checked_references(command).is_none()
            {
                complete = false;
            }
            if let Some(scripts) = members.script_bodies(command) {
                if members.checked_script_bodies(command).is_none() {
                    complete = false;
                }
                for body in scripts {
                    if let Some(parent) = body.definition_parent() {
                        pending.push_back((parent.clone(), body.content_span(), depth + 1));
                    }
                }
            } else if let Some(offset) = command.first().map(|head| head.span().start())
                && let Some(words) = source_structure::source_registry_words_in_definition_body_at(
                    self.source,
                    self.analysis,
                    offset,
                    parent,
                )
            {
                if words
                    .source_script_bodies_for_syntax_coverage(&self.context)
                    .is_none()
                {
                    complete = false;
                }
                for body in words.source_script_bodies(&self.context) {
                    if let Some(parent) = body.definition_parent_for(&self.context, Some(parent)) {
                        pending.push_back((parent, body.content_span(), depth + 1));
                    } else {
                        complete = false;
                    }
                }
            } else {
                complete = false;
            }
        }
        complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
    use tcl_registry::{CommandRegistry, model::ingress};

    fn analyse(source: &str, dialect: &'static str) -> AnalysisResult {
        let profile = ingress::resolve_environment(dialect).analyser_profile();
        let mut registry = CommandRegistry::build_default();
        for (original, custom) in [
            ("oo::class", "class-maker"),
            ("oo::define", "configure-class"),
        ] {
            let mut spec = registry.get(original).unwrap().clone();
            spec.name = custom;
            registry.insert(spec);
        }
        let context =
            ingress::context_for_profile(profile).with_command_store(std::sync::Arc::new(registry));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, dialect)
    }

    fn member_spans(source: &str, analysis: &AnalysisResult, class: &ClassDef) -> Vec<Span> {
        spans(
            source,
            analysis,
            analysis.resolved_profile().unwrap(),
            class,
            "m",
        )
    }

    #[test]
    fn checked_member_hazards_keep_unknown_coverage_separate_from_partial_spans() {
        let source = "class-maker create C {method m {} {}; export m; export $computed}";
        for dialect in ["tcl", "tcl8.6"] {
            let plain = "class-maker create C {method m {} {}}";
            let plain_analysis = analyse(plain, dialect);
            let plain_class = plain_analysis.all_classes.get("::C").unwrap();
            assert_eq!(
                checked_spans(
                    plain,
                    &plain_analysis,
                    plain_analysis.resolved_profile().unwrap(),
                    plain_class,
                    "m"
                ),
                Some(Vec::new())
            );
            let analysis = analyse(source, dialect);
            let class = analysis.all_classes.get("::C").unwrap();
            let original = member_spans(source, &analysis, class);
            assert_eq!(original.len(), 1, "{dialect}");
            assert_eq!(&source[original[0].as_range()], "m");
            assert!(
                checked_spans(
                    source,
                    &analysis,
                    analysis.resolved_profile().unwrap(),
                    class,
                    "m"
                )
                .is_none()
            );
            assert!(
                checked_spans(
                    &source.replace("export m", "export n"),
                    &analysis,
                    analysis.resolved_profile().unwrap(),
                    class,
                    "m"
                )
                .is_none()
            );
        }
    }

    #[test]
    fn checked_member_hazards_refuse_fatal_member_tail_without_losing_prefix_spans() {
        for dialect in ["tcl", "tcl8.6"] {
            for (source, count) in [
                (
                    "class-maker create C {method m {} {}; export m; \"unterminated}",
                    1,
                ),
                ("class-maker create C {\"unterminated}", 0),
            ] {
                let analysis = analyse(source, dialect);
                let class = analysis.all_classes.get("::C").unwrap();
                let partial = member_spans(source, &analysis, class);
                assert_eq!(partial.len(), count, "{dialect}: {source}");
                assert!(partial.iter().all(|span| &source[span.as_range()] == "m"));
                assert!(
                    checked_spans(
                        source,
                        &analysis,
                        analysis.resolved_profile().unwrap(),
                        class,
                        "m"
                    )
                    .is_none(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn member_hazards_use_retained_custom_schema_and_nested_definition_vocabulary() {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        // Readonly source syntax; no native table, entered worker or edit permission.
        let source = "class-maker create C {method m {} {}; export m; self {export m}; if {1} {export m}}; configure-class C {export m}; class-maker create D {export m}";
        for dialect in ["tcl", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = analyse(source, dialect);
            let class = analysis.all_classes.get("::C").unwrap();
            let found = member_spans(source, &analysis, class);
            let expected: Vec<_> = source
                .match_indices("export m")
                .take(4)
                .map(|(at, _)| {
                    Span::new(
                        u32::try_from(at + 7).unwrap(),
                        u32::try_from(at + 8).unwrap(),
                    )
                })
                .collect();
            assert_eq!(found, expected, "{dialect}");
            assert!(found.iter().all(|span| &source[span.as_range()] == "m"));
        }
    }

    #[test]
    fn member_hazards_follow_original_configured_target_moves_and_captured_operands() {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        for dialect in ["tcl", "tcl9.0"] {
            for (tail, visible) in [
                (
                    "rename C Held; interp alias {} configure {} configure-class Held; configure {export m}",
                    true,
                ),
                (
                    "rename configure-class configured; configured C {export m}",
                    true,
                ),
                ("rename C {}; configure-class C {export m}", false),
                ("rename C Held; configure-class C {export m}", false),
                (
                    "proc configure-class {args} {}; configure-class C {export m}",
                    false,
                ),
            ] {
                let source = format!("class-maker create C {{method m {{}} {{}}}}; {tail}");
                let analysis = analyse(&source, dialect);
                let class = analysis.all_classes.values().next().unwrap();
                let found = member_spans(&source, &analysis, class);
                assert_eq!(found.len(), usize::from(visible), "{dialect}: {source}");
                if visible {
                    assert_eq!(&source[found[0].as_range()], "m");
                }
            }
        }
    }

    #[test]
    fn member_hazards_refuse_stale_missing_and_unowned_class_metadata() {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        let source =
            "class-maker create C {method m {} {}; export m}; configure-class C {export m}";
        for dialect in ["tcl", "tcl9.0"] {
            let mut analysis = analyse(source, dialect);
            let class = analysis.all_classes["::C"].clone();
            assert_eq!(member_spans(source, &analysis, &class).len(), 2);
            assert!(
                member_spans(&source.replace("export", "filter"), &analysis, &class).is_empty()
            );
            let mut unowned = class.clone();
            unowned.name_span = Span::new(0, 0);
            assert!(member_spans(source, &analysis, &unowned).is_empty());
            let mut stale_config = analysis.clone();
            let bom = &mut stale_config.body_lexer_config.as_mut().unwrap().leading_bom;
            *bom = if *bom == tcl_lexer::LeadingBom::Skip {
                tcl_lexer::LeadingBom::Content
            } else {
                tcl_lexer::LeadingBom::Skip
            };
            assert!(member_spans(source, &stale_config, &class).is_empty());
            analysis.resolved_input = None;
            // Keep the original requested profile; a missing input must not be repaired.
            let profile = ingress::resolve_environment(dialect).analyser_profile();
            assert!(spans(source, &analysis, profile, &class, "m").is_empty());
        }
    }
}
