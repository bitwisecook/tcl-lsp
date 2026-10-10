// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-driven traversal of potential Tcl source regions.
//!
//! Consumers that need to inspect a command embedded in another command must
//! not each invent a partial list of body-bearing commands.  This walker keeps
//! the source-coordinate and command-identity rules in one place: it visits
//! normal body arguments, clause-list arm bodies, lambda bodies, definition
//! members, and live command substitutions.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::lambda_literal::{split_lambda_literal, split_original_lambda_literal};
use tcl_compiler::realm::CommandBindingRealm;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Token, TokenType, close_quote_offset};
use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::{ArgRole, CommandRegistry, ScriptTiming};

use crate::oo_body::{HeadWords, is_member, member_body_indices_in, next_definition_grammar};

/// Defensive recursion limit shared by executable-source walkers.
const MAX_EXECUTABLE_REGION_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Whether source belongs to the direct region or a potential body.
/// This classification supplies no reached execution, frame, effects or completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutableContext {
    /// Top-level source or a live command-substitution source region.
    Direct,
    /// A registry-declared body, case action, definition body, or lambda body.
    PotentialBody,
}

/// The deepest potential source region containing a cursor.
///
/// `start` follows the delimiter that introduced the region. `depth` is the
/// lexer nesting depth required to interpret its prefix with the same grammar
/// as [`visit_executable_commands`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutableRegion {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) depth: u32,
}

struct RegionProbe {
    cursor: usize,
    best: Option<ExecutableRegion>,
}

impl RegionProbe {
    fn consider(&mut self, start: usize, end: usize, depth: u32) {
        if start <= self.cursor
            && self.cursor <= end
            && self.best.is_none_or(|best| depth > best.depth)
        {
            self.best = Some(ExecutableRegion { start, end, depth });
        }
    }
}

/// Visit original potential source regions under the same retained analysis.
/// Body applicability stays conditional; this supplies no execution or frame.
pub(crate) fn visit_analysis_executable_commands(
    source: &str,
    analysis: &AnalysisResult,
    visitor: &mut impl FnMut(&SegmentedCommand, HeadWords<'_>, ExecutableContext) -> bool,
) {
    let Some((config, registry, identities)) = analysis_walk_inputs(source, analysis) else {
        return;
    };
    let input = analysis
        .resolved_input
        .as_ref()
        .expect("validated original input");
    let mut walk = ExecutableWalker {
        source,
        config,
        registry,
        availability: Some(input.availability_context().authoring_query()),
        identities,
        visitor,
        region_probe: None,
        analysis: Some(analysis),
        definition_parent: None,
    };
    let _ = walk.region(0, source.len(), 0, None, ExecutableContext::Direct);
}

/// Innermost original source region under the actual complete input/Registry.
pub(crate) fn innermost_analysis_executable_region_at(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cursor: usize,
) -> Option<ExecutableRegion> {
    let (config, actual_registry, identities) = analysis_walk_inputs(source, analysis)?;
    if cursor > source.len()
        || registry.snapshot().semantic_key() != actual_registry.snapshot().semantic_key()
    {
        return None;
    }
    let input = analysis.resolved_input.as_ref()?;
    let mut probe = RegionProbe { cursor, best: None };
    let mut visitor =
        |_command: &SegmentedCommand, _head: HeadWords<'_>, _context: ExecutableContext| false;
    let mut walk = ExecutableWalker {
        source,
        config,
        registry: actual_registry,
        availability: Some(input.availability_context().authoring_query()),
        identities,
        visitor: &mut visitor,
        region_probe: Some(&mut probe),
        analysis: Some(analysis),
        definition_parent: None,
    };
    let _ = walk.region(0, source.len(), 0, None, ExecutableContext::Direct);
    if let Some(region) = identities.original_incomplete_body_region_at(
        &tcl_lexer::SourceImage::document(source),
        config,
        u32::try_from(cursor).ok()?,
        actual_registry,
    ) {
        probe.consider(region.start() as usize, region.end() as usize, 1);
    }
    probe.best
}

fn analysis_walk_inputs<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
) -> Option<(LexerConfig, &'a CommandRegistry, &'a CommandBindingRealm)> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    if input.lexer_config() != config || !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    Some((
        config,
        analysis.resolved_registry()?,
        analysis.retained_command_realm()?,
    ))
}

struct ExecutableWalker<'a, F> {
    source: &'a str,
    config: LexerConfig,
    registry: &'a CommandRegistry,
    availability: Option<SurfaceQuery<'a>>,
    identities: &'a CommandBindingRealm,
    visitor: &'a mut F,
    region_probe: Option<&'a mut RegionProbe>,
    analysis: Option<&'a AnalysisResult>,
    definition_parent: Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
}

impl<F: FnMut(&SegmentedCommand, HeadWords<'_>, ExecutableContext) -> bool>
    ExecutableWalker<'_, F>
{
    fn begin_region(&mut self, start: usize, end: usize, depth: u32) -> bool {
        if MAX_EXECUTABLE_REGION_DEPTH.exceeded(depth) || start > end || end > self.source.len() {
            return false;
        }
        if let Some(probe) = self.region_probe.as_deref_mut() {
            probe.consider(start, end, depth);
        }
        start < end
    }

    fn original_region(
        &mut self,
        span: tcl_lexer::Span,
        depth: u32,
        parent: Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
        context: ExecutableContext,
    ) -> bool {
        let previous = std::mem::replace(&mut self.definition_parent, parent);
        let stopped = self.region(
            span.start() as usize,
            span.end() as usize,
            depth,
            None,
            context,
        );
        self.definition_parent = previous;
        stopped
    }

    fn region(
        &mut self,
        start: usize,
        end: usize,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
        context: ExecutableContext,
    ) -> bool {
        if !self.begin_region(start, end, depth) {
            return false;
        }
        let commands = segment_commands_with_offset_and_config(
            &self.source[start..end],
            u32::try_from(start).unwrap_or(0),
            self.config.at_depth(depth),
        );
        let definition_members = self.analysis.and_then(|analysis| {
            let context = analysis.resolved_input.as_ref()?.context_registry();
            self.definition_parent.as_ref()?.definition_member_region(
                &context,
                tcl_lexer::Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?),
            )
        });
        for command in &commands {
            let Some(head_tok) = command.argv.first() else {
                continue;
            };
            let head = self
                .identities
                .head_words(command.name(), head_tok.span.start());
            if (self.visitor)(command, head, context) {
                return true;
            }

            // A command substitution is live wherever it appears in a bare
            // or quoted word, including a command's own head.  Braced words
            // are intentionally excluded by `command_substitution_regions`.
            if self.live_substitutions(command, depth, grammar, context) {
                return true;
            }

            if self.analysis.is_some() {
                if self.original_source_regions(command, depth, definition_members.as_ref()) {
                    return true;
                }
                continue;
            }
            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            let member = grammar.filter(|g| is_member(g, head.written));
            let body_indices = member.map_or_else(
                || {
                    self.registry
                        .arg_indices_for_role(head.resolved, &args, ArgRole::Body)
                },
                |g| member_body_indices_in(g, head.written, &args, self.availability),
            );
            let next_grammar = next_definition_grammar(head, &args, grammar, self.registry);
            let case_list = self
                .registry
                .case_invocation(head.resolved, &args, self.availability)
                .and_then(|(spec, invocation)| invocation.clause_list_index.map(|i| (spec, i)));

            // A case-list descriptor owns its nested scripts even when the
            // outer list word has no generic `ArgRole::Body`.  In particular,
            // Expect's descriptor supplies its clause bodies directly rather
            // than duplicating that grammar as a flat argument role.
            if let Some((spec, case_index)) = case_list
                && !self.script_is_reference_only(head.resolved, &args, case_index)
                && let (Some(&token), Some(list)) =
                    (command.arg_tokens().get(case_index), args.get(case_index))
                && token.kind == TokenType::Str
                && self.case_list_regions(token, list, &spec, depth, next_grammar)
            {
                return true;
            }

            for index in body_indices {
                if self.script_is_reference_only(head.resolved, &args, index) {
                    continue;
                }
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                if case_list.is_some_and(|(_, case_index)| case_index == index) {
                    continue;
                }
                let single_token = command
                    .arg_single_token()
                    .get(index)
                    .copied()
                    .unwrap_or(false);
                if let Some((body_start, body_end)) =
                    literal_body_region(self.source, token, single_token)
                    && self.region(
                        body_start,
                        body_end,
                        depth + 1,
                        next_grammar,
                        ExecutableContext::PotentialBody,
                    )
                {
                    return true;
                }
            }

            for index in
                self.registry
                    .arg_indices_for_role(head.resolved, &args, ArgRole::LambdaLiteral)
            {
                if self.script_is_reference_only(head.resolved, &args, index) {
                    continue;
                }
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                let Some(body) = split_lambda_literal(self.source, token)
                    .and_then(|elements| elements.braced_body())
                else {
                    continue;
                };
                if self.region(
                    body.start() as usize,
                    body.end() as usize,
                    depth + 1,
                    None,
                    ExecutableContext::PotentialBody,
                ) {
                    return true;
                }
            }
        }
        false
    }

    fn original_source_regions(
        &mut self,
        command: &SegmentedCommand,
        depth: u32,
        definition_members: Option<
            &tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion,
        >,
    ) -> bool {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        let Some(analysis) = self.analysis else {
            return false;
        };
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        let context = input.context_registry();
        if let Some(members) = definition_members {
            if let Some(words) = command
                .argv
                .first()
                .and_then(|head| members.original_command_words_at(head.span.start()))
                && let Some(bodies) = members.script_bodies(words)
            {
                for body in bodies {
                    if self.original_region(
                        body.content_span(),
                        depth + 1,
                        body.definition_parent().cloned(),
                        ExecutableContext::PotentialBody,
                    ) {
                        return true;
                    }
                }
                return false;
            }
        }
        if let Some(words) =
            tcl_compiler::registry_invocation::source_structure::source_registry_words(
                self.source,
                analysis,
                command,
            )
        {
            for body in words.source_script_bodies_for(
                &context,
                tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            ) {
                let parent = body.definition_parent_for(&context, self.definition_parent.as_ref());
                if self.original_region(
                    body.content_span(),
                    depth + 1,
                    parent,
                    ExecutableContext::PotentialBody,
                ) {
                    return true;
                }
            }
            if let Some(bodies) = words.source_expression_script_bodies(input) {
                for body in bodies {
                    if self.original_region(
                        body.content_span(),
                        depth + 1,
                        self.definition_parent.clone(),
                        ExecutableContext::PotentialBody,
                    ) {
                        return true;
                    }
                }
            }
            return self.original_lambda_regions(&words, &context, depth);
        }
        if let Some(declared) =
            tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                self.source,
                analysis,
                command,
            )
        {
            for body in declared.source_script_bodies_for(
                tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            ) {
                if self.original_region(
                    body.content_span(),
                    depth + 1,
                    None,
                    ExecutableContext::PotentialBody,
                ) {
                    return true;
                }
            }
        }
        false
    }

    fn original_lambda_regions(
        &mut self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        context: &tcl_registry::model::ContextRegistry,
        depth: u32,
    ) -> bool {
        let executable = words
            .with_source_schema(context, |schema| schema.authored_source_script_arguments())
            .flatten()
            .unwrap_or_default();
        for (index, role) in words.roles().unwrap_or_default() {
            if *role != ArgRole::LambdaLiteral || !executable.contains(index) {
                continue;
            }
            let Some(word) = words
                .operands()
                .get(*index)
                .and_then(Option::as_ref)
                .and_then(|operand| operand.word())
            else {
                continue;
            };
            let Some(body) =
                split_original_lambda_literal(word).and_then(|fields| fields.braced_body())
            else {
                continue;
            };
            if self.original_region(body, depth + 1, None, ExecutableContext::PotentialBody) {
                return true;
            }
        }
        false
    }

    fn live_substitutions(
        &mut self,
        command: &SegmentedCommand,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
        context: ExecutableContext,
    ) -> bool {
        if self.analysis.is_some() {
            let image = tcl_lexer::SourceImage::document(self.source);
            let Ok(plan) = tcl_lexer::native_script_words_in(image, command.span, self.config)
            else {
                return false;
            };
            for command in plan.commands {
                for word in command.words {
                    for part in word.executable_parts().all_parts() {
                        if let tcl_lexer::ExecutablePart::Command { body } = part.part
                            && self.original_region(
                                body,
                                depth + 1,
                                self.definition_parent.clone(),
                                context,
                            )
                        {
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        command.argv.iter().any(|token| {
            command_substitution_regions(self.source, self.config, *token)
                .into_iter()
                .any(|(start, end)| self.region(start, end, depth + 1, grammar, context))
        })
    }

    fn script_is_reference_only(&self, head: &str, args: &[&str], index: usize) -> bool {
        self.registry
            .script_timing(head, args, index, self.availability)
            == Some(ScriptTiming::ReferenceOnly)
    }

    fn case_list_regions(
        &mut self,
        token: Token,
        list: &str,
        spec: &tcl_registry::CaseListSpec,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
    ) -> bool {
        // The compiler-owned flattener is the source-coordinate bridge between
        // a Tcl list element and a source token.  Besides braced actions, it
        // marks a substitution-free quoted action as `Str`; dynamic and
        // backslash-built actions remain opaque, so this traversal cannot
        // invent a statically executable body.
        for (_, (body_text, body_token)) in tcl_compiler::segmenter::flatten_case_list_clauses(
            self.source,
            list,
            token,
            spec,
            self.config,
        ) {
            if spec.fallthrough_body == Some(body_text.as_str()) {
                continue;
            }
            let Some((start, end)) = case_action_region(self.source, body_token) else {
                continue;
            };
            if self.region(
                start,
                end,
                depth + 1,
                grammar,
                ExecutableContext::PotentialBody,
            ) {
                return true;
            }
        }
        false
    }
}

/// Return a statically executable case action's verbatim script region.
///
/// [`tcl_compiler::segmenter::flatten_case_list_clauses`] only gives a
/// `TokenType::Str` body token to an action that it proved literal.  List
/// elements may still be braced, quoted, or bare, so strip their opening
/// delimiter here while retaining the segmenter's absolute spans.  The list
/// parser's token end is already the byte at the closing delimiter.
fn case_action_region(source: &str, token: Token) -> Option<(usize, usize)> {
    if token.kind != TokenType::Str {
        return None;
    }
    let start = token.span.start() as usize;
    let end = token.span.end() as usize;
    if start >= end || end > source.len() {
        return None;
    }
    let bytes = source.as_bytes();
    let start = start + usize::from(matches!(bytes.get(start), Some(b'{' | b'"')));
    // Empty actions are still executable regions: the cursor between their
    // delimiters must leave the containing `switch`/`expect` signature before
    // the user types the action's first command.
    (start <= end).then_some((start, end))
}

/// Return a registry-declared body's statically source-mappable script region.
///
/// Braced words are literal by Tcl definition. A substitution-free quoted
/// word is literal as well, provided it contains no backslash sequence whose
/// decoded value would differ from the written source. Compound quoted words
/// (`"puts $value"`, `"[build]"`) have multiple lexer fragments and are
/// deliberately withheld: reparsing their written source would invent a body
/// that does not exist until the enclosing command performs substitution.
fn literal_body_region(source: &str, token: Token, single_token: bool) -> Option<(usize, usize)> {
    match token.kind {
        TokenType::Str => braced_body_region(source, token),
        TokenType::Esc if single_token => quoted_literal_body_region(source, token),
        _ => None,
    }
}

/// Return a braced body's verbatim source content, excluding delimiters.
fn braced_body_region(source: &str, token: Token) -> Option<(usize, usize)> {
    let start = token.span.start() as usize + token.content_offset as usize;
    let raw_end = token.span.end() as usize;
    let bytes = source.as_bytes();
    let end = if raw_end > start && raw_end - start == 1 && bytes.get(raw_end - 1) == Some(&b'}') {
        start
    } else {
        raw_end
    };
    // Keep `{}` as a zero-length region. `begin_region` probes it before
    // declining to segment commands, so signature help cannot fall back to
    // the containing command while the caret awaits the body's first command.
    (start <= end && end <= source.len()).then_some((start, end))
}

/// Return a substitution-free quoted body's verbatim content.
fn quoted_literal_body_region(source: &str, token: Token) -> Option<(usize, usize)> {
    if token.kind != TokenType::Esc || token.content_offset != 1 {
        return None;
    }
    let raw_start = token.span.start() as usize;
    let raw_end = token.span.end() as usize;
    let bytes = source.as_bytes();
    if raw_start >= source.len() || raw_end > source.len() || bytes[raw_start] != b'"' {
        return None;
    }

    let start = raw_start + 1;
    // The shared quote scanner owns close semantics (including escaped quotes
    // and nested command substitutions). An unterminated opening token runs
    // through EOF, which is also the provisional inner end signature help
    // needs while the closing quote is still being typed.
    let end = close_quote_offset(source, raw_start)
        .map_or_else(|| (raw_end == source.len()).then_some(raw_end), Some)?;
    let content = source.get(start..end)?;
    (!content.as_bytes().contains(&b'\\')).then_some((start, end))
}

/// Locate active bracket substitutions inside one token, preserving absolute
/// offsets.  The segmenter coalesces compound bare/quoted words into `Esc`,
/// so those are re-lexed to recover every embedded `Cmd` fragment.
pub(crate) fn command_substitution_regions(
    source: &str,
    config: LexerConfig,
    token: Token,
) -> Vec<(usize, usize)> {
    let start = token.span.start() as usize;
    let end = token.span.end() as usize;
    if start >= end || end > source.len() {
        return Vec::new();
    }
    let strip = |fragment_start: usize, fragment_end: usize| {
        let inner_start =
            fragment_start + usize::from(source.as_bytes().get(fragment_start) == Some(&b'['));
        let inner_end = fragment_end
            - usize::from(
                fragment_end > inner_start
                    && source.as_bytes().get(fragment_end - 1) == Some(&b']'),
            );
        (inner_start, inner_end)
    };
    match token.kind {
        TokenType::Cmd => vec![strip(start, end)],
        TokenType::Esc if source.as_bytes()[start..end].contains(&b'[') => {
            let Ok(tokens) =
                Lexer::with_source_map(SourceMap::new(&source[start..end]), config).tokenise_all()
            else {
                return Vec::new();
            };
            tokens
                .into_iter()
                .filter(|token| token.kind == TokenType::Cmd)
                .map(|token| {
                    strip(
                        start + token.span.start() as usize,
                        start + token.span.end() as usize,
                    )
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visited_format_heads(source: &str, dialect: &str) -> Vec<(String, u32, String)> {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let mut heads = Vec::new();
        visit_analysis_executable_commands(source, &analysis, &mut |command, identity, context| {
            if command.name() == "format" || command.name() == "fmt" {
                assert_eq!(context, ExecutableContext::PotentialBody);
                // Original potential source has a written head and whole
                // command span, independently of entered-frame lookup.
                heads.push((
                    identity.written.to_owned(),
                    command.span.start(),
                    source[command.span.start() as usize..command.span.end() as usize].to_owned(),
                ));
            }
            false
        });
        heads
    }

    #[test]
    fn quoted_case_actions_retain_absolute_spans_for_each_registry_descriptor() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        for (source, dialect, written) in [
            ("switch $x {a \"format {%d} 1\"}", "tcl8.6", "format"),
            ("expect {-re {ready} \"format {%d} 1\"}", "expect", "format"),
        ] {
            let heads = visited_format_heads(source, dialect);
            assert_eq!(
                heads,
                vec![(
                    written.to_owned(),
                    u32::try_from(source.rfind(written).expect("nested head")).unwrap(),
                    "format {%d} 1".to_owned(),
                ),],
                "quoted case action must preserve its whole-document command span: {source}"
            );
        }
    }

    #[test]
    fn quoted_case_actions_preserve_identity_and_abstain_for_dynamic_or_malformed_lists() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        let aliased = "interp alias {} fmt {} format\nswitch $x {a \"fmt {%d} 1\"}";
        assert_eq!(
            visited_format_heads(aliased, "tcl8.6"),
            vec![(
                "fmt".to_owned(),
                u32::try_from(aliased.rfind("fmt").expect("nested alias")).unwrap(),
                "fmt {%d} 1".to_owned(),
            )]
        );

        let renamed = "rename format saved\nswitch $x {a \"format {%d} 1\"}";
        assert_eq!(
            visited_format_heads(renamed, "tcl8.6"),
            vec![(
                "format".to_owned(),
                u32::try_from(renamed.rfind("format").expect("nested head")).unwrap(),
                "format {%d} 1".to_owned(),
            )],
            "a renamed head retains only its written potential source; lookup is a separate question"
        );

        for source in [
            "set actions {a \"format {%d} 1\"}\nswitch $x $actions",
            "switch $x {a \"format {%d} 1\" orphan}",
        ] {
            assert!(
                visited_format_heads(source, "tcl8.6").is_empty(),
                "dynamic and malformed lists must not expose nested actions: {source}"
            );
        }
    }

    #[test]
    fn quoted_declared_bodies_recurse_only_when_source_mappable() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        let dialect = "tcl8.6";
        let literal = "proc p {} \"format {%d} 1\"";
        assert_eq!(
            visited_format_heads(literal, dialect),
            vec![(
                "format".to_owned(),
                u32::try_from(literal.find("format").expect("nested head")).unwrap(),
                "format {%d} 1".to_owned(),
            )],
            "a complete substitution-free quoted body retains its potential source span",
        );

        let unterminated = "proc p {} \"format {%d} 1";
        assert!(
            visited_format_heads(unterminated, dialect).is_empty(),
            "an incomplete quoted operand cannot issue a complete potential body; cursor recovery is a separate purpose",
        );

        for source in [
            "proc p {} \"format $pattern 1\"",
            "proc p {} \"format\\ {%d} 1\"",
        ] {
            assert!(
                visited_format_heads(source, dialect).is_empty(),
                "a substituted or backslash-decoded body is not source-mappable: {source}",
            );
        }
    }

    #[test]
    fn original_analysis_regions_keep_alias_case_and_shadow_source_ownership() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        for source in [
            "interp alias {} choose {} switch\nchoose x {x {format live}}",
            "if 1 {format live}",
            "apply {{} {format live}}",
            "expr {[format live]}",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            let mut nested = 0;
            visit_analysis_executable_commands(source, &analysis, &mut |command, _, context| {
                if command.name() == "format" && context == ExecutableContext::PotentialBody {
                    nested += 1;
                }
                false
            });
            assert_eq!(nested, 1, "{source}");
        }
        let source = "proc if args {}\nif 1 {format inert}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut nested = 0;
        visit_analysis_executable_commands(source, &analysis, &mut |command, _, _| {
            if command.name() == "format" {
                nested += 1;
            }
            false
        });
        assert_eq!(nested, 0);
    }

    #[test]
    fn original_analysis_regions_decline_stale_source_config_and_registry() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        let source = "if 1 {format live}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let registry = analysis.resolved_registry().unwrap();
        let cursor = source.find("live").unwrap();
        assert!(
            innermost_analysis_executable_region_at(source, &analysis, registry, cursor).is_some()
        );
        assert!(
            innermost_analysis_executable_region_at(
                "if 1 {format stale}",
                &analysis,
                registry,
                cursor
            )
            .is_none()
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(
            innermost_analysis_executable_region_at(source, &stale, registry, cursor).is_none()
        );
        let mut changed = CommandRegistry::build_default();
        changed.insert(tcl_registry::CommandSpec {
            name: "foreign-command",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        assert!(
            innermost_analysis_executable_region_at(source, &analysis, &changed, cursor).is_none()
        );
    }

    #[test]
    fn retained_analysis_regions_respect_reference_only_script_purpose() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        fn reference_only_first(
            _args: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, ScriptTiming)> {
            vec![(0, ScriptTiming::ReferenceOnly)]
        }
        let source = "reference-script {format reference}\nrun-script {format potential}";
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "reference-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            script_timing_resolver: Some(reference_only_first),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "run-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let generation = tcl_registry::model::context_for_profile(profile);
        let context =
            std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(registry)));
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl8.6");
        let mut visited = Vec::new();
        visit_analysis_executable_commands(source, &analysis, &mut |command, _, scope| {
            if command.name() == "format" {
                visited.push((command.span.start(), scope));
            }
            false
        });
        assert_eq!(
            visited,
            vec![(
                u32::try_from(source.rfind("format").unwrap()).unwrap(),
                ExecutableContext::PotentialBody
            )]
        );
        let segment = segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words =
            crate::original_invocation::source_registry_words(source, &analysis, &segment).unwrap();
        assert_eq!(
            words.source_script_bodies(&context).len(),
            1,
            "reference syntax remains available to navigation"
        );
        assert!(words.source_script_bodies_for(&context, tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation).is_empty());
    }

    #[test]
    fn original_analysis_definition_vocabulary_clears_ordinary_and_procedure_bodies() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        for (source, expected) in [
            (
                "oo::class create C {if 1 {method café {} {format live}}}",
                1,
            ),
            (
                "oo::class create C {self self {method café {} {format live}}}",
                1,
            ),
            (
                "oo::class create C {proc p {} {method ordinary {} {format inert}}}",
                0,
            ),
            (
                "oo::class create C {method m {} {method ordinary {} {format inert}}}",
                0,
            ),
            (
                "oo::class create C {apply {{} {method ordinary {} {format inert}}}}",
                0,
            ),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            let mut visited = Vec::new();
            visit_analysis_executable_commands(source, &analysis, &mut |command, _, context| {
                if command.name() == "format" {
                    visited.push((command.span.start(), context));
                }
                false
            });
            assert_eq!(visited.len(), expected, "{source}");
            for (start, context) in visited {
                assert_eq!(
                    source.get(start as usize..start as usize + 6),
                    Some("format")
                );
                assert_eq!(context, ExecutableContext::PotentialBody);
            }
        }
    }

    #[test]
    fn original_analysis_member_regions_keep_parent_vocabulary_and_whole_geometry() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        for (source, dialect, expected) in [
            (
                "oo::class create C {method café {} {format live}}",
                "tcl8.6",
                1,
            ),
            (
                "oo::class create C {self self {method m {} {format live}}}",
                "tcl8.6",
                1,
            ),
            (
                "oo::class create C {method m -private {} {format live}}",
                "tcl9.0",
                1,
            ),
            (
                "oo::class create C {method m -private {} {format hidden}}",
                "tcl8.6",
                0,
            ),
            (
                "oo::class create C {$member m {} {format hidden}}",
                "tcl8.6",
                0,
            ),
            (
                "oo::class create C {method m $params {format hidden}}",
                "tcl8.6",
                0,
            ),
            (
                r#"oo::class create C {method m {} "\u0066ormat hidden"}"#,
                "tcl8.6",
                0,
            ),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            let mut visited = Vec::new();
            visit_analysis_executable_commands(source, &analysis, &mut |command, _, context| {
                if command.name() == "format" {
                    visited.push((command.span.start(), context));
                }
                false
            });
            assert_eq!(visited.len(), expected, "{source}");
            for (start, context) in visited {
                assert_eq!(
                    usize::try_from(start).unwrap(),
                    source.find("format").unwrap()
                );
                assert_eq!(context, ExecutableContext::PotentialBody);
            }
        }
        let source = "oo::class create C {method m {} prefix[format live]suffix}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut visited = Vec::new();
        visit_analysis_executable_commands(source, &analysis, &mut |command, _, _| {
            if command.name() == "format" {
                visited.push(command.span.start());
            }
            false
        });
        assert_eq!(
            visited,
            vec![u32::try_from(source.find("format").unwrap()).unwrap()],
            "live compound-word substitution keeps whole original word geometry"
        );
    }

    #[test]
    fn original_analysis_declared_body_roles_keep_syntax_separate_from_timing() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {script:body}\n# tcl-lsp: stubs-end\nhold {format source-only}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let command = segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words =
            tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                source, &analysis, &command,
            )
            .unwrap();
        assert_eq!(
            words
                .source_script_bodies_for(
                    tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::Syntax
                )
                .len(),
            1
        );
        assert!(words.source_script_bodies_for(tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation).is_empty());
        let mut visited = Vec::new();
        visit_analysis_executable_commands(source, &analysis, &mut |command, _, _| {
            if command.name() == "format" {
                visited.push(command.span.start());
            }
            false
        });
        assert!(
            visited.is_empty(),
            "a declaration without timing cannot acquire potential evaluation"
        );
    }

    #[test]
    fn reference_only_bodies_are_not_executable_regions() {
        // naming.core.original-executable-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
        fn reference_only_first(
            _args: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, ScriptTiming)> {
            vec![(0, ScriptTiming::ReferenceOnly)]
        }

        let source = "remove-script {frame .ghost}\nproc shown {} {frame .shown}";
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "remove-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            script_timing_resolver: Some(reference_only_first),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let base = tcl_registry::model::context_for_profile(profile);
        let context = std::sync::Arc::new(base.with_command_store(std::sync::Arc::new(registry)));
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl8.6");
        let mut frame_spans = Vec::new();
        visit_analysis_executable_commands(
            source,
            &analysis,
            &mut |command, _identity, context| {
                if command.name() == "frame" {
                    assert_eq!(context, ExecutableContext::PotentialBody);
                    frame_spans.push(command.span.start());
                }
                false
            },
        );

        assert_eq!(
            frame_spans,
            vec![u32::try_from(source.rfind("frame").expect("live body")).unwrap()],
            "a reference-only script is matched metadata, not potential execution"
        );
    }
}
