// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Flat conditional path expressions from authentic retained source words.

use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::realm::CommandBindingRealm;
use crate::registry_invocation::source_structure::{
    OriginalRegistryWords, source_registry_words, source_registry_words_at,
};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, ExecutableText, NativeWord, SourceChannel, SourceImage};
use tcl_registry::SourcePathOperation;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathPart {
    Literal(String),
    Variable(String),
    Command {
        operation: SourcePathOperation,
        arguments: Vec<usize>,
    },
}

/// Sealed conditional source expression retaining its complete original owner.
/// Capture chooses typed authored operations and exact effective arguments;
/// evaluation never reinterprets a reported or written command head. This
/// receipt supplies no native object, entered body, cell or execution grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourcePathExpression {
    input: ResolvedAnalysisInput,
    image: SourceImage,
    realm: Arc<CommandBindingRealm>,
    span: tcl_lexer::Span,
    words: Vec<Vec<PathPart>>,
}

impl OriginalSourcePathExpression {
    /// Apply the explicit path algebra using caller-supplied document and
    /// variable advice. The result is a candidate path, not an observed value
    /// or a filesystem/current-filename fact.
    #[must_use]
    pub fn evaluate(
        &self,
        info_script: Option<&str>,
        resolve_var: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        super::fold_path_value(&self.evaluate_word_value(info_script, resolve_var)?)
    }

    /// Exact original word span retained by this conditional source plan.
    #[must_use]
    pub const fn source_span(&self) -> tcl_lexer::Span {
        self.span
    }

    /// Same immutable analysis, complete input and source generation. This
    /// correspondence supplies no runtime value or file evaluation.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &AnalysisResult) -> bool {
        analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.body_lexer_config == Some(self.input.lexer_config())
            && analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            && analysis
                .retained_command_realm_arc()
                .is_some_and(|realm| Arc::ptr_eq(realm, &self.realm))
    }

    /// Same retained source inventory before a host folds indexed edges.
    #[must_use]
    pub fn matches_assignments(&self, assignments: &super::PathConstantAssignments) -> bool {
        assignments.matches_source_owner(&self.input, &self.image, &self.realm)
    }

    fn evaluate_word_value(
        &self,
        info_script: Option<&str>,
        resolve_var: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        let info_script = info_script
            .map(super::to_tcl_slash_form)
            .filter(|path| !super::is_drive_relative(path));
        let mut values: Vec<Option<String>> = vec![None; self.words.len()];
        for (id, parts) in self.words.iter().enumerate().rev() {
            let mut value = String::new();
            for part in parts {
                match part {
                    PathPart::Literal(text) => value.push_str(text),
                    PathPart::Variable(name) => value.push_str(&resolve_var(name)?),
                    PathPart::Command {
                        operation,
                        arguments,
                    } => {
                        let arguments = arguments
                            .iter()
                            .map(|&argument| values.get(argument)?.clone())
                            .collect::<Option<Vec<String>>>()?;
                        let result = match operation {
                            SourcePathOperation::Join => super::path_join(&arguments),
                            SourcePathOperation::Dirname => super::path_dirname(arguments.first()?),
                            SourcePathOperation::Normalize => {
                                super::eval_file_normalize(arguments.first()?)?
                            }
                            SourcePathOperation::ScriptPath => info_script.clone()?,
                        };
                        value.push_str(&result);
                    }
                }
            }
            values[id] = Some(value);
        }
        values.into_iter().next()?
    }
}

struct Builder<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    words: Vec<Vec<PathPart>>,
    pending: Vec<(usize, NativeWord)>,
}

impl Builder<'_> {
    fn add_word(&mut self, word: NativeWord) -> usize {
        let id = self.words.len();
        self.words.push(Vec::new());
        self.pending.push((id, word));
        id
    }

    fn add_literal(&mut self, bytes: &[u8]) -> Option<usize> {
        let value = std::str::from_utf8(bytes).ok()?.to_owned();
        let id = self.words.len();
        self.words.push(vec![PathPart::Literal(value)]);
        Some(id)
    }

    fn command(&mut self, body: tcl_lexer::Span) -> Option<PathPart> {
        let config = self.analysis.resolved_input.as_ref()?.lexer_config();
        let source = self.source.get(body.as_range())?;
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(source, body.start(), config);
        let [command] = commands.as_slice() else {
            return None;
        };
        let words = source_registry_words(self.source, self.analysis, command)?;
        let context = self
            .analysis
            .resolved_input
            .as_ref()?
            .borrowed_context_registry();
        let selected = words.with_source_schema(context, |invocation| {
            invocation.authored_source_path_operation()
        })??;
        let arguments = selected
            .arguments
            .map(|argument| self.argument(&words, argument))
            .collect::<Option<Vec<_>>>()?;
        Some(PathPart::Command {
            operation: selected.operation,
            arguments,
        })
    }

    fn argument(&mut self, words: &OriginalRegistryWords, argument: usize) -> Option<usize> {
        if let Some(bytes) = words.arguments().get(argument)?.literal_bytes() {
            return self.add_literal(bytes);
        }
        let word = words.operands().get(argument)?.as_ref()?.word()?.clone();
        Some(self.add_word(word))
    }

    fn collect(mut self) -> Option<Vec<Vec<PathPart>>> {
        while let Some((id, word)) = self.pending.pop() {
            if word.group().expand {
                return None;
            }
            if let Some(value) = tcl_syntax::word_rules::original_static_word_source_bytes(&word) {
                self.words[id] = vec![PathPart::Literal(String::from_utf8(value).ok()?)];
                continue;
            }
            let arena = word.executable_parts();
            let mut parts = Vec::new();
            for component in arena.list(arena.root()) {
                parts.push(match &component.part {
                    ExecutablePart::Text(ExecutableText::Original) => {
                        let value = tcl_syntax::backslash::source_literal_bytes(
                            arena.bytes(component.span)?,
                            SourceChannel::Document,
                        );
                        PathPart::Literal(std::str::from_utf8(&value).ok()?.to_owned())
                    }
                    ExecutablePart::Text(ExecutableText::Decoded(value)) => {
                        PathPart::Literal(std::str::from_utf8(value).ok()?.to_owned())
                    }
                    ExecutablePart::Variable { name, index: None } => {
                        let bytes = arena.bytes(*name)?;
                        if tcl_syntax::naming::split_element_ref_bytes(bytes).is_some() {
                            return None;
                        }
                        PathPart::Variable(std::str::from_utf8(bytes).ok()?.to_owned())
                    }
                    ExecutablePart::Command { body } => self.command(*body)?,
                    _ => return None,
                });
            }
            self.words[id] = parts;
        }
        Some(self.words)
    }
}

/// Capture only genuine whole source words under the same complete analysis,
/// selected grammar and retained realm. Missing/stale context, unsupported
/// operations, expansion or non-source dynamic operands decline the plan.
/// Bound literal arguments retain their exact one-argument values.
#[must_use]
pub fn capture_source_path_expression(
    source: &str,
    analysis: &AnalysisResult,
    word: &NativeWord,
) -> Option<OriginalSourcePathExpression> {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let input = analysis.resolved_input.as_ref()?;
    super::path_constants::authored_policy(input.analyser_profile())?;
    let image = SourceImage::document(source);
    let realm = analysis.retained_command_realm_arc()?;
    if analysis.body_lexer_config != Some(input.lexer_config())
        || word.image() != &image
        || word.config() != input.lexer_config()
        || !analysis.matches_original_source_image(&image, input.lexer_config())
    {
        return None;
    }
    let mut builder = Builder {
        source,
        analysis,
        words: Vec::new(),
        pending: Vec::new(),
    };
    builder.add_word(word.clone());
    Some(OriginalSourcePathExpression {
        input: input.clone(),
        image,
        realm: Arc::clone(realm),
        span: word.word_span(),
        words: builder.collect()?,
    })
}

/// Capture a whole original word at an observed source span. Its original
/// parent head remains independently selected by callers requiring a schema.
#[must_use]
pub fn capture_source_path_expression_at_analysis(
    analysis: &AnalysisResult,
    span: tcl_lexer::Span,
) -> Option<OriginalSourcePathExpression> {
    let (source, word, _) = original_path_word(analysis, span)?;
    capture_source_path_expression(source, analysis, &word)
}

fn original_path_word(
    analysis: &AnalysisResult,
    span: tcl_lexer::Span,
) -> Option<(&str, NativeWord, u32)> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = realm.original_source_image()?;
    if analysis.body_lexer_config != Some(input.lexer_config())
        || !analysis.matches_original_source_image(image, input.lexer_config())
    {
        return None;
    }
    let (word, head) =
        realm.original_written_word_at_span_in_source(image, span, input.lexer_config())?;
    Some((image.try_text().ok()?, word, head))
}

/// Capture one selected possible source-file edge without reparsing its
/// reporting path or manufacturing an analysis from an indexed dialect label.
/// A child declaration keeps its separate original source-load lineage.
#[must_use]
pub fn capture_source_target_path_expression(
    analysis: &AnalysisResult,
    target: &crate::signature_scan::types::SignatureSource,
) -> Option<OriginalSourcePathExpression> {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    if let Some(original) = &target.original_interpreter_source_load {
        original.matches_analysis(analysis).then_some(())?;
        return capture_source_path_expression(
            original.path_word().image().try_text().ok()?,
            analysis,
            original.path_word(),
        );
    }
    let (source, word, head) = original_path_word(analysis, target.range)?;
    let words = source_registry_words_at(source, analysis, head)?;
    let context = analysis
        .resolved_input
        .as_ref()?
        .borrowed_context_registry();
    let tcl_registry::source_navigation::SourceNavigationOperand::File { argument } = words
        .with_source_schema(context, |schema| {
            schema.authored_source_navigation_operand()
        })??
    else {
        return None;
    };
    (words.operands().get(argument)?.as_ref()?.word()? == &word).then_some(())?;
    capture_source_path_expression(source, analysis, &word)
}

/// Selected conditional package-search-path source expression. The selected
/// mutation determines whether its completed word is one element or a list.
/// It supplies no observed global cell, mutation or package installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceAutoPathExpression {
    expression: OriginalSourcePathExpression,
    form: crate::analyser::types::AutoPathForm,
}

impl OriginalSourceAutoPathExpression {
    /// Apply the selected source list grammar after the whole argument value
    /// is complete. No element is reparsed as a Tcl substitution or command.
    #[must_use]
    pub fn evaluate(
        &self,
        info_script: Option<&str>,
        resolve_var: &dyn Fn(&str) -> Option<String>,
    ) -> Option<Vec<String>> {
        let value = self
            .expression
            .evaluate_word_value(info_script, resolve_var)?;
        let values = match self.form {
            crate::analyser::types::AutoPathForm::Append => vec![value],
            crate::analyser::types::AutoPathForm::Assign => {
                tcl_syntax::word_rules::WordValueRules::from_config(
                    &self.expression.input.lexer_config(),
                )
                .split_list(&value)
                .ok()?
                .into_iter()
                .map(|element| element.into_owned())
                .collect()
            }
        };
        values
            .iter()
            .map(|value| super::fold_path_value(value))
            .collect()
    }

    /// Same authentic analysis and inventory as the retained argument plan.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &AnalysisResult) -> bool {
        self.expression.matches_analysis(analysis)
    }
}

/// Join an auto-path reporting row to its actual selected mutation, original
/// receiver operand and complete source word. Unproved local/global aliases,
/// shadows, cooked words, expansion and stale analysis decline this advice.
#[must_use]
pub fn capture_source_auto_path_expression(
    analysis: &AnalysisResult,
    entry: &crate::analyser::types::AutoPathEntry,
) -> Option<OriginalSourceAutoPathExpression> {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    use crate::analyser::types::AutoPathForm;
    let (source, word, head) = original_path_word(analysis, entry.range)?;
    let words = source_registry_words_at(source, analysis, head)?;
    let context = analysis
        .resolved_input
        .as_ref()?
        .borrowed_context_registry();
    let variable = words.with_source_schema(context, |schema| match entry.form {
        AutoPathForm::Assign => {
            let pairs = schema.authored_source_assignment_arguments()?;
            let [(variable, Some(value))] = pairs.as_slice() else {
                return None;
            };
            (words.operands().get(*value)?.as_ref()?.word()? == &word).then_some(*variable)
        }
        AutoPathForm::Append => {
            let mut layout = schema.authored_source_list_append_arguments()?;
            layout.values.find(|&value| {
                words
                    .operands()
                    .get(value)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                    == Some(&word)
            })?;
            Some(layout.variable)
        }
    })??;
    let receiver = words.arguments().get(variable)?.literal_bytes()?;
    if receiver != b"::auto_path"
        && (receiver != b"auto_path"
            || analysis.offset_is_inside_any_definition_body(head)
            || !analysis.original_namespace_scope_at(head)?.is_root())
    {
        return None;
    }
    Some(OriginalSourceAutoPathExpression {
        expression: capture_source_path_expression(source, analysis, &word)?,
        form: entry.form,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_value(
        source: &str,
        profile: &'static tcl_dialect::DialectProfile,
        config: tcl_lexer::LexerConfig,
    ) -> (AnalysisResult, NativeWord) {
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, commands.last().unwrap()).unwrap();
        let word = words.operands()[1]
            .as_ref()
            .unwrap()
            .word()
            .unwrap()
            .clone();
        (analysis, word)
    }

    fn analysed(source: &str) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn original_source_target_plans_keep_selected_words_and_inventory_currency() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let source =
            "interp alias {} build {} file join {/ROOT with space}; source [build {literal$é.tcl}]";
        let analysis = analysed(source);
        let [target] = analysis.source_targets.as_slice() else {
            panic!("one source row required");
        };
        let expression = capture_source_target_path_expression(&analysis, target).unwrap();
        assert_eq!(
            expression
                .evaluate(Some("/ROOT/main.tcl"), &|_| None)
                .as_deref(),
            Some("/ROOT with space/literal$é.tcl")
        );
        assert!(expression.matches_analysis(&analysis));
        assert!(expression.matches_assignments(&analysis.path_constant_assignments));
        let mut reporting = target.clone();
        reporting.raw_path = "unrelated.tcl".into();
        reporting.is_literal = true;
        assert_eq!(
            capture_source_target_path_expression(&analysis, &reporting),
            Some(expression.clone())
        );
        let separately_analysed = analysed(source);
        assert!(!expression.matches_analysis(&separately_analysed));
        assert!(!expression.matches_assignments(&separately_analysed.path_constant_assignments));
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(capture_source_target_path_expression(&missing, target).is_none());
        let mut stale = analysis.clone();
        stale.body_lexer_config = None;
        assert!(capture_source_path_expression_at_analysis(&stale, target.range).is_none());
        for source in [
            "proc file {args} {}; source [file join /ROOT child.tcl]",
            "rename file moved; interp alias {} file {} list; source [file join /ROOT child.tcl]",
            "source [file join {*}$parts]",
        ] {
            let analysis = analysed(source);
            assert!(
                !analysis.source_targets.is_empty(),
                "source row is a necessary premise: {source}"
            );
            for target in &analysis.source_targets {
                assert!(
                    capture_source_target_path_expression(&analysis, target).is_none(),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn original_auto_path_plans_split_completed_assignment_values_only() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        use super::super::PathConstantLookup;
        for (source, expected) in [
            (
                "set auto_path {/ROOT/a {/ROOT with space}}",
                vec!["/ROOT/a", "/ROOT with space"],
            ),
            (
                "set auto_path [file join /ROOT {a b}]",
                vec!["/ROOT/a", "b"],
            ),
            (
                "lappend auto_path [file join /ROOT {a b}]",
                vec!["/ROOT/a b"],
            ),
            (
                "set auto_path {/ROOT/é {/ROOT/literal$}}",
                vec!["/ROOT/é", "/ROOT/literal$"],
            ),
            (
                "interp alias {} build {} file join /ROOT; lappend auto_path [build {literal$é}]",
                vec!["/ROOT/literal$é"],
            ),
        ] {
            let analysis = analysed(source);
            let [entry] = analysis.auto_path_entries.as_slice() else {
                panic!("one original mutation row required: {source}");
            };
            let expression = capture_source_auto_path_expression(&analysis, entry).unwrap();
            let constants = super::super::fold_constant_assignments(
                &analysis.path_constant_assignments,
                Some("/ROOT/main.tcl"),
            );
            assert_eq!(
                expression
                    .evaluate(Some("/ROOT/main.tcl"), &|name| constants
                        .at(entry.range.start())
                        .path_constant(name))
                    .unwrap(),
                expected,
                "{source}"
            );
            assert!(expression.matches_analysis(&analysis));
            let mut report = entry.clone();
            report.raw_path = "unrelated".into();
            assert_eq!(
                capture_source_auto_path_expression(&analysis, &report),
                Some(expression)
            );
            report.form = match entry.form {
                crate::analyser::types::AutoPathForm::Assign => {
                    crate::analyser::types::AutoPathForm::Append
                }
                crate::analyser::types::AutoPathForm::Append => {
                    crate::analyser::types::AutoPathForm::Assign
                }
            };
            assert!(capture_source_auto_path_expression(&analysis, &report).is_none());
        }
        for source in [
            "namespace eval N {lappend auto_path /ROOT}",
            "proc local {} {set auto_path /ROOT}",
            "proc file {args} {}; lappend auto_path [file join /ROOT leaf]",
            "rename file moved; interp alias {} file {} list; set auto_path [file join /ROOT leaf]",
        ] {
            let analysis = analysed(source);
            assert!(
                !analysis.auto_path_entries.is_empty(),
                "original row required: {source}"
            );
            for entry in &analysis.auto_path_entries {
                assert!(
                    capture_source_auto_path_expression(&analysis, entry).is_none(),
                    "{source}"
                );
            }
        }
        let source = "set auto_path \"{/ROOT/a\"";
        let analysis = analysed(source);
        let entry = analysis
            .auto_path_entries
            .first()
            .expect("original complete argument");
        let expression =
            capture_source_auto_path_expression(&analysis, entry).expect("literal source plan");
        assert!(
            expression
                .evaluate(Some("/ROOT/main.tcl"), &|_| None)
                .is_none()
        );
    }

    #[test]
    fn original_source_path_operations_keep_alias_targets_and_bound_values_separate() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        for (source, expected) in [
            (
                "interp alias {} build {} file join {/ROOT with space}; set path [build {literal$é}]",
                "/ROOT with space/literal$é",
            ),
            (
                "rename file moved; set path [moved join /ROOT leaf]",
                "/ROOT/leaf",
            ),
            (
                "interp alias {} norm {} file normalize; set path [norm /ROOT/a/../leaf]",
                "/ROOT/leaf",
            ),
            (
                "interp alias {} current {} info script; set path [file dirname [current]]",
                "/ROOT",
            ),
        ] {
            let (analysis, word) = last_value(source, profile, config);
            let expression = capture_source_path_expression(source, &analysis, &word).unwrap();
            assert_eq!(
                expression
                    .evaluate(Some("/ROOT/main.tcl"), &|_| None)
                    .as_deref(),
                Some(expected),
                "{source}"
            );
            let constants = super::super::fold_constant_assignments(
                &analysis.path_constant_assignments,
                Some("/ROOT/main.tcl"),
            );
            assert_eq!(constants.get("path").map(String::as_str), Some(expected));
        }
        for source in [
            "rename file moved; interp alias {} file {} list; set path [file join /ROOT leaf]",
            "proc file {args} {}; set path [file join /ROOT leaf]",
            "set path [file join {*}$parts]",
            "set path [file join /ROOT leaf; list elsewhere]",
        ] {
            let (analysis, word) = last_value(source, profile, config);
            assert!(
                capture_source_path_expression(source, &analysis, &word).is_none(),
                "{source}"
            );
            let constants =
                super::super::fold_constant_assignments(&analysis.path_constant_assignments, None);
            assert!(constants.get("path").is_none(), "{source}");
        }
    }

    #[test]
    fn original_source_path_parts_keep_selected_variable_roots_and_missing_premises() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        let source = "set path ${a{b}/leaf";
        let (analysis, word) = last_value(source, profile, config);
        let expression = capture_source_path_expression(source, &analysis, &word).unwrap();
        assert_eq!(
            expression
                .evaluate(None, &|name| (name == "a{b").then(|| "/ROOT".to_owned()))
                .as_deref(),
            Some("/ROOT/leaf")
        );
        assert!(
            expression
                .evaluate(None, &|name| (name == "a").then(|| "/WRONG".to_owned()))
                .is_none()
        );
        assert!(
            capture_source_path_expression(source, &AnalysisResult::default(), &word).is_none()
        );
        assert!(
            capture_source_path_expression(&source.replace("leaf", "tail"), &analysis, &word)
                .is_none()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(
                analysis
                    .resolved_input
                    .as_ref()
                    .unwrap()
                    .borrowed_context_registry()
                    .with_command_store(Arc::new(tcl_registry::CommandRegistry::build_default())),
            ),
            config,
        ));
        assert!(capture_source_path_expression(source, &foreign, &word).is_none());
        let mut stale = analysis.clone();
        stale.body_lexer_config = Some(tcl_lexer::LexerConfig::for_profile(Some(profile)));
        assert!(capture_source_path_expression(source, &stale, &word).is_none());
        let literal_source = "set path \"${root}/one\r\ntwo\\$literal\"";
        let (analysis, word) = last_value(literal_source, profile, config);
        let expression = capture_source_path_expression(literal_source, &analysis, &word).unwrap();
        assert_eq!(
            expression
                .evaluate(None, &|name| (name == "root").then(|| "/ROOT".to_owned()))
                .as_deref(),
            Some("/ROOT/one\ntwo$literal"),
        );
        for source in ["set path $a(index)", "set path ${a(index)}"] {
            let (analysis, word) = last_value(source, profile, config);
            assert!(capture_source_path_expression(source, &analysis, &word).is_none());
        }
    }

    #[test]
    fn original_source_path_jim_script_advice_requires_current_known_input_and_explicit_filename() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let source = "set path [info script]";
        let (analysis, word) = last_value(source, profile, config);
        let expression = capture_source_path_expression(source, &analysis, &word).unwrap();
        assert_eq!(
            expression
                .evaluate(Some("/ROOT/main.tcl"), &|_| None)
                .as_deref(),
            Some("/ROOT/main.tcl")
        );
        assert!(expression.evaluate(None, &|_| None).is_none());
        let unknown = tcl_dialect::DialectProfile::projected_from_point(
            "source-path-jim079",
            &[],
            "Unknown Jim source path recipe",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let unknown_input = ResolvedAnalysisInput::new(
            unknown,
            unknown,
            tcl_registry::model::ingress::context_for_profile(unknown),
            tcl_lexer::LexerConfig::for_profile(Some(unknown)),
        );
        let unknown_analysis = crate::analyser::Analyser::new()
            .with_resolved_input(unknown_input)
            .analyse(source, unknown.name);
        assert!(capture_source_path_expression(source, &unknown_analysis, &word).is_none());
    }
}
