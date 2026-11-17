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

//! Script bodies selected from operands: geometry, cases, lambdas and expressions.

use super::{
    OriginalConditionalSourceCaseBodies, OriginalSourceScriptBody, OriginalSourceScriptPurpose,
};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::ArgRole;
use tcl_registry::model::ContextRegistry;

/// Source syntax body selected solely by a genuine authored declaration.
/// The declaration, complete original argv, full input and whole original body
/// remain retained. A Body role without timing grants no potential evaluation,
/// builtin traits, Native lookup, entered frame, completion or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredSourceScriptBody {
    pub(super) source: Arc<crate::command_binding::OriginalDeclaredCommandWords>,
    pub(super) container: NativeWord,
    pub(super) extent: Span,
    pub(super) content: Span,
}

impl OriginalDeclaredSourceScriptBody {
    /// Genuine complete source body word, never a cooked or expansion child.
    #[must_use]
    pub const fn original_container(&self) -> &NativeWord {
        &self.container
    }
    /// Original fold extent, excluding its closing delimiter.
    #[must_use]
    pub const fn extent_span(&self) -> Span {
        self.extent
    }
    /// Exact original script content without genuine word delimiters.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Authentic complete declaration and its conditional source obligations.
    #[must_use]
    pub fn source_words(&self) -> &crate::command_binding::OriginalDeclaredCommandWords {
        &self.source
    }
    /// Exact immutable source image and complete parser configuration.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.source.site().source.source_image() == image
            && self.source.resolved_input().lexer_config() == config
            && self.container.image() == image
            && self.container.config() == config
    }
    /// Complete retained availability and immutable Registry generation.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        let input = self.source.resolved_input().context_registry();
        input.context() == context.context()
            && input.commands().snapshot().semantic_key()
                == context.commands().snapshot().semantic_key()
    }
}

pub(crate) fn declared_source_script_bodies_for(
    words: &crate::command_binding::OriginalDeclaredCommandWords,
    purpose: OriginalSourceScriptPurpose,
) -> Vec<OriginalDeclaredSourceScriptBody> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    // Authored declarations specify source Body roles, but do not specify
    // script timing. Role syntax cannot establish potential evaluation.
    if purpose != OriginalSourceScriptPurpose::Syntax {
        return Vec::new();
    }
    let source = Arc::new(words.clone());
    let image = words.site().source.source_image();
    let config = words.resolved_input().lexer_config();
    words
        .supplied_argument_roles()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(argument, role)| {
            if role != ArgRole::Body {
                return None;
            }
            let word = words.argument_word(argument)?;
            if word.image() != image || word.config() != config {
                return None;
            }
            let geometry = original_word_script_body_geometry(word)?;
            Some(OriginalDeclaredSourceScriptBody {
                source: Arc::clone(&source),
                container: geometry.container,
                extent: geometry.extent,
                content: geometry.content,
            })
        })
        .collect()
}

/// Sealed readonly ancestry of a script region selected by an original hosted
/// role or case grammar. Parent lookup evidence never becomes a child lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceScriptBodyOrigin {
    pub(super) container: NativeWord,
    pub(super) content: Span,
    pub(super) parent: super::super::VendorRegistryInvocationShape,
    regions: Vec<Span>,
    barriers: super::super::VendorSourceCatalogueBarriers,
    obligations: Vec<crate::command_binding::VendorCatalogueSourceObligation>,
}

impl std::hash::Hash for OriginalSourceScriptBodyOrigin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.container, state);
        std::hash::Hash::hash(&self.content, state);
        std::hash::Hash::hash(self.parent.registry(), state);
    }
}

impl OriginalSourceScriptBodyOrigin {
    pub(crate) fn matches_child(&self, context: &ContextRegistry, original: &[NativeWord]) -> bool {
        self.parent.context() == context.context()
            && self.parent.registry() == &context.commands().snapshot().semantic_key()
            && self.barriers.matches_context(context)
            && !original.is_empty()
            && original.iter().all(|word| {
                word.image() == self.container.image()
                    && word.config() == self.container.config()
                    && word.span().start() >= self.content.start()
                    && word.span().end() <= self.content.end()
            })
    }
    pub(crate) fn merge(&self, other: &Self) -> Option<Self> {
        if self.container != other.container
            || self.content != other.content
            || self.parent != other.parent
            || self.regions != other.regions
            || self.barriers != other.barriers
        {
            return None;
        }
        let mut merged = self.clone();
        for obligation in &other.obligations {
            if !merged.obligations.contains(obligation) {
                merged.obligations.push(obligation.clone());
            }
        }
        Some(merged)
    }
    pub(crate) fn source_regions(&self) -> &[Span] {
        &self.regions
    }
    pub(crate) fn obligations(&self) -> &[crate::command_binding::VendorCatalogueSourceObligation] {
        &self.obligations
    }
}

impl OriginalSourceScriptBody {
    pub(crate) fn vendor_origin(&self) -> Option<std::sync::Arc<OriginalSourceScriptBodyOrigin>> {
        let super::super::source_structure::OriginalRegistrySource::Vendor(metadata) =
            self.source.source()
        else {
            return None;
        };
        let mut regions = metadata
            .catalogue()
            .script_body_origin()
            .map_or_default(|origin| origin.source_regions().to_vec());
        regions.push(self.content);
        Some(std::sync::Arc::new(OriginalSourceScriptBodyOrigin {
            regions,
            container: self.container.clone(),
            content: self.content,
            parent: metadata.shape().clone(),
            barriers: metadata.authored_barriers().clone(),
            obligations: metadata.obligations().to_vec(),
        }))
    }
}

#[derive(Clone)]
pub(super) struct ScriptBodyGeometry {
    pub(super) container: NativeWord,
    pub(super) extent: Span,
    pub(super) container_extent: Span,
    pub(super) content: Span,
}

pub(super) fn original_word_script_body_geometry(word: &NativeWord) -> Option<ScriptBodyGeometry> {
    let unchanged = match word.group().kind {
        tcl_lexer::WordKind::Braced => true,
        tcl_lexer::WordKind::Bare | tcl_lexer::WordKind::Quoted => {
            word.executable_parts().all_parts().all(|part| {
                matches!(
                    part.part,
                    tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Original)
                )
            })
        }
    };
    if word.group().expand || !unchanged {
        return None;
    }
    let content = word.content_span().ok()?;
    let extent = Span::new(word.group().span.start(), content.end());
    Some(ScriptBodyGeometry {
        container: word.clone(),
        extent,
        container_extent: extent,
        content,
    })
}

/// Original script content geometry; selected body roles and entry remain separate.
pub(crate) fn original_written_script_body_span(word: &NativeWord) -> Option<Span> {
    Some(original_word_script_body_geometry(word)?.content)
}

/// Lexical zero-command shape of one genuine written script word.
/// Selected body roles, timing, effects and execution remain independent.
pub(crate) fn original_written_script_body_is_empty(word: &NativeWord) -> Option<bool> {
    let geometry = original_word_script_body_geometry(word)?;
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        word.image().clone(),
        geometry.content,
        word.config(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    Some(plan.commands.is_empty() && plan.fatal_tail.is_none())
}

fn source_operand_body(
    operand: &super::super::source_structure::OriginalOperandSource,
) -> Option<ScriptBodyGeometry> {
    if let Some(word) = operand.word() {
        return original_word_script_body_geometry(word);
    }
    // A frozen expansion child supplies only its independently retained static
    // source extent and parent. It cannot become a fabricated whole body word.
    let input = operand.input()?;
    let container = input.original_static_list_container()?;
    let parent = container.parent_word();
    if parent.image().bytes().get(operand.span().as_range()) != Some(input.bytes()) {
        return None;
    }
    Some(ScriptBodyGeometry {
        container: parent.clone(),
        extent: operand.span(),
        container_extent: operand.span(),
        content: operand.span(),
    })
}

fn source_lambda_body(
    words: &super::super::source_structure::OriginalRegistryWords,
    registry_context: &ContextRegistry,
) -> Option<ScriptBodyGeometry> {
    let selected = words.with_source_schema(
        registry_context,
        crate::registry_invocation::source_schema_queries::lambda_call,
    )??;
    let word = words
        .operands()
        .get(selected.lambda_argument)?
        .as_ref()?
        .word()?;
    let parent = original_word_script_body_geometry(word)?;
    let value = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
    if word.image().bytes().get(parent.content.as_range())? != value.as_slice() {
        return None;
    }
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(word)?;
    if !(2..=3).contains(&fields.len()) {
        return None;
    }
    let body = fields.get(1)?;
    let content = body.source_span()?;
    if content.start() < parent.content.start()
        || content.end() > parent.content.end()
        || word.image().bytes().get(content.as_range())? != body.value().as_bytes()
    {
        return None;
    }
    Some(ScriptBodyGeometry {
        container: word.clone(),
        extent: content,
        container_extent: parent.extent,
        content,
    })
}

impl super::super::source_structure::OriginalRegistryWords {
    /// Exact unchanged lambda-list body geometry under its selected source schema.
    /// This separate projection supplies no caller frame, namespace, parameter
    /// binding, body entry or generic script-inventory coverage.
    #[must_use]
    pub fn original_lambda_body_geometry(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceScriptBody> {
        let body = source_lambda_body(self, context)?;
        Some(OriginalSourceScriptBody {
            source: Arc::new(self.clone()),
            container: body.container,
            extent: body.extent,
            container_extent: body.container_extent,
            content: body.content,
        })
    }
}

fn source_case_invocation(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<(
    tcl_registry::CaseListSpec,
    tcl_registry::spec::CaseInvocation,
)> {
    schema.authored_source_case_invocation()
}

/// Complete original body geometry for a mutation-coverage consumer. The
/// ordinary source inventory may expose valid partial regions; it cannot
/// establish that every possible executable body was inspected.
pub(in super::super) fn source_script_bodies_for_mutation_coverage(
    words: &super::super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<Vec<OriginalSourceScriptBody>> {
    source_script_bodies_for_coverage(
        words,
        context,
        OriginalSourceScriptPurpose::PotentialEvaluation,
    )
}

/// Shared complete body geometry for independently selected syntax/evaluation
/// purposes. Unrepresented lambda containers and malformed bodies stay unknown.
pub(in super::super) fn source_script_bodies_for_coverage(
    words: &super::super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
    purpose: OriginalSourceScriptPurpose,
) -> Option<Vec<OriginalSourceScriptBody>> {
    if (matches!(
        words.source(),
        super::super::source_structure::OriginalRegistrySource::DefinitionMember(_)
    ) || words.definition_parent().is_some())
        && purpose != OriginalSourceScriptPurpose::Syntax
    {
        return None;
    }
    let roles = words.roles()?;
    let timing_at = |argument| {
        words
            .with_source_schema(context, |schema| {
                schema.authored_source_script_timing_at(argument)
            })
            .flatten()
    };
    // This Body inventory does not represent lambda-container bodies.
    // Missing representation cannot certify complete evaluated coverage.
    for &(argument, role) in roles {
        if role == ArgRole::LambdaLiteral
            && (purpose == OriginalSourceScriptPurpose::Syntax
                || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
        {
            return None;
        }
    }
    let case = words.with_source_schema(context, source_case_invocation)?;
    let list_argument = case.and_then(|(_, layout)| layout.clause_list_index);
    let mut expected = Vec::new();
    if let Some((case, layout)) = case
        && let Some(argument) = layout.clause_list_index
        && (purpose == OriginalSourceScriptPurpose::Syntax
            || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
    {
        let operand = words.operands().get(argument)?.as_ref()?;
        let body = source_operand_body(operand)?;
        let list = body
            .container
            .image()
            .try_text()
            .ok()?
            .get(body.content.as_range())?;
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: case.clause_flags,
            clause_value_flags: case.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list_with_syntax(
            list,
            &shape,
            body.container.config().list_parse,
        ) {
            if !clause.valid || clause.pattern.is_none() {
                return None;
            }
            let element = clause.body?;
            if !element.braced {
                return None;
            }
            let range = element.content_range();
            let start = body
                .content
                .start()
                .checked_add(u32::try_from(range.start).ok()?)?;
            let end = body
                .content
                .start()
                .checked_add(u32::try_from(range.end).ok()?)?;
            expected.push(Span::new(start, end));
        }
    }
    for &(argument, role) in roles {
        if role == ArgRole::Body
            && Some(argument) != list_argument
            && (purpose == OriginalSourceScriptPurpose::Syntax
                || timing_at(argument)? != tcl_registry::ScriptTiming::ReferenceOnly)
        {
            expected.push(source_operand_body(words.operands().get(argument)?.as_ref()?)?.content);
        }
    }
    expected.sort_by_key(|span| (span.start(), span.end()));
    expected.dedup();
    let bodies = source_script_bodies_for(words, context, purpose);
    (bodies
        .iter()
        .map(OriginalSourceScriptBody::content_span)
        .collect::<Vec<_>>()
        == expected)
        .then_some(bodies)
}

/// Possible case-body source geometry for a syntax consumer that must visit
/// every candidate arm. Unlike the strong inventory below, this carries the
/// Registry's explicit unknown-selection residual and never chooses a branch.
pub(crate) fn conditional_source_case_bodies_for(
    words: &super::super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
) -> Option<OriginalConditionalSourceCaseBodies> {
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let config = words
        .operands()
        .iter()
        .flatten()
        .find_map(|operand| operand.word().map(NativeWord::config))?;
    let value_rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let (case, projection) = words
        .with_source_schema(context, |schema| {
            let case = *schema.semantics.options.case_list?;
            let projection = schema.authored_source_case_clause_projection(value_rules)?;
            Some((case, projection))
        })
        .flatten()?;
    let source = std::sync::Arc::new(words.clone());
    let mut bodies = Vec::new();
    for candidate in &projection.bodies {
        let operand = words.operands().get(candidate.argument)?.as_ref()?;
        let geometry = source_operand_body(operand)?;
        let mut retain = |geometry: ScriptBodyGeometry| {
            if words.matches_source(geometry.container.image(), geometry.container.config())
                && geometry.content.start() >= geometry.container.span().start()
                && geometry.content.end() <= geometry.container.span().end()
            {
                bodies.push(OriginalSourceScriptBody {
                    source: std::sync::Arc::clone(&source),
                    container: geometry.container,
                    extent: geometry.extent,
                    container_extent: geometry.container_extent,
                    content: geometry.content,
                });
            }
        };
        if let Some(list_element) = candidate.list_element {
            let text = geometry.container.image().try_text().ok()?;
            let list = text.get(geometry.content.as_range())?;
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: case.clause_flags,
                clause_value_flags: case.clause_value_flags,
            };
            let clause = tcl_syntax::case_list::split_case_list_with_syntax(
                list,
                &shape,
                geometry.container.config().list_parse,
            )
            .into_iter()
            .find(|clause| clause.valid && clause.body_index == Some(list_element))?;
            let element = clause.body.filter(|body| body.braced)?;
            let range = element.content_range();
            let start = geometry
                .content
                .start()
                .checked_add(u32::try_from(range.start).ok()?)?;
            let end = geometry
                .content
                .start()
                .checked_add(u32::try_from(range.end).ok()?)?;
            let span = Span::new(start, end);
            retain(ScriptBodyGeometry {
                container: geometry.container,
                extent: span,
                container_extent: geometry.extent,
                content: span,
            });
        } else {
            retain(geometry);
        }
    }
    bodies.sort_unstable_by_key(|body| (body.content.start(), body.content.end()));
    bodies.dedup_by_key(|body| body.content);
    Some(OriginalConditionalSourceCaseBodies {
        source,
        projection,
        bodies,
    })
}

/// Project original script geometry through the shared authored role/case
/// grammar. Unknown source applicability stays attached to every region;
/// malformed lists and cooked values cannot donate original child positions.
pub(in super::super) fn source_script_bodies_for(
    words: &super::super::source_structure::OriginalRegistryWords,
    context: &ContextRegistry,
    purpose: OriginalSourceScriptPurpose,
) -> Vec<OriginalSourceScriptBody> {
    if (matches!(
        words.source(),
        super::super::source_structure::OriginalRegistrySource::DefinitionMember(_)
    ) || words.definition_parent().is_some())
        && purpose != OriginalSourceScriptPurpose::Syntax
    {
        return Vec::new();
    }
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return Vec::new();
    }
    let admits = |argument| {
        purpose == OriginalSourceScriptPurpose::Syntax
            || matches!(
                words.with_source_schema(context, |schema| schema.authored_source_script_timing_at(argument)),
                Some(Some(timing)) if timing != tcl_registry::ScriptTiming::ReferenceOnly
            )
    };
    let case = words
        .with_source_schema(context, source_case_invocation)
        .flatten();
    let list_argument = case.and_then(|(_, layout)| layout.clause_list_index);
    let source = std::sync::Arc::new(words.clone());
    let mut out = Vec::new();
    let mut retain = |geometry: ScriptBodyGeometry| {
        if !words.matches_source(geometry.container.image(), geometry.container.config())
            || geometry.content.start() < geometry.container.span().start()
            || geometry.content.end() > geometry.container.span().end()
        {
            return;
        }
        out.push(OriginalSourceScriptBody {
            source: std::sync::Arc::clone(&source),
            container: geometry.container,
            extent: geometry.extent,
            container_extent: geometry.container_extent,
            content: geometry.content,
        });
    };
    // A selected case layout names its clause list positively. Where the
    // option scan cannot fix that word's timing (an 8.4 two-word `switch` whose
    // dynamic subject may read as an option), the clause bodies still run
    // whenever the command succeeds, so only a reference-only timing excludes
    // them from a potential-evaluation inventory.
    let case_list_may_run = |argument| {
        purpose == OriginalSourceScriptPurpose::Syntax
            || !matches!(
                words.with_source_schema(context, |schema| schema
                    .authored_source_script_timing_at(argument)),
                Some(Some(tcl_registry::ScriptTiming::ReferenceOnly))
            )
    };
    if case.is_some_and(|(_, layout)| layout.clause_list_index.is_none_or(case_list_may_run)) {
        retain_source_case_bodies(words, case.as_ref(), &mut retain);
    }
    let concatenates = words
        .with_source_schema(context, |schema| {
            schema
                .facts()
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        })
        .unwrap_or(false);
    let body_count = words
        .roles()
        .unwrap_or_default()
        .iter()
        .filter(|(_, role)| *role == ArgRole::Body)
        .count();
    for &(argument, role) in words.roles().unwrap_or_default() {
        // A concatenating schema marks the first fragment as Body; the
        // remaining actual argv still belongs to that one constructed script.
        // Only one complete original word can supply a contiguous body here.
        if concatenates
            && (body_count != 1
                || argument.checked_add(1) != Some(words.arguments().len())
                || words
                    .operands()
                    .get(argument)
                    .and_then(Option::as_ref)
                    .and_then(super::super::source_structure::OriginalOperandSource::word)
                    .is_none())
        {
            continue;
        }
        if role != ArgRole::Body || list_argument == Some(argument) || !admits(argument) {
            continue;
        }
        if let Some(body) = words
            .operands()
            .get(argument)
            .and_then(Option::as_ref)
            .and_then(source_operand_body)
        {
            retain(body);
        }
    }
    out.sort_by_key(|body| (body.content.start(), body.content.end()));
    out.dedup_by_key(|body| body.content);
    out
}

/// Original command regions inside one whole source expression operand.
/// The actual complete input selects expression grammar; no reconstructed
/// argv fragments or cooked expression can donate child source offsets.
pub(in super::super) fn source_expression_script_bodies(
    words: &super::super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let context = input.context_registry();
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let whole_source = words.with_source_schema(&context, |schema| {
        !schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
            || schema
                .words
                .arguments()
                .exact_argv_len()
                .zip(schema.semantics.argument_offset.checked_add(1))
                .is_some_and(|(count, expected)| count == expected)
    })?;
    if !whole_source {
        return None;
    }
    let roles = words.roles()?;
    let mut expressions = roles.iter().filter(|(_, role)| *role == ArgRole::Expr);
    let Some(&(argument, _)) = expressions.next() else {
        return Some(Vec::new());
    };
    if expressions.next().is_some() {
        return None;
    }
    source_expression_script_bodies_for_argument(words, input, argument)
}

/// Expression substitution regions at the genuine content base retained by
/// the expression AST. Multiple Expr operands remain separate source owners.
pub(crate) fn source_expression_script_bodies_at(
    words: &super::super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    expression_base: u32,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let context = input.context_registry();
    if words.context() != Some(context.context()) || !words.matches_registry(context.commands()) {
        return None;
    }
    let arguments = words
        .roles()?
        .iter()
        .filter_map(|&(argument, role)| {
            if role != ArgRole::Expr {
                return None;
            }
            let geometry = source_operand_body(words.operands().get(argument)?.as_ref()?)?;
            (geometry.content.start() == expression_base).then_some(argument)
        })
        .collect::<Vec<_>>();
    let [argument] = arguments.as_slice() else {
        return None;
    };
    source_expression_script_bodies_for_argument(words, input, *argument)
}

fn source_expression_script_bodies_for_argument(
    words: &super::super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    argument: usize,
) -> Option<Vec<OriginalSourceScriptBody>> {
    let geometry = source_operand_body(words.operands().get(argument)?.as_ref()?)?;
    let config = input.lexer_config();
    if !words.matches_source(geometry.container.image(), config)
        || geometry.container.config() != config
    {
        return None;
    }
    let expression = geometry
        .container
        .image()
        .bytes()
        .get(geometry.content.as_range())?;
    let profile = input.analyser_profile();
    let grammar = config.grammar_over(profile.grammar);
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        expression,
        &grammar,
        profile.expr_grammar_base,
        profile.f5_core_expr_grammar(),
    );
    if unknown {
        return None;
    }
    let terms = tcl_lexer::expression_terms(expression, &tokens, config)?;
    let source = Arc::new(words.clone());
    let mut bodies = terms
        .iter()
        .filter(|term| term.kind == tcl_lexer::ExprTermKind::Command)
        .map(|term| term.value.clone())
        .collect::<Vec<_>>();
    bodies.extend(quoted_operand_commands(expression, &terms, profile, config));
    bodies
        .into_iter()
        .map(|body| {
            let start = geometry.content.start().checked_add(body.start)?;
            let end = geometry.content.start().checked_add(body.end)?;
            let command_span = Span::new(start, end);
            (start <= end && end <= geometry.content.end()).then_some(OriginalSourceScriptBody {
                source: Arc::clone(&source),
                container: geometry.container.clone(),
                extent: command_span,
                container_extent: geometry.container_extent,
                content: command_span,
            })
        })
        .collect()
}

/// The bodies of the command substitutions `expr` evaluates inside a quoted
/// operand (`{"[class match a equals b]"}`), which the term scan leaves in
/// one string term. The expression owner says which ones are live.
fn quoted_operand_commands(
    expression: &[u8],
    terms: &[tcl_lexer::ExprTerm],
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> Vec<std::ops::Range<u32>> {
    let quoted = terms
        .iter()
        .filter(|term| term.kind == tcl_lexer::ExprTermKind::EscapedString)
        .map(|term| term.value.clone())
        .collect::<Vec<_>>();
    if quoted.is_empty() {
        return Vec::new();
    }
    let Ok(text) = std::str::from_utf8(expression) else {
        return Vec::new();
    };
    let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(profile);
    tcl_syntax::expr::live_expression_substitutions(text, profile, config, |parsed| {
        surface.validate(parsed).is_ok()
    })
    .commands
    .into_iter()
    .filter(|command| {
        expression.get(command.start() as usize) == Some(&b'[')
            && command.end() > command.start() + 1
            && expression.get(command.end() as usize - 1) == Some(&b']')
            && quoted
                .iter()
                .any(|operand| operand.start <= command.start() && command.end() <= operand.end)
    })
    .map(|command| command.start() + 1..command.end() - 1)
    .collect()
}

fn retain_source_case_bodies(
    words: &super::super::source_structure::OriginalRegistryWords,
    case: Option<&(
        tcl_registry::CaseListSpec,
        tcl_registry::spec::CaseInvocation,
    )>,
    retain: &mut impl FnMut(ScriptBodyGeometry),
) {
    if let Some((case, layout)) = case
        && let Some(argument) = layout.clause_list_index
        && let Some(operand) = words.operands().get(argument).and_then(Option::as_ref)
        && let Some(body) = source_operand_body(operand)
        && let Ok(text) = body.container.image().try_text()
        && let Some(list) = text.get(body.content.as_range())
    {
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: case.clause_flags,
            clause_value_flags: case.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list_with_syntax(
            list,
            &shape,
            body.container.config().list_parse,
        ) {
            let Some(element) = clause.body.filter(|element| clause.valid && element.braced) else {
                continue;
            };
            let range = element.content_range();
            let Some(start) = u32::try_from(range.start)
                .ok()
                .and_then(|offset| body.content.start().checked_add(offset))
            else {
                continue;
            };
            let Some(end) = u32::try_from(range.end)
                .ok()
                .and_then(|offset| body.content.start().checked_add(offset))
            else {
                continue;
            };
            let body_span = Span::new(start, end);
            retain(ScriptBodyGeometry {
                container: body.container.clone(),
                extent: body_span,
                container_extent: body.extent,
                content: body_span,
            });
        }
    }
    if let Some((case, layout)) = case
        && let Some(start) = layout.inline_clause_start
        && let Some(values) = words.arguments().get(start..).and_then(|arguments| {
            arguments
                .iter()
                .map(|argument| {
                    let bytes = argument.literal_bytes()?;
                    (!bytes.contains(&0)).then_some(())?;
                    std::str::from_utf8(bytes).ok()
                })
                .collect::<Option<Vec<_>>>()
        })
        && let Some(clauses) = case.inline_clauses(&values, 0)
    {
        for clause in clauses {
            let Some(argument) = clause.body_index.and_then(|index| start.checked_add(index))
            else {
                continue;
            };
            if let Some(body) = words
                .operands()
                .get(argument)
                .and_then(Option::as_ref)
                .and_then(source_operand_body)
            {
                retain(body);
            }
        }
    }
}
