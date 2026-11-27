// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The lexer-confirmed comments of every registry-reachable script: the walker over bodies, case lists, lambda literals and substitutions.

use std::collections::HashSet;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::{Span, TokenType};
use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::{ArgRole, CommandRegistry};

/// Physical source comments under the actual complete analysis and Registry.
/// Authored Body, member/case/lambda geometry stays source syntax only; this
/// does not enter a frame, choose runtime dispatch or grant an edit.
#[must_use]
pub fn script_comment_facts_from_analysis(
    source: &str,
    analysis: &super::super::AnalysisResult,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    CommentLineWalker::from_analysis(source, analysis)?.collect_original()
}

/// Lexer-confirmed comments at every original command position, including
/// inline `;#` comments, under the actual complete analysis and Registry.
/// Source-body selection remains conditional syntax; this supplies no entered
/// body, execution, complete command census or source-edit authority.
#[must_use]
pub fn script_command_comment_facts_from_analysis(
    source: &str,
    analysis: &super::super::AnalysisResult,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let mut walker = CommentLineWalker::from_analysis(source, analysis)?;
    walker.comment_position = ScriptCommentPosition::CommandPosition;
    walker.collect_original()
}

/// One lexer-confirmed command-position comment in a registry-reachable script.
///
/// `span` is an absolute byte range in the whole document and includes the
/// leading `#`. Physical-line queries retain only markers preceded by horizontal
/// whitespace on that line; the command-position query also retains inline
/// comments. The exact slice carries the selected lexical purpose and never
/// turns a quoted or braced data argument into script.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptCommentFact {
    /// Absolute lexer-confirmed comment-token span, including `#`.
    pub span: Span,
    /// Zero-based physical line on which the `#` begins.
    pub line: usize,
    /// Exact lexer-confirmed comment text, including `#` and any continued
    /// physical lines. Consumers must inspect this instead of rescanning the
    /// source line that happens to contain the comment.
    pub text: String,
}

/// Lexer-confirmed physical comments in the document's top-level script or a
/// registry-declared script body. Braced data arguments are never recursively
/// tokenised as scripts. Callers must pass the document's resolved lexer
/// configuration and registry; there is deliberately no implicit Tcl profile.
#[must_use]
pub fn script_comment_facts(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ScriptCommentFact> {
    // Every retained physical comment includes its original marker byte.
    // An absent marker closes this inventory without selecting body roles.
    if !source.as_bytes().contains(&b'#') {
        return Vec::new();
    }
    CommentLineWalker::new(source, config, registry).collect()
}

/// Physical comment tokens in explicitly supplied original script regions.
///
/// The caller owns each region's source-syntax applicability. This lexical
/// projection does not select command roles, reconstruct an analysis, prove
/// execution or authorise an edit. Regions and comment slices remain in the
/// supplied image's original byte coordinates; cooked values have no mapping.
/// Unknown text, geometry or lexing returns `None`, never an empty inventory.
#[must_use]
pub fn original_script_comment_facts(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<ScriptCommentFact>> {
    original_script_comment_facts_for_position(
        image,
        scripts,
        config,
        ScriptCommentPosition::PhysicalLine,
    )
}

/// Command-position comment tokens, inline `;#` comments included, in
/// explicitly supplied original script regions. Otherwise exactly
/// [`original_script_comment_facts`]: the caller owns each region's
/// applicability and nothing is interpreted.
#[must_use]
pub fn original_script_command_comment_facts(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<ScriptCommentFact>> {
    original_script_comment_facts_for_position(
        image,
        scripts,
        config,
        ScriptCommentPosition::CommandPosition,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptCommentPosition {
    PhysicalLine,
    CommandPosition,
}

fn original_script_comment_facts_for_position(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
    position: ScriptCommentPosition,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = image.try_text().ok()?;
    let source_map = image.source_map();
    let mut facts = Vec::new();
    for &(region, depth) in scripts {
        let script = source.get(region.as_range())?;
        let tokens = tcl_lexer::Lexer::with_config(script, config.normalized().at_depth(depth))
            .tokenise_all()
            .ok()?;
        for token in tokens
            .into_iter()
            .filter(|token| token.kind == TokenType::Comment)
        {
            let span = Span::new(
                region.start().checked_add(token.span.start())?,
                region.start().checked_add(token.span.end())?,
            );
            let start = usize::try_from(span.start()).ok()?;
            let before = source.get(..start)?;
            let line_start = before.rfind('\n').map_or(0, |offset| offset + 1);
            if position == ScriptCommentPosition::PhysicalLine
                && !source
                    .get(line_start..start)?
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c))
            {
                continue;
            }
            facts.push(ScriptCommentFact {
                span,
                line: usize::try_from(source_map.line_index().line_at(span.start())).ok()?,
                text: source.get(span.as_range())?.to_owned(),
            });
        }
    }
    facts.sort_unstable_by_key(|fact| (fact.span.start(), fact.span.end()));
    facts.dedup_by_key(|fact| fact.span);
    Some(facts)
}

/// Lexer-fact walk over every registry-declared script shape.  This is shared
/// by `# noqa`, W115, and code actions so neither feature can accidentally
/// invent a separate idea of where executable comment lines may occur.
struct CommentLineWalker<'a> {
    whole: &'a str,
    pub(super) config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    identities: std::borrow::Cow<'a, crate::realm::CommandBindingRealm>,
    source_context: Option<
        std::sync::Arc<crate::registry_invocation::source_structure::OriginalSourceRegistryContext>,
    >,
    analysis: Option<&'a super::super::AnalysisResult>,
    availability: Option<SurfaceQuery<'a>>,
    visited: HashSet<(u32, u32)>,
    pub(super) facts: Vec<ScriptCommentFact>,
    comment_position: ScriptCommentPosition,
}

impl<'a> CommentLineWalker<'a> {
    pub(super) fn new(
        whole: &'a str,
        config: tcl_lexer::LexerConfig,
        registry: &'a CommandRegistry,
    ) -> Self {
        let source_context = registry.profile().filter(|profile| {
            tcl_native_model::InvocationDialect::of_profile(profile).authored_name_policy().is_some()
        }).map(|profile| {
            let generation = crate::environment_ingress::context_for_profile(profile);
            let context = std::sync::Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
            let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
            std::sync::Arc::new(crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(whole, input))
        });
        let identities = source_context.as_ref().map_or_else(
            || crate::realm::document_realm_bindings_with_config(whole, config, registry),
            |source| source.retained_source_realm().clone(),
        );
        Self {
            whole,
            config,
            registry,
            identities: std::borrow::Cow::Owned(identities),
            source_context,
            analysis: None,
            availability: registry
                .profile()
                .map(tcl_dialect::DialectProfile::surface_query),
            visited: HashSet::new(),
            facts: Vec::new(),
            comment_position: ScriptCommentPosition::PhysicalLine,
        }
    }

    fn from_analysis(whole: &'a str, analysis: &'a super::super::AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let input = analysis.resolved_input.as_ref()?;
        let image = tcl_lexer::SourceImage::document(whole);
        analysis
            .matches_original_source_image(&image, config)
            .then_some(())?;
        Some(Self {
            whole,
            config,
            registry: analysis.resolved_registry()?,
            identities: std::borrow::Cow::Borrowed(analysis.retained_command_realm()?),
            source_context: None,
            analysis: Some(analysis),
            availability: Some(input.availability_context().authoring_query()),
            visited: HashSet::new(),
            facts: Vec::new(),
            comment_position: ScriptCommentPosition::PhysicalLine,
        })
    }

    fn collect_original(mut self) -> Option<Vec<ScriptCommentFact>> {
        if !self.whole.as_bytes().contains(&b'#') {
            return Some(Vec::new());
        }
        let end = u32::try_from(self.whole.len()).ok()?;
        self.visit_original(Span::new(0, end), 0, None)?;
        self.facts
            .sort_unstable_by_key(|fact| (fact.span.start(), fact.span.end()));
        self.facts.dedup_by_key(|fact| fact.span);
        Some(self.facts)
    }

    fn visit_original(
        &mut self,
        region: Span,
        depth: u32,
        definition_parent: Option<&crate::registry_invocation::OriginalSourceScriptBody>,
    ) -> Option<()> {
        if depth > 256 || !self.visited.insert((region.start(), region.end())) {
            return Some(());
        }
        let analysis = self.analysis?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let image = tcl_lexer::SourceImage::document(self.whole);
        self.facts
            .extend(original_script_comment_facts_for_position(
                &image,
                &[(region, depth)],
                self.config,
                self.comment_position,
            )?);
        let script = self.whole.get(region.as_range())?;
        let commands = tcl_source::segment::segment_commands_with_offset_and_config(
            script,
            region.start(),
            self.config.at_depth(depth),
        );
        let plan = tcl_lexer::native_script_words_in(image, region, self.config).ok()?;
        let definition_members =
            definition_parent.and_then(|parent| parent.definition_member_region(&context, region));
        for native in &plan.commands {
            for word in &native.words {
                for part in word.executable_parts().all_parts() {
                    if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                        self.visit_original(body, depth + 1, definition_parent)?;
                    }
                }
            }
            if let Some(members) = &definition_members
                && let Some(bodies) = members.script_bodies(&native.words)
            {
                for body in bodies {
                    self.visit_original(body.content_span(), depth + 1, body.definition_parent())?;
                }
                continue;
            }
            let Some(command) = commands.iter().find(|command| {
                command.argv.first().map(|token| token.span.start())
                    == native.words.first().map(|word| word.span().start())
            }) else {
                continue;
            };
            if let Some(declared) =
                crate::registry_invocation::source_structure::source_declared_command_words(
                    self.whole, analysis, command,
                )
            {
                for body in declared.source_script_bodies_for(
                    crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
                ) {
                    self.visit_original(body.content_span(), depth + 1, None)?;
                }
                continue;
            }
            let Some(words) = crate::registry_invocation::source_structure::source_registry_words(
                self.whole, analysis, command,
            ) else {
                continue;
            };
            for body in words.source_script_bodies(&context) {
                let parent = body.definition_parent_for(&context, definition_parent);
                self.visit_original(body.content_span(), depth + 1, parent.as_ref())?;
            }
            if let Some(bodies) =
                words.source_expression_script_bodies(analysis.resolved_input.as_ref()?)
            {
                for body in bodies {
                    self.visit_original(body.content_span(), depth + 1, definition_parent)?;
                }
            }
            for &(ordinal, role) in words.roles().unwrap_or_default() {
                if role != ArgRole::LambdaLiteral {
                    continue;
                }
                let Some(word) = words
                    .operands()
                    .get(ordinal)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                else {
                    continue;
                };
                if let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                    .and_then(|lambda| lambda.braced_body())
                {
                    self.visit_original(body, depth + 1, None)?;
                }
            }
        }
        Some(())
    }

    pub(super) fn collect(mut self) -> Vec<ScriptCommentFact> {
        self.visit(self.whole, 0, 0, None);
        self.facts.sort_unstable_by_key(|fact| fact.span.start());
        self.facts.dedup_by_key(|fact| fact.span);
        self.facts
    }

    fn visit(
        &mut self,
        script: &'a str,
        base_offset: u32,
        depth: u32,
        definition_grammar: Option<&'static DefinitionBodyGrammar>,
    ) {
        if depth > 64
            || !self
                .visited
                .insert((base_offset, u32::try_from(script.len()).unwrap_or(u32::MAX)))
        {
            return;
        }
        self.record_comments(script, base_offset, depth);
        for command in tcl_source::segment::segment_commands_with_offset_and_config(
            script,
            0,
            self.config.at_depth(depth),
        ) {
            if self.source_context.is_some() && definition_grammar.is_none() {
                self.visit_source_schema(&command, base_offset, depth);
                self.visit_command_substitutions(script, &command, base_offset, depth);
                continue;
            }
            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            // Member keywords are syntax within an already selected definition
            // grammar. They do not establish a Tcl command implementation.
            let name = command.name();
            let literal_head = command.single_token_word.first() == Some(&true)
                && command
                    .argv
                    .first()
                    .is_some_and(|token| matches!(token.kind, TokenType::Str | TokenType::Esc));
            let surface = tcl_registry::model::DocumentCommandSurface::new(self.registry, None);
            let view = crate::registry_invocation::segmented_body_assistance(
                &surface,
                self.identities.source_bindings_ref(),
                script,
                self.config.at_depth(depth),
                &command,
                base_offset,
                self.identities.source_metadata_context().as_deref(),
            );
            let roles = view
                .as_ref()
                .map_or(&[][..], |view| view.possible_roles.as_slice());
            let member = definition_grammar
                .filter(|_| literal_head)
                .and_then(|grammar| grammar.member(name));
            let body_indices = member.map_or_else(
                || {
                    roles
                        .iter()
                        .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
                        .collect()
                },
                |_| {
                    definition_grammar
                        .expect("member has grammar")
                        .member_body_indices_in(name, &args, self.availability)
                },
            );
            let next_grammar = next_definition_grammar(
                view.as_ref()
                    .map_or(&[], |view| view.possible_definition_grammars.as_slice()),
                &args,
                definition_grammar,
                member,
            );
            let case_lists = view
                .as_ref()
                .map_or(&[][..], |view| view.possible_case_lists.as_slice());
            for &(spec, index) in case_lists {
                if let Some(&token) = command.arg_tokens().get(index)
                    && token.kind == tcl_lexer::TokenType::Str
                {
                    self.visit_case_list(script, token, &spec, base_offset, depth);
                }
            }

            for index in body_indices {
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                if token.kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                if case_lists
                    .iter()
                    .any(|&(_, case_index)| case_index == index)
                {
                    continue;
                }
                self.visit_braced_word(script, token, base_offset, depth, next_grammar);
            }
            self.visit_lambda_literals(script, &command, roles, base_offset, depth);
            self.visit_command_substitutions(script, &command, base_offset, depth);
        }
    }

    /// Readonly source roles select physical script geometry. Neither unknown
    /// argument values nor an unavailable execution entry turn a body into data.
    fn visit_source_schema(
        &mut self,
        command: &tcl_source::segment::SegmentedCommand,
        base_offset: u32,
        depth: u32,
    ) {
        let Some(source) = &self.source_context else {
            return;
        };
        let command = command.clone().shifted_by(base_offset);
        let Some(words) = source
            .words(&command)
            .or_else(|| source.conditional_native_case_words(&command))
        else {
            return;
        };
        let context = source.editing_input().context_registry();
        let conditional_native = matches!(
            words.source(),
            crate::registry_invocation::source_structure::OriginalRegistrySource::ConditionalNativeSource(
                _
            )
        );
        let grammar = (!conditional_native)
            .then(|| {
                words
                    .with_source_schema(&context, |schema| {
                        context
                            .context()
                            .resolve_spec(context.commands(), schema.canonical_command)
                            .and_then(|spec| spec.definition_body)
                    })
                    .flatten()
            })
            .flatten();
        let mut children = if conditional_native {
            Vec::new()
        } else {
            words
                .source_script_bodies(&context)
                .into_iter()
                .map(|body| body.content_span())
                .collect::<Vec<_>>()
        };
        // Case candidates are syntax-only even when the subject/option
        // selection is unknown; retain that residual while visiting their
        // original spans, without treating any candidate as entered.
        if let Some(candidates) = words.conditional_source_case_bodies(&context)
            && candidates.matches_source(source.source_image(), self.config)
        {
            let _selection_unknown = candidates.selection_unknown();
            children.extend(candidates.bodies().iter().map(
                super::super::super::registry_invocation::OriginalSourceScriptBody::content_span,
            ));
        }
        for &(ordinal, role) in words.roles().unwrap_or_default() {
            if role != ArgRole::LambdaLiteral {
                continue;
            }
            let Some(word) = words
                .operands()
                .get(ordinal)
                .and_then(Option::as_ref)
                .and_then(|operand| operand.word())
            else {
                continue;
            };
            if let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                .and_then(|lambda| lambda.braced_body())
            {
                children.push(body);
            }
        }
        children.sort_unstable_by_key(|span| (span.start(), span.end()));
        children.dedup();
        for child in children {
            if let Some(script) = self.whole.get(child.as_range()) {
                self.visit(script, child.start(), depth.saturating_add(1), grammar);
            }
        }
    }

    fn record_comments(&mut self, script: &str, base_offset: u32, depth: u32) {
        let Some(end) = u32::try_from(script.len())
            .ok()
            .and_then(|length| base_offset.checked_add(length))
        else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(self.whole);
        if let Some(facts) = original_script_comment_facts(
            &image,
            &[(Span::new(base_offset, end), depth)],
            self.config,
        ) {
            self.facts.extend(facts);
        }
    }

    fn visit_braced_word(
        &mut self,
        script: &'a str,
        token: tcl_lexer::Token,
        base_offset: u32,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
    ) {
        let whole_word = tcl_lexer::word_span_at(script, token.span);
        let start = usize::try_from(whole_word.start()).unwrap_or(script.len());
        let end = usize::try_from(whole_word.end()).unwrap_or(script.len());
        if end <= start + 1 || script.as_bytes().get(start) != Some(&b'{') {
            return;
        }
        let Some(body) = script.get(start + 1..end - 1) else {
            return;
        };
        self.visit(
            body,
            base_offset.saturating_add(u32::try_from(start + 1).unwrap_or(u32::MAX)),
            depth.saturating_add(1),
            grammar,
        );
    }

    fn visit_case_list(
        &mut self,
        script: &'a str,
        token: tcl_lexer::Token,
        spec: &tcl_registry::CaseListSpec,
        base_offset: u32,
        depth: u32,
    ) {
        let content_start = token.span.start() as usize + token.content_offset as usize;
        let content_end = token.span.end() as usize;
        let Some(list) = script.get(content_start..content_end) else {
            return;
        };
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: spec.clause_flags,
            clause_value_flags: spec.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list(list, &shape) {
            let Some(body) = clause.body.filter(|body| body.braced) else {
                continue;
            };
            let range = body.content_range();
            let start = content_start + range.start;
            let end = content_start + range.end;
            let Some(arm) = script.get(start..end) else {
                continue;
            };
            self.visit(
                arm,
                base_offset.saturating_add(u32::try_from(start).unwrap_or(u32::MAX)),
                depth.saturating_add(1),
                None,
            );
        }
    }

    fn visit_lambda_literals(
        &mut self,
        script: &'a str,
        command: &tcl_source::segment::SegmentedCommand,
        roles: &[(usize, ArgRole)],
        base_offset: u32,
        depth: u32,
    ) {
        if let Some(source) = self.source_context.clone() {
            let command = command.clone().shifted_by(base_offset);
            let Some(words) = source.words(&command) else {
                return;
            };
            if !words.matches_source(source.source_image(), self.config) {
                return;
            }
            for &(ordinal, role) in words.roles().unwrap_or_default() {
                if role != ArgRole::LambdaLiteral {
                    continue;
                }
                let Some(word) = words
                    .operands()
                    .get(ordinal)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                else {
                    continue;
                };
                if word.image() != source.source_image() || word.config() != self.config {
                    continue;
                }
                let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                    .and_then(|lambda| lambda.braced_body())
                else {
                    continue;
                };
                if let Some(lambda) = self.whole.get(body.as_range()) {
                    self.visit(lambda, body.start(), depth.saturating_add(1), None);
                }
            }
            return;
        }
        for index in roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::LambdaLiteral).then_some(index))
        {
            let Some(&token) = command.arg_tokens().get(index) else {
                continue;
            };
            let Some(body) = crate::lambda_literal::split_lambda_literal(script, token)
                .and_then(|literal| literal.braced_body())
            else {
                continue;
            };
            let start = body.start() as usize;
            let end = body.end() as usize;
            let Some(lambda) = script.get(start..end) else {
                continue;
            };
            self.visit(
                lambda,
                base_offset.saturating_add(body.start()),
                depth.saturating_add(1),
                None,
            );
        }
    }

    /// Descend command-substitution tokens exactly as the CST segmented them.
    /// A braced word has no command-substitution token, so inert data remains
    /// opaque; bare and quoted words retain every executable bracketed span
    /// without a second source scan.
    fn visit_command_substitutions(
        &mut self,
        script: &'a str,
        command: &tcl_source::segment::SegmentedCommand,
        base_offset: u32,
        depth: u32,
    ) {
        for &token in &command.all_tokens {
            if token.kind != tcl_lexer::TokenType::Cmd {
                continue;
            }
            let start = token.span.start() as usize + token.content_offset as usize;
            let end = token.span.end() as usize;
            let Some(inner) = script.get(start..end) else {
                continue;
            };
            self.visit(
                inner,
                base_offset.saturating_add(
                    token
                        .span
                        .start()
                        .saturating_add(u32::from(token.content_offset)),
                ),
                depth.saturating_add(1),
                None,
            );
        }
    }
}

fn next_definition_grammar(
    possible: &[&'static DefinitionBodyGrammar],
    args: &[&str],
    current: Option<&'static DefinitionBodyGrammar>,
    member: Option<&'static tcl_registry::definer::MemberSpec>,
) -> Option<&'static DefinitionBodyGrammar> {
    if let [grammar] = possible {
        return Some(*grammar);
    }
    if !possible.is_empty() {
        return None;
    }
    let Some(member) = member else {
        return current;
    };
    if member.wrapper_block_body
        && args
            .first()
            .is_some_and(|inner| current.is_some_and(|grammar| grammar.member(inner).is_none()))
    {
        current
    } else {
        None
    }
}
