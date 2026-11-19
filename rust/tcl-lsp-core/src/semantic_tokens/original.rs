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

//! Original naming roles. Declarations supply syntax roles; only a retained
//! dispatch receipt supplies a source method-call role. Neither selects a
//! runtime value, compilation entry or future callback.

use super::{ArgOverride, ScriptCtx, TokenKind};
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_compiler::signature_scan::scope::SignatureSourceNameKey;
use tcl_lexer::{SourceImage, Span};

#[derive(Default)]
pub(super) struct OriginalTokenRoles {
    names: FxHashMap<u32, (Span, ArgOverride)>,
    heads: FxHashSet<u32>,
    class_heads: FxHashSet<u32>,
    operands: FxHashMap<u32, crate::original_invocation::OriginalOperandSource>,
    formal_dialects: FxHashMap<u32, Option<tcl_registry::InvocationDialect>>,
    current: bool,
    lexical_oo: bool,
    /// Heads the analysis recorded an invocation for. The structure-only
    /// tier records few, so any other head asks the realm when it is walked.
    recorded_heads: FxHashSet<u32>,
}

impl OriginalTokenRoles {
    pub(super) fn capture(source: &str, analysis: Option<&AnalysisResult>) -> Self {
        let Some(analysis) = analysis else {
            return Self::default();
        };
        let image = SourceImage::document(source);
        let mut roles = Self {
            lexical_oo: analysis.allows_lexical_declaration_advice(),
            recorded_heads: analysis
                .command_invocations
                .iter()
                .map(|invocation| invocation.range.start())
                .collect(),
            ..Self::default()
        };
        roles.heads.extend(
            analysis
                .command_invocations
                .iter()
                .filter(|invocation| invocation.original_name_input.is_some())
                .map(|invocation| invocation.range.start()),
        );
        roles.current = analysis
            .body_lexer_config
            .is_some_and(|config| analysis.matches_original_source_image(&image, config));
        if !roles.current {
            return roles;
        }
        let Some(config) = analysis.body_lexer_config else {
            return roles;
        };
        roles.capture_vendor_names(&image, analysis);
        // A vendor unit lists each procedure natively as well; there the
        // vendor row alone decides whether the name can be coloured.
        let vendor_names: FxHashSet<u32> = analysis
            .original_vendor_procedure_declarations()
            .map(|declaration| declaration.name_input().span().start())
            .collect();
        for declaration in analysis.original_procedure_declarations() {
            if !vendor_names.contains(&declaration.name_input().span().start()) {
                roles.name(&image, declaration.name_input(), ArgOverride::ProcNameDef);
            }
        }
        roles.capture_counted_classes(&image, config, analysis);
        for declaration in analysis.original_class_declarations() {
            roles.name(&image, declaration.name_input(), ArgOverride::ClassNameDef);
            for member in declaration.metadata().original_members.declarations() {
                roles.member(&image, config, member);
            }
            for property in declaration.metadata().original_properties.declarations() {
                if !property.matches_producer_input(analysis) {
                    continue;
                }
                roles.name(
                    &image,
                    property.declaration().name_input(),
                    ArgOverride::MemberName,
                );
            }
        }
        for configuration in analysis.original_class_configurations() {
            for member in configuration.members().declarations() {
                roles.member(&image, config, member);
            }
            for property in configuration.properties().declarations() {
                if !property.matches_producer_input(analysis) {
                    continue;
                }
                roles.name(
                    &image,
                    property.declaration().name_input(),
                    ArgOverride::MemberName,
                );
            }
        }
        for configuration in analysis.original_object_configurations() {
            for member in configuration.members().declarations() {
                roles.member(&image, config, member);
            }
            for property in configuration.properties().declarations() {
                if !property.matches_producer_input(analysis) {
                    continue;
                }
                roles.name(
                    &image,
                    property.declaration().name_input(),
                    ArgOverride::MemberName,
                );
            }
        }
        for occurrence in &analysis.original_variable_symbols {
            if occurrence.is_declaration()
                && let Some(key) = occurrence.original_name_input().original_word_key()
                && key.span() == occurrence.span()
            {
                roles.name(&image, key, ArgOverride::VarDecl);
            }
        }
        roles.capture_class_heads(source, analysis);
        roles
    }

    fn capture_class_heads(&mut self, source: &str, analysis: &AnalysisResult) {
        for invocation in &analysis.command_invocations {
            if matches!(
                crate::original_oo::class_for_invocation(analysis, source, invocation),
                std::ops::ControlFlow::Break(Some(_))
            ) || crate::original_oo::counted_class_for_invocation(source, analysis, invocation)
                .is_some()
            {
                self.class_heads.insert(invocation.range.start());
            }
        }
    }

    fn capture_counted_classes(
        &mut self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        analysis: &AnalysisResult,
    ) {
        if let Some(input) = analysis.resolved_input.as_ref() {
            let context = input.context_registry();
            for declaration in &analysis.original_counted_class_metadata {
                if declaration.name_input().lexer_config() == config
                    && declaration.metadata().body().matches_source(image, config)
                    && declaration.metadata().body().matches_context(&context)
                {
                    self.name(image, declaration.name_input(), ArgOverride::ClassNameDef);
                    let body = declaration.metadata().body().content_span();
                    if let Some(methods) = declaration.metadata().initial_method_declarations() {
                        for method in methods {
                            let key = method.original_name_input();
                            let word = method.original_word();
                            let site = method.site();
                            if key.original_word() == word
                                && key.source_image() == image
                                && key.lexer_config() == config
                                && key.policy() == declaration.name().policy()
                                && site.source == declaration.declaration_site().source
                                && body.start() <= site.offset
                                && site.offset <= word.span().start()
                                && word.span().end() <= body.end()
                            {
                                self.name(image, key, ArgOverride::MemberName);
                            }
                        }
                    }
                }
            }
        }
    }

    fn capture_vendor_names(&mut self, image: &SourceImage, analysis: &AnalysisResult) {
        for declaration in analysis.original_vendor_procedure_declarations() {
            self.vendor_name(
                image,
                declaration.name_input(),
                declaration.purpose(),
                ArgOverride::ProcNameDef,
            );
        }
        for declaration in analysis.original_vendor_class_declarations() {
            self.vendor_name(
                image,
                declaration.name_input(),
                declaration.purpose(),
                ArgOverride::ClassNameDef,
            );
        }
        for declaration in analysis.original_vendor_symbol_declarations() {
            let kind = match declaration.metadata().kind {
                tcl_registry::DefinedSymbolKind::Event => super::TokenKind::Event,
                tcl_registry::DefinedSymbolKind::Test => super::TokenKind::Function,
                tcl_registry::DefinedSymbolKind::Constraint => super::TokenKind::Variable,
                tcl_registry::DefinedSymbolKind::Matcher => super::TokenKind::Operator,
            };
            self.vendor_name(
                image,
                declaration.name_input(),
                declaration.purpose(),
                ArgOverride::Kind(kind),
            );
        }
    }

    fn member(
        &mut self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        member: &tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    ) {
        if let Some(original) = member.declaration().static_occurrence() {
            self.name(image, original.name_input(), ArgOverride::MemberName);
        }
        for (word, kind) in [
            (member.parameters_word(), ArgOverride::ParamList),
            (member.body_word(), ArgOverride::BodyScript),
        ] {
            let Some(word) = word else {
                continue;
            };
            let [token] = word.tokens() else {
                continue;
            };
            if word.image() != image
                || word.config() != config
                || word.group().expand
                || matches!(kind, ArgOverride::BodyScript)
                    && token.kind != tcl_lexer::TokenType::Str
            {
                continue;
            }
            self.names
                .insert(token.span.start(), (token.span, kind.clone()));
            let input = SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                member.declaration().original_name_input().policy(),
            )
            .map(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord);
            let operand = crate::original_invocation::OriginalOperandSource {
                span: token.span,
                input,
                word: Some(word.clone()),
            };
            self.operands.entry(token.span.start()).or_insert(operand);
            if matches!(kind, ArgOverride::ParamList) {
                self.formal_dialects
                    .entry(token.span.start())
                    .and_modify(|dialect| {
                        if *dialect != member.source_dialect() {
                            *dialect = None;
                        }
                    })
                    .or_insert(member.source_dialect());
            }
        }
    }

    /// Exact selected worker-word geometry, without formal binding or body entry.
    pub(super) fn operand_at(
        &self,
        span: Span,
    ) -> Option<&crate::original_invocation::OriginalOperandSource> {
        let operand = self.operands.get(&span.start())?;
        (self.current && operand.span == span).then_some(operand)
    }

    fn name(&mut self, image: &SourceImage, key: &SignatureSourceNameKey, kind: ArgOverride) {
        if key.source_image() == image {
            self.names.insert(key.span().start(), (key.span(), kind));
        }
    }

    fn vendor_name(
        &mut self,
        image: &SourceImage,
        input: &tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput,
        purpose: tcl_syntax::naming::VendorSourceNamePurpose,
        kind: ArgOverride,
    ) {
        if input.source_image() != image || input.literal_units(purpose).is_none() {
            return;
        }
        if let Some(token) = input.original_word().tokens().first() {
            self.names.insert(token.span.start(), (token.span, kind));
        }
    }

    // Native-policy OO heads never borrow reporting overlays when their
    // original producer or current method receipt is unavailable.
    pub(super) fn has_head(&self, offset: u32) -> bool {
        !self.lexical_oo || self.heads.contains(&offset)
    }
    pub(super) fn is_class_head(&self, offset: u32) -> bool {
        self.class_heads.contains(&offset)
    }

    /// Whether the command's head names a class: recorded by the analysis,
    /// or read from the realm for a head it recorded no invocation for.
    pub(super) fn head_names_class(&self, ctx: ScriptCtx<'_>, command: &SegmentedCommand) -> bool {
        let Some(head) = command.argv.first() else {
            return false;
        };
        if self.is_class_head(head.span.start()) {
            return true;
        }
        self.current
            && !self.recorded_heads.contains(&head.span.start())
            && ctx.analysis.is_some_and(|analysis| {
                crate::original_oo::command_head_names_class(analysis, ctx.full_source, command)
            })
    }

    pub(super) fn insert_overrides(
        &self,
        ctx: ScriptCtx<'_>,
        command: &SegmentedCommand,
        overrides: &mut FxHashMap<u32, ArgOverride>,
    ) {
        if !self.current {
            return;
        }
        for token in command.arg_tokens() {
            if let Some((span, kind)) = self.names.get(&token.span.start())
                && *span == token.span
            {
                overrides.insert(token.span.start(), kind.clone());
            }
        }
        let Some(analysis) = ctx.analysis else {
            return;
        };
        let Some(selected) =
            crate::receiver_identity::method_at_command(analysis, ctx.full_source, command)
        else {
            return;
        };
        // Computed selectors keep their substitutions and nested scripts. A
        // dispatch identity cannot flatten them into an editable name token.
        if selected
            .original_selector_input()
            .original_word_key()
            .is_some()
        {
            overrides.insert(
                selected.selector.start(),
                ArgOverride::Kind(TokenKind::Method),
            );
        }
    }
}

/// A source head uses the selected full-context descriptor for its traits and
/// the original naming domain for written qualifiers and their coordinates.
pub(super) fn registry_head(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    entries: &mut Vec<super::Entry>,
) {
    let Some(&head) = command.argv.first() else {
        return;
    };
    let traits = words.with_source_schema(ctx.generation, |schema| {
        schema.authored_source_descriptors().command.traits
    });
    let kind = if traits
        .is_some_and(|traits| traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD))
    {
        TokenKind::Keyword
    } else if traits.is_some_and(|traits| traits.contains(tcl_registry::Traits::OPERATOR_COMMAND)) {
        TokenKind::Operator
    } else {
        TokenKind::Function
    };
    let mods = if kind == TokenKind::Function && traits.is_some() {
        super::MOD_DEFAULT_LIBRARY
    } else {
        0
    };
    let split = original_head_split(ctx, words);
    if let Some((prefix, tail)) = split {
        super::push_token(
            ctx.line_index,
            ctx.full_source,
            tcl_lexer::Token {
                span: prefix,
                ..head
            },
            TokenKind::Namespace,
            if traits.is_some() {
                super::MOD_DEFAULT_LIBRARY
            } else {
                0
            },
            entries,
        );
        super::push_token(
            ctx.line_index,
            ctx.full_source,
            tcl_lexer::Token { span: tail, ..head },
            kind,
            mods,
            entries,
        );
    } else {
        super::push_token(ctx.line_index, ctx.full_source, head, kind, mods, entries);
    }
}

/// Original written qualifiers retain their independently selected naming
/// domain and source components; a Registry target label supplies no spelling.
fn original_head_split(
    ctx: ScriptCtx<'_>,
    words: &crate::original_invocation::OriginalRegistryWords,
) -> Option<(Span, Span)> {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let original = words.head_source()?;
    let word = original.word()?;
    if word.image() != &SourceImage::document(ctx.full_source) || word.config() != ctx.config {
        return None;
    }
    if let Some(input) = original.input() {
        let key = input.original_word_key()?;
        (key.original_word() == word).then_some(())?;
        let tail = input.policy().recipe().command_tail_extent(input.bytes())?;
        (tail.start != 0).then_some(())?;
        let prefix =
            crate::original_name_edit::original_static_name_value_span(input, 0..tail.start)?;
        let tail = crate::original_name_edit::original_static_name_value_span(input, tail)?;
        return (prefix.end() == tail.start()).then_some((prefix, tail));
    }
    let word = original_logical_head(ctx, words)?;
    let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
    let tail = tcl_syntax::naming::written_command_tail(&value);
    let boundary = value.len().checked_sub(tail.len())?;
    (boundary != 0 && !tail.is_empty()).then_some(())?;
    let start = tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, 0)?;
    let split = tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, boundary)?;
    let end = tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, value.len())?;
    Some((Span::new(start, split), Span::new(split, end)))
}

/// Complete static Logical heads retain their positive source-domain seal;
/// multiple lexical escape components do not make that whole word dynamic.
pub(super) fn original_logical_head<'a>(
    ctx: ScriptCtx<'_>,
    words: &'a crate::original_invocation::OriginalRegistryWords,
) -> Option<&'a tcl_lexer::NativeWord> {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) =
        &words.source
    else {
        return None;
    };
    let retained = ctx.analysis?.resolved_input.as_ref()?;
    (advice.logical_source_input() == Some(retained)).then_some(())?;
    let input = advice.original_head().logical_input()?;
    let word = words.head_source()?.word()?;
    if input.original_word() != word
        || word.image() != &SourceImage::document(ctx.full_source)
        || word.config() != ctx.config
    {
        return None;
    }
    (tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?.as_slice()
        == input.bytes())
    .then_some(word)
}

/// A selected definition-body grammar may classify its own complete original
/// member word. This is separate from ordinary command lookup and never uses
/// a Registry label after a source-schema refusal.
pub(super) fn definition_member_head(ctx: ScriptCtx<'_>, command: &SegmentedCommand) -> bool {
    definition_member_head_word(ctx, command).is_some()
}

pub(super) fn definition_member_head_word<'a>(
    ctx: ScriptCtx<'a>,
    command: &SegmentedCommand,
) -> Option<&'a tcl_lexer::NativeWord> {
    let members = ctx.original_definition_members?;
    let words = original_definition_member_words(ctx, command)?;
    members.member_keyword(words)?;
    words.first()
}

fn original_definition_member_words<'a>(
    ctx: ScriptCtx<'a>,
    command: &SegmentedCommand,
) -> Option<&'a [tcl_lexer::NativeWord]> {
    let start = command.argv.first()?.span.start();
    ctx.original_definition_members?
        .original_command_words_at(start)
}

/// A source member's receiver vocabulary is independent of its definition vocabulary.
pub(super) fn receiver_grammar_for_body(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    body: tcl_lexer::Token,
) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
    let current =
        crate::original_context::CurrentSourceContext::capture(ctx.full_source, ctx.analysis?)?;
    if current.config() != ctx.config || current.context().context() != ctx.context {
        return None;
    }
    if let Some(members) = ctx.original_definition_members
        && definition_member_head(ctx, command)
    {
        let original = original_definition_member_words(ctx, command)?;
        let bodies = members.script_bodies(original)?;
        let image = SourceImage::document(ctx.full_source);
        let mut matching = bodies.iter().filter(|selected| {
            selected.matches_source(&image, ctx.config)
                && selected.matches_context(ctx.generation)
                && selected.original_container().tokens().contains(&body)
        });
        let selected = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        return selected.receiver_grammar(ctx.generation);
    }
    let grammar = ctx.receiver_grammar?;
    original?
        .source_script_bodies(ctx.generation)
        .into_iter()
        .any(|selected| {
            selected
                .original_container()
                .tokens()
                .iter()
                .any(|token| token.span == body.span)
                && selected.inherits_immediate_caller_source_context(ctx.generation)
        })
        .then_some(grammar)
}

pub(super) fn definition_member_bodies(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
) -> Option<Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>> {
    ctx.original_definition_members?
        .script_bodies(original_definition_member_words(ctx, command)?)
}

fn insert_member_source_role(
    command: &SegmentedCommand,
    word: &tcl_lexer::NativeWord,
    role: tcl_registry::ArgRole,
    kind: ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !word
        .tokens()
        .iter()
        .all(|token| command.all_tokens.contains(token))
    {
        return;
    }
    if role == tcl_registry::ArgRole::VarWrite {
        super::retag_original_variable_argument(command, word, kind, overrides);
    } else if tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some() {
        if role == tcl_registry::ArgRole::Body && word.group().kind != tcl_lexer::WordKind::Braced {
            return;
        }
        if matches!(
            role,
            tcl_registry::ArgRole::ParamList | tcl_registry::ArgRole::Body
        ) {
            if let [token] = word.tokens() {
                overrides.entry(token.span.start()).or_insert(kind);
            }
        } else {
            super::mark_literal_fragments(command, word.word_span(), &kind, overrides);
        }
    }
}

fn member_source_role_kinds() -> [(tcl_registry::ArgRole, ArgOverride); 6] {
    use tcl_registry::ArgRole;
    [
        (ArgRole::VarWrite, ArgOverride::VarDecl),
        (ArgRole::Name, ArgOverride::MemberName),
        (ArgRole::ParamList, ArgOverride::ParamList),
        (ArgRole::Option, ArgOverride::Decorator),
        (
            ArgRole::NamespaceName,
            ArgOverride::Kind(TokenKind::Namespace),
        ),
        (ArgRole::Keyword, ArgOverride::Kind(TokenKind::Keyword)),
    ]
}

pub(super) fn definition_member_parameter_operands(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
) -> Vec<crate::original_invocation::OriginalOperandSource> {
    let Some(words) = original_definition_member_words(ctx, command).and_then(|original| {
        ctx.original_definition_members?
            .argument_words_for_role(original, tcl_registry::ArgRole::ParamList)
    }) else {
        return Vec::new();
    };
    words
        .into_iter()
        .filter_map(|word| {
            let [token] = word.tokens() else { return None };
            Some(crate::original_invocation::OriginalOperandSource {
                span: token.span,
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect()
}

pub(super) fn insert_definition_member_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(original) = original_definition_member_words(ctx, command) else {
        return;
    };
    let Some(members) = ctx.original_definition_members else {
        return;
    };
    for (role, kind) in member_source_role_kinds() {
        for word in members
            .argument_words_for_role(original, role)
            .into_iter()
            .flatten()
        {
            insert_member_source_role(command, word, role, kind.clone(), overrides);
        }
    }
    for reference in members.references(original).into_iter().flatten() {
        let kind = match reference.kind() {
            tcl_registry::definer::MemberRefKind::Class => ArgOverride::ClassNameRef,
            tcl_registry::definer::MemberRefKind::Method => ArgOverride::Kind(TokenKind::Method),
        };
        insert_member_source_role(
            command,
            reference.original_word(),
            tcl_registry::ArgRole::Name,
            kind,
            overrides,
        );
    }
}

/// Ordinary source roles preserve the actual effective argv and each operand's
/// own original word. Conditional schemas keep their obligations in `words`;
/// these colours claim source syntax only, never an executed variable or call.
pub(super) fn registry_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
) -> FxHashMap<u32, ArgOverride> {
    let mut overrides = FxHashMap::default();
    match &words.source {
        crate::original_invocation::OriginalRegistrySource::Selected => {}
        crate::original_invocation::OriginalRegistrySource::ConditionalNativeSource(_)
        | crate::original_invocation::OriginalRegistrySource::DefinitionMember(_) => {
            return overrides;
        }
        crate::original_invocation::OriginalRegistrySource::Scoped(scoped) => {
            if scoped.body().context() != ctx.context {
                return overrides;
            }
        }
        crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) => {
            if advice.context() != ctx.context {
                return overrides;
            }
        }
        crate::original_invocation::OriginalRegistrySource::ProducedPrefix(prefix) => {
            if !prefix.matches_source_context(
                &SourceImage::document(ctx.full_source),
                ctx.config,
                ctx.generation,
            ) {
                return overrides;
            }
        }
        crate::original_invocation::OriginalRegistrySource::Conditional(metadata) => {
            if metadata.context() != ctx.context {
                return overrides;
            }
        }
        crate::original_invocation::OriginalRegistrySource::Vendor(metadata) => {
            if metadata.shape().context() != ctx.context {
                return overrides;
            }
        }
    }
    // A dynamic word leaves the role set incomplete. The schema queries below
    // answer per word, so they still colour what the written words determine.
    if let Some(roles) = words.roles.as_deref() {
        insert_source_roles(
            command,
            roles,
            |ordinal| ArgumentWords::Original(words).literal(command, ordinal),
            |ordinal| original_argument_token(command, words, ordinal),
            |ordinal| {
                let operand = words.operands.get(ordinal)?.as_ref()?;
                command
                    .argv
                    .iter()
                    .any(|token| token.span == operand.span)
                    .then_some(())?;
                operand.word.as_ref()
            },
            &mut overrides,
        );
    } else {
        insert_written_word_roles(ctx, command, words, &mut overrides);
    }
    insert_format_overrides(ctx, command, words, &mut overrides);
    insert_pattern_overrides(ctx, command, words, &mut overrides);
    let _ = words.with_source_schema(ctx.generation, |schema| {
        insert_source_schema_overrides(
            ctx,
            command,
            ArgumentWords::Original(words),
            words.roles.as_deref().unwrap_or(&[]),
            schema,
            &mut overrides,
        );
    });
    overrides
}

/// The argument words a role mapping reads: the retained operands of a
/// selected invocation, or the written words of a command the analysis kept
/// no invocation for.
#[derive(Clone, Copy)]
enum ArgumentWords<'a> {
    Original(&'a crate::original_invocation::OriginalRegistryWords),
    Written,
}

impl ArgumentWords<'_> {
    /// The token argument `ordinal` starts at.
    fn token(self, command: &SegmentedCommand, ordinal: usize) -> Option<tcl_lexer::Token> {
        match self {
            Self::Original(words) => original_argument_token(command, words, ordinal),
            Self::Written => command.argv.get(ordinal.checked_add(1)?).copied(),
        }
    }

    /// Whether the value of argument `ordinal` is its written spelling.
    fn literal(self, command: &SegmentedCommand, ordinal: usize) -> bool {
        match self {
            Self::Original(words) => words
                .arguments
                .get(ordinal)
                .is_some_and(|word| word.literal_bytes().is_some()),
            Self::Written => ordinal.checked_add(1).is_some_and(|word| {
                command.single_token_word.get(word) == Some(&true)
                    && command.argv.get(word).is_some_and(|token| {
                        matches!(
                            token.kind,
                            tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
                        )
                    })
            }),
        }
    }

    fn literal_text(
        self,
        command: &SegmentedCommand,
        ordinal: usize,
    ) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Original(words) => {
                let bytes = words.arguments.get(ordinal)?.literal_bytes()?;
                Some(std::borrow::Cow::Owned(
                    std::str::from_utf8(bytes).ok()?.to_owned(),
                ))
            }
            Self::Written => self.literal(command, ordinal).then(|| {
                std::borrow::Cow::Owned(command.texts.get(ordinal + 1).cloned().unwrap_or_default())
            }),
        }
    }
}

/// Every role the written-word display reads.
const WRITTEN_ROLES: &[tcl_registry::ArgRole] = &[
    tcl_registry::ArgRole::Body,
    tcl_registry::ArgRole::Expr,
    tcl_registry::ArgRole::LoopVarList,
    tcl_registry::ArgRole::ParamList,
    tcl_registry::ArgRole::CommandName,
    tcl_registry::ArgRole::CommandNameProbe,
    tcl_registry::ArgRole::LambdaLiteral,
    tcl_registry::ArgRole::Subcommand,
    tcl_registry::ArgRole::Keyword,
    tcl_registry::ArgRole::Option,
    tcl_registry::ArgRole::OptionTerminator,
    tcl_registry::ArgRole::NamespaceName,
    tcl_registry::ArgRole::Name,
    tcl_registry::ArgRole::VarWrite,
    tcl_registry::ArgRole::VarRead,
];

/// Display roles of a command the analysis kept no invocation for, read from
/// its written words and the document's own descriptor `spec`. Each written
/// spelling stands in for its value. This selects no command and grants no
/// binding, edit or diagnostic.
pub(super) fn insert_written_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    spec: &tcl_registry::CommandSpec,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    let texts: Vec<&str> = command.texts[1..].iter().map(String::as_str).collect();
    let arguments = tcl_registry::InvocationArguments::literals(&texts);
    let words = ArgumentWords::Written;
    let roles = ctx
        .registry
        .arg_role_assignments_words(spec.name, arguments, WRITTEN_ROLES)
        .unwrap_or_default();
    for &(ordinal, role) in &roles {
        let kind = match role {
            ArgRole::VarWrite => ArgOverride::VarDecl,
            ArgRole::VarRead => ArgOverride::VarRef,
            _ => continue,
        };
        if words.literal(command, ordinal)
            && let Some(token) = words.token(command, ordinal)
        {
            overrides.entry(token.span.start()).or_insert(kind);
        }
    }
    insert_source_roles(
        command,
        &roles,
        |ordinal| words.literal(command, ordinal),
        |ordinal| words.token(command, ordinal),
        |_| None,
        overrides,
    );
    for pattern in ctx.registry.pattern_args_words(spec.name, arguments) {
        if pattern.kind == tcl_registry::patterns::PatternType::Regex
            && let Some(token) = words.token(command, usize::from(pattern.index))
        {
            super::mark_literal_fragments(
                command,
                token.span,
                &ArgOverride::RegexPattern,
                overrides,
            );
        }
    }
    if let Some(schema) = ctx.registry.resolve_invocation(spec.name, &texts, None) {
        insert_source_schema_overrides(ctx, command, words, &roles, &schema, overrides);
    }
}

/// Display roles for a command whose exact role set a dynamic word leaves
/// incomplete. Each written spelling stands in for its unknown value, so `$re`
/// is an operand rather than a switch. Lowering, rename and diagnostics read
/// the exact roles and are unaffected.
fn insert_written_word_roles(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    if words.arguments.len() + 1 != command.texts.len()
        || words
            .arguments
            .iter()
            .all(|word| word.literal_bytes().is_some())
    {
        return;
    }
    let Some((name, script_frame)) = words.with_source_schema(ctx.generation, |schema| {
        (
            schema.authored_source_descriptors().command.name,
            schema.semantics.frame_effect.is_some_and(|effect| {
                effect.layout == tcl_registry::FrameArgLayout::ScriptInSelectedFrame
            }),
        )
    }) else {
        return;
    };
    let texts: Vec<&str> = command.texts[1..].iter().map(String::as_str).collect();
    let arguments = tcl_registry::InvocationArguments::literals(&texts);
    let mut roles = if script_frame {
        // Whatever the level word evaluates to, the last word is script text.
        texts
            .len()
            .checked_sub(1)
            .into_iter()
            .map(|last| (last, ArgRole::Body))
            .collect()
    } else {
        ctx.registry
            .arg_role_assignments_words(name, arguments, &[ArgRole::VarWrite, ArgRole::VarRead])
            .unwrap_or_default()
    };
    roles.retain(|&(_, role)| role != ArgRole::Pattern);
    insert_source_roles(
        command,
        &roles,
        |ordinal| ArgumentWords::Original(words).literal(command, ordinal),
        |ordinal| original_argument_token(command, words, ordinal),
        |ordinal| words.operands.get(ordinal)?.as_ref()?.word.as_ref(),
        overrides,
    );
    for pattern in ctx.registry.pattern_args_words(name, arguments) {
        if pattern.kind == tcl_registry::patterns::PatternType::Regex
            && let Some(Some(operand)) = words.operands.get(usize::from(pattern.index))
        {
            super::mark_literal_fragments(
                command,
                operand.span,
                &ArgOverride::RegexPattern,
                overrides,
            );
        }
    }
}

/// Source declarations own their roles independently of the Registry. Their
/// original whole words provide the same direct-written anchors; no builtin
/// descriptor, installed command or expression runtime is inherited.
pub(super) fn declared_overrides(
    command: &SegmentedCommand,
    words: &tcl_compiler::command_binding::OriginalDeclaredCommandWords,
) -> FxHashMap<u32, ArgOverride> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let mut overrides = FxHashMap::default();
    if let Some(roles) = words.supplied_argument_roles() {
        insert_source_roles(
            command,
            &roles,
            |ordinal| {
                words
                    .arguments()
                    .get(ordinal)
                    .is_some_and(|word| word.literal_bytes().is_some())
            },
            |ordinal| {
                let word = words.argument_word(ordinal)?;
                let token = *word.tokens().first()?;
                (command.argv.get(ordinal.checked_add(1)?)?.span == token.span).then_some(token)
            },
            |ordinal| words.argument_word(ordinal),
            &mut overrides,
        );
    }
    overrides
}

fn insert_source_roles<'word>(
    command: &SegmentedCommand,
    roles: &[(usize, tcl_registry::ArgRole)],
    static_at: impl Fn(usize) -> bool,
    token_at: impl Fn(usize) -> Option<tcl_lexer::Token>,
    word_at: impl Fn(usize) -> Option<&'word tcl_lexer::NativeWord>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    for &(ordinal, role) in roles {
        if matches!(role, ArgRole::VarWrite | ArgRole::VarRead) {
            if let Some(word) = word_at(ordinal) {
                super::retag_original_variable_argument(
                    command,
                    word,
                    if role == ArgRole::VarWrite {
                        ArgOverride::VarDecl
                    } else {
                        ArgOverride::VarRef
                    },
                    overrides,
                );
            }
            continue;
        }
        let Some(token) = token_at(ordinal) else {
            continue;
        };
        let static_value = static_at(ordinal);
        let kind = match role {
            ArgRole::Body if token.kind == tcl_lexer::TokenType::Str => {
                Some(ArgOverride::BodyScript)
            }
            ArgRole::Expr if token.kind == tcl_lexer::TokenType::Str => {
                Some(ArgOverride::ExprScript)
            }
            ArgRole::LoopVarList if static_value => Some(ArgOverride::LoopVarList),
            ArgRole::ParamList if static_value => Some(ArgOverride::ParamList),
            ArgRole::CommandPrefix | ArgRole::CommandName | ArgRole::CommandNameProbe
                if static_value =>
            {
                Some(ArgOverride::CommandRef)
            }
            ArgRole::LambdaLiteral if static_value => Some(ArgOverride::LambdaLiteral),
            ArgRole::Subcommand => Some(ArgOverride::SubcommandKeyword),
            ArgRole::Keyword => Some(ArgOverride::KeywordArg),
            ArgRole::Option | ArgRole::OptionTerminator => Some(ArgOverride::Decorator),
            ArgRole::NamespaceName if static_value => Some(ArgOverride::Kind(TokenKind::Namespace)),
            _ => None,
        };
        if let Some(kind) = kind {
            overrides.entry(token.span.start()).or_insert(kind);
        }
    }
}

fn insert_source_schema_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: ArgumentWords<'_>,
    roles: &[(usize, tcl_registry::ArgRole)],
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let token_at = |ordinal| words.token(command, ordinal);
    if matches!(
        schema.subcommand,
        tcl_registry::resolved_invocation::SubcommandResolution::Exact(_)
            | tcl_registry::resolved_invocation::SubcommandResolution::UniquePrefix(_)
    ) && let Some(token) = token_at(0)
    {
        overrides.insert(token.span.start(), ArgOverride::SubcommandKeyword);
    }
    if let Some(scan) = schema.authored_source_diagnostic_options() {
        if scan.subcommands.len() > 1
            && let Some(token) = token_at(1)
        {
            overrides.insert(token.span.start(), ArgOverride::SubcommandKeyword);
        }
        if let tcl_registry::resolved_invocation::AuthoredSourceOptionBoundary::Terminator(argument) =
            scan.boundary
            && let Some(token) = token_at(argument)
        {
            overrides.insert(token.span.start(), ArgOverride::Decorator);
        }
        for option in scan.options.into_iter().filter(|option| option.available) {
            if let Some(token) = token_at(option.argument) {
                overrides.insert(token.span.start(), ArgOverride::Decorator);
            }
            if let Some(values) = option.values {
                for ordinal in values {
                    if words.literal(command, ordinal)
                        && let Some(token) = token_at(ordinal)
                    {
                        overrides
                            .entry(token.span.start())
                            .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
                    }
                }
            }
        }
    }
    if let ArgumentWords::Original(original) = words {
        insert_inline_member_keywords(ctx, command, original, schema, overrides);
    }
    for ordinal in schema.authored_source_enum_arguments() {
        if let Some(token) = token_at(ordinal) {
            overrides
                .entry(token.span.start())
                .and_modify(|kind| {
                    if matches!(kind, ArgOverride::Kind(TokenKind::OptionValue)) {
                        *kind = ArgOverride::Kind(TokenKind::EnumMember);
                    }
                })
                .or_insert(ArgOverride::Kind(TokenKind::EnumMember));
        }
    }
    insert_source_case_overrides(command, words, schema, overrides);
    insert_logical_definition_overrides(ctx, command, words, roles, schema, overrides);
    if ctx.dialect.is_irules() {
        for object in tcl_irules::original_source_object_operands(schema, None) {
            if let Some(token) = token_at(object.argument())
                && words.literal(command, object.argument())
            {
                overrides.insert(token.span.start(), ArgOverride::Kind(TokenKind::Object));
            }
        }
    }
}

fn insert_source_case_overrides(
    command: &SegmentedCommand,
    words: ArgumentWords<'_>,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let token_at = |ordinal| words.token(command, ordinal);
    if let Some((spec, case)) = schema.authored_source_case_invocation()
        && let Some(ordinal) = case.clause_list_index
        && let Some(token) = token_at(ordinal)
        && token.kind == tcl_lexer::TokenType::Str
    {
        overrides.insert(
            token.span.start(),
            ArgOverride::CaseList(
                Box::new(spec),
                case.mode == tcl_registry::spec::CaseMatchMode::Regexp,
            ),
        );
    }
    for clause in schema
        .authored_source_inline_case_clauses()
        .unwrap_or_default()
    {
        for ordinal in clause.flag_indices {
            if let Some(token) = token_at(ordinal) {
                overrides.insert(token.span.start(), ArgOverride::Decorator);
            }
        }
        if clause.mode == tcl_registry::spec::CaseMatchMode::Regexp
            && let Some(token) = token_at(clause.pattern_index)
        {
            super::mark_literal_fragments(
                command,
                token.span,
                &ArgOverride::RegexPattern,
                overrides,
            );
        }
        if let Some(ordinal) = clause.body_index
            && let Some(token) = token_at(ordinal)
            && token.kind == tcl_lexer::TokenType::Str
        {
            overrides.insert(token.span.start(), ArgOverride::BodyScript);
        }
    }
}

/// Only a positively sealed Logical source word may use model definition
/// colours. Native and hosted declarations retain their own naming receipt.
fn insert_logical_definition_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: ArgumentWords<'_>,
    roles: &[(usize, tcl_registry::ArgRole)],
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    // A hosted iRules document has no Logical head; its rule-load declarations
    // (`when`, `proc`) are still read from the selected schema. Written words
    // have no head receipt at all and are display only, except in an iRules
    // document, where the rule loader's own boundary decides what declares.
    let logical = match words {
        ArgumentWords::Original(original) => original_logical_head(ctx, original).is_some(),
        ArgumentWords::Written => !ctx.dialect.is_irules(),
    };
    if !logical && !ctx.dialect.is_irules() {
        return;
    }
    let top_level_declaration = ctx
        .irules_top_level_declaration_heads
        .contains(&command.argv[0].span.start());
    let selected = schema.authored_source_descriptors();
    if schema
        .semantics
        .traits
        .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
        && selected.command.definition_body.is_none()
        && (logical || top_level_declaration)
    {
        for &(ordinal, role) in roles {
            if role == ArgRole::Name
                && words.literal(command, ordinal)
                && let Some(token) = words.token(command, ordinal)
            {
                overrides
                    .entry(token.span.start())
                    .or_insert(ArgOverride::ProcNameDef);
            }
        }
    }
    if schema
        .semantics
        .traits
        .contains(tcl_registry::Traits::IS_EVENT_HANDLER)
        && top_level_declaration
        && let Some(token) = words.token(command, 0)
        && words
            .literal_text(command, 0)
            .is_some_and(|name| super::is_event_name(&name))
    {
        overrides.insert(token.span.start(), ArgOverride::Kind(TokenKind::Event));
    }
    if let Some((ordinal, declares)) = source_class_ordinal(schema)
        && words.literal(command, ordinal)
        && let Some(token) = words.token(command, ordinal)
    {
        overrides.entry(token.span.start()).or_insert(if declares {
            ArgOverride::ClassNameDef
        } else {
            ArgOverride::ClassNameRef
        });
    }
    if let ArgumentWords::Original(original) = words {
        insert_logical_inline_member_overrides(ctx, command, original, schema, overrides);
    }
}

fn source_class_ordinal(
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
) -> Option<(usize, bool)> {
    let transitions = schema.state_transitions();
    for fact in transitions.facts() {
        match &fact.transition {
            tcl_registry::StateTransition::ObjectDispatch(
                tcl_registry::ObjectDispatchTransition::Create {
                    target: tcl_registry::ObjectDispatchTarget::Named(target),
                    kind: tcl_registry::ObjectDispatchKind::Class,
                    ..
                },
            ) => return Some((target.argument_index()?, true)),
            tcl_registry::StateTransition::ObjectDispatch(
                tcl_registry::ObjectDispatchTransition::Configure {
                    target,
                    layer: tcl_registry::ObjectDispatchLayer::Class,
                },
            ) => return Some((target.argument_index()?, false)),
            _ => {}
        }
    }
    let grammar = schema
        .authored_source_descriptors()
        .command
        .definition_body?;
    let installation = grammar.command_installation()?;
    Some((usize::from(installation.name_at), true))
}

pub(super) fn definition_class_name<'a>(
    ctx: ScriptCtx<'a>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
) -> Option<&'a str> {
    original_logical_head(ctx, words)?;
    let ordinal = words
        .with_source_schema(ctx.generation, source_class_ordinal)
        .flatten()?
        .0;
    let token = original_argument_token(command, words, ordinal)?;
    let value = words.arguments.get(ordinal)?.literal_bytes()?;
    let (start, text) = super::subspec_content(ctx.full_source, token)?;
    (text.as_bytes() == value && start <= token.span.end() as usize).then_some(text)
}

/// An inline definition call (`oo::define C method …`) writes its member word
/// at argument 1, after any wrapper members (`self`). Membership comes from the
/// selected descriptor's grammar, so the word is a keyword on every profile.
fn insert_inline_member_keywords(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !matches!(
        schema.semantics.analyser_hook,
        Some(
            tcl_registry::hooks::AnalyserHookId::OoDefine
                | tcl_registry::hooks::AnalyserHookId::OoObjdefine
        )
    ) {
        return;
    }
    let Some(grammar) = schema.authored_source_descriptors().command.definition_body else {
        return;
    };
    let mut ordinal = 1;
    while let Some(member) = words
        .arguments
        .get(ordinal)
        .and_then(|word| word.literal_bytes())
        .and_then(|bytes| grammar.source_member_in(bytes, Some(ctx.context.authoring_query())))
    {
        let Some(token) = original_argument_token(command, words, ordinal) else {
            break;
        };
        overrides.insert(token.span.start(), ArgOverride::Kind(TokenKind::Keyword));
        if member.kind != tcl_registry::definer::MemberKind::Wrapper {
            break;
        }
        ordinal += 1;
    }
}

fn insert_logical_inline_member_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let selected = schema.authored_source_descriptors().command;
    if !matches!(
        schema.semantics.analyser_hook,
        Some(
            tcl_registry::hooks::AnalyserHookId::OoDefine
                | tcl_registry::hooks::AnalyserHookId::OoObjdefine
        )
    ) {
        return;
    }
    let Some(grammar) = selected.definition_body else {
        return;
    };
    // The independent member grammar consumes the same complete written vector.
    // Captured prefixes cannot be assigned the current call's source geometry.
    if words.operands.iter().any(Option::is_none) || words.operands.len() + 1 != command.argv.len()
    {
        return;
    }
    let Some(member) = schema.words.arguments().literal_at(1) else {
        return;
    };
    if grammar
        .source_member_in(member.as_bytes(), Some(ctx.context.authoring_query()))
        .is_none()
    {
        return;
    }
    let Some(token) = original_argument_token(command, words, 1) else {
        return;
    };
    overrides
        .entry(token.span.start())
        .or_insert(ArgOverride::Kind(TokenKind::Keyword));
    for (role, kind) in member_source_role_kinds() {
        if let Some(indices) = grammar.source_member_arg_indices_for_role_in(
            member.as_bytes(),
            schema.words.arguments().slice_from(2),
            Some(ctx.context.authoring_query()),
            role,
        ) {
            for index in indices {
                if let Some(word) = index
                    .checked_add(2)
                    .and_then(|ordinal| words.operands.get(ordinal)?.as_ref()?.word.as_ref())
                {
                    insert_member_source_role(command, word, role, kind.clone(), overrides);
                }
            }
        }
    }
    for body in words.source_script_bodies(ctx.generation) {
        insert_member_source_role(
            command,
            body.original_container(),
            tcl_registry::ArgRole::Body,
            ArgOverride::BodyScript,
            overrides,
        );
    }
    if let Some((kind, indices)) = grammar.source_member_ref_indices_in(
        member.as_bytes(),
        schema.words.arguments().slice_from(2),
        Some(ctx.context.authoring_query()),
    ) {
        let kind = match kind {
            tcl_registry::definer::MemberRefKind::Class => ArgOverride::ClassNameRef,
            tcl_registry::definer::MemberRefKind::Method => ArgOverride::Kind(TokenKind::Method),
        };
        for index in indices {
            if let Some(word) = index
                .checked_add(2)
                .and_then(|ordinal| words.operands.get(ordinal)?.as_ref()?.word.as_ref())
            {
                insert_member_source_role(
                    command,
                    word,
                    tcl_registry::ArgRole::Name,
                    kind.clone(),
                    overrides,
                );
            }
        }
    }
}

/// The exact effective argument's own representative source token. Captured
/// prefixes and unmatched source extents have no current written anchor.
fn original_argument_token(
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    ordinal: usize,
) -> Option<tcl_lexer::Token> {
    let operand = words.operands.get(ordinal)?.as_ref()?;
    let word = operand.word.as_ref()?;
    let token = command
        .argv
        .iter()
        .find(|token| token.span == operand.span)?;
    word.tokens()
        .first()
        .is_some_and(|first| first.span == token.span)
        .then_some(*token)
}

/// Embedded conversion languages use the retained schema for every source
/// domain and only the exact original operand at each effective ordinal.
pub(super) fn insert_format_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let _ = words.with_source_schema(ctx.generation, |schema| {
        for found in schema
            .authored_source_format_arguments()
            .unwrap_or_default()
        {
            if let Some(token) = original_argument_token(command, words, found.index) {
                let kind = match found.kind {
                    tcl_registry::FormatType::Sprintf if found.scan => {
                        let Some(parts) = template_subtoken_spans(ctx, words, found) else {
                            continue;
                        };
                        ArgOverride::TemplateParts(parts)
                    }
                    tcl_registry::FormatType::Sprintf => ArgOverride::SprintfFormat,
                    tcl_registry::FormatType::Clock => ArgOverride::ClockFormat,
                    tcl_registry::FormatType::Binary => ArgOverride::BinaryFormat,
                    tcl_registry::FormatType::Regsub => {
                        let Some(captures) = template_subtoken_spans(ctx, words, found) else {
                            continue;
                        };
                        ArgOverride::TemplateParts(captures)
                    }
                };
                super::mark_literal_fragments(command, token.span, &kind, overrides);
            }
        }
    });
}

fn template_subtoken_spans(
    ctx: ScriptCtx<'_>,
    words: &crate::original_invocation::OriginalRegistryWords,
    format: tcl_registry::FormatStringArg,
) -> Option<Vec<(std::ops::Range<usize>, TokenKind)>> {
    let input = ctx.analysis?.resolved_input.as_ref()?;
    let operand = crate::format_template::FormatOperand::capture(words, input, format.index)?;
    let template = crate::format_template::FormatTemplate::from_operand(ctx.full_source, &operand)?;
    let mut parts = Vec::new();
    let mut push = |span: std::ops::Range<usize>, kind| {
        if !span.is_empty() {
            parts.push((
                template.original_offset(span.start)?..template.original_offset(span.end)?,
                kind,
            ));
        }
        Some(())
    };
    match format.kind {
        tcl_registry::FormatType::Sprintf if format.scan => {
            for field in tcl_syntax::scan::conversion_spans(&template.content, ctx.dialect) {
                push(field.percent, TokenKind::FormatPercent)?;
                if field.spec.xpg_index.is_some() {
                    let dollar = field.assignment.end - 1;
                    push(field.assignment.start..dollar, TokenKind::FormatWidth)?;
                    push(dollar..field.assignment.end, TokenKind::FormatPercent)?;
                } else {
                    push(field.assignment, TokenKind::FormatFlag)?;
                }
                push(field.width, TokenKind::FormatWidth)?;
                push(field.size, TokenKind::FormatFlag)?;
                push(field.conversion, TokenKind::FormatSpec)?;
            }
        }
        tcl_registry::FormatType::Regsub => {
            for (span, part) in tcl_cmd_core::regex::replacement_parts(template.content.as_bytes())
            {
                if let tcl_cmd_core::regex::ReplacementPart::Capture(_) = part {
                    let kind = if span.len() == 1 {
                        TokenKind::Operator
                    } else {
                        TokenKind::Number
                    };
                    push(span, kind)?;
                }
            }
        }
        _ => return None,
    }
    (!parts.is_empty()).then_some(parts)
}

/// Embedded patterns from the same retained schema and source-word geometry
/// for Native, hosted and explicitly Logical input. Missing receipts decline;
/// source text cannot reselect an embedded-language descriptor.
pub(super) fn insert_pattern_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let _ = words.with_source_schema(ctx.generation, |schema| {
        for pattern in schema
            .authored_source_pattern_arguments()
            .unwrap_or_default()
        {
            if pattern.kind == tcl_registry::patterns::PatternType::Regex
                && let Some(token) =
                    original_argument_token(command, words, usize::from(pattern.index))
            {
                super::mark_literal_fragments(
                    command,
                    token.span,
                    &ArgOverride::RegexPattern,
                    overrides,
                );
            }
        }
    });
}

fn push_original_extent(
    ctx: ScriptCtx<'_>,
    span: Span,
    kind: TokenKind,
    modifier: u32,
    entries: &mut Vec<super::Entry>,
) -> bool {
    let Some(text) = ctx.full_source.get(span.as_range()) else {
        return false;
    };
    if text.is_empty() {
        return false;
    }
    super::push_span_entries(
        ctx.full_source,
        ctx.line_index,
        span.start() as usize,
        text,
        kind,
        modifier,
        entries,
    );
    true
}

fn source_operand_word(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
) -> Option<&tcl_lexer::NativeWord> {
    let analysis = ctx.analysis?;
    let input = analysis.resolved_input.as_ref()?;
    if input.lexer_config() != ctx.config || input.context_registry().context() != ctx.context {
        return None;
    }
    let word = ctx
        .operand
        .and_then(|operand| operand.word.as_ref())
        .or_else(|| {
            let declared = ctx.declared_words?;
            (declared.resolved_input() == input).then_some(())?;
            declared.original_words().iter().skip(1).find(|word| {
                word.tokens()
                    .first()
                    .is_some_and(|first| first.span == tok.span)
            })
        })?;
    let image = analysis.retained_command_realm()?.original_source_image()?;
    if word.image() != image
        || word.config() != ctx.config
        || image.try_text().ok() != Some(ctx.full_source)
        || !analysis.matches_original_source_image(image, ctx.config)
    {
        return None;
    }
    Some(word)
}

pub(super) fn collect_loop_variables(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    entries: &mut Vec<super::Entry>,
) -> bool {
    if let Some(children) = ctx
        .operand
        .and_then(|operand| operand.input.as_ref())
        .and_then(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::original_list_elements_with_source_spans)
    {
        let mut emitted = children.is_empty();
        for (_, span) in children {
            if let Some(span) = span {
                emitted |= push_original_extent(
                    ctx,
                    span,
                    TokenKind::Variable,
                    super::MOD_DECLARATION,
                    entries,
                );
            }
        }
        return emitted;
    }
    if !ctx.lexical {
        return false;
    }
    let Some(fields) = source_operand_word(ctx, tok)
        .and_then(tcl_syntax::word_rules::original_static_word_list_elements)
    else {
        return false;
    };
    let mut emitted = fields.is_empty();
    for field in fields {
        if !field.value().is_empty()
            && !field.value().as_bytes().contains(&0)
            && let Some(span) = field.source_span()
        {
            emitted |= push_original_extent(
                ctx,
                span,
                TokenKind::Variable,
                super::MOD_DECLARATION,
                entries,
            );
        }
    }
    emitted
}

fn collect_source_parameter_fields(
    ctx: ScriptCtx<'_>,
    fields: &[tcl_syntax::word_rules::OriginalSourceListElement],
    entries: &mut Vec<super::Entry>,
) -> bool {
    let Some(parameters) = fields
        .iter()
        .map(tcl_syntax::word_rules::OriginalSourceListElement::elements)
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    if parameters.iter().any(|fields| {
        !(1..=2).contains(&fields.len())
            || fields[0].value().is_empty()
            || fields[0].value().as_bytes().contains(&0)
    }) {
        return false;
    }
    let mut emitted = parameters.is_empty();
    for fields in parameters {
        if let Some(span) = fields[0].source_span() {
            emitted |= push_original_extent(
                ctx,
                span,
                TokenKind::Parameter,
                super::MOD_DECLARATION,
                entries,
            );
        }
        if let Some(field) = fields.get(1)
            && let Some(span) = field.source_span()
        {
            let kind = if tcl_syntax::number::is_whole_number(field.value(), ctx.numbers) {
                TokenKind::Number
            } else {
                TokenKind::String
            };
            emitted |= push_original_extent(ctx, span, kind, 0, entries);
        }
    }
    emitted
}

/// The complete written word that starts at `tok`, lexed on its own from the
/// document. Display only: it carries no command, value or name receipt.
fn written_word(ctx: ScriptCtx<'_>, tok: tcl_lexer::Token) -> Option<tcl_lexer::NativeWord> {
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(ctx.full_source),
        tcl_lexer::word_span_at(ctx.full_source, tok.span),
        ctx.config,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (word.tokens().first()?.span == tok.span).then(|| word.clone())
}

/// Parameter fields of a command with no retained invocation, from its
/// written list.
fn collect_written_parameter_fields(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    entries: &mut Vec<super::Entry>,
) -> bool {
    ctx.original_words.is_none()
        && ctx.declared_words.is_none()
        && written_word(ctx, tok)
            .as_ref()
            .and_then(tcl_syntax::word_rules::original_static_word_list_elements)
            .is_some_and(|fields| collect_source_parameter_fields(ctx, &fields, entries))
}

pub(super) fn collect_parameter_fields(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    entries: &mut Vec<super::Entry>,
) -> bool {
    use tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameters;
    let Some(analysis) = ctx.analysis else {
        return collect_written_parameter_fields(ctx, tok, entries);
    };
    // One pass over the declarations for the whole walk; the first declaration
    // whose parameter word starts at a token keeps that key.
    let formal = ctx
        .walk
        .procedure_formals
        .get_or_init(|| {
            let mut index = FxHashMap::default();
            for declaration in analysis.original_procedure_declarations() {
                let Some(formal) = analysis.original_procedure_formals(declaration, ctx.registry)
                else {
                    continue;
                };
                let Some(first) = formal
                    .original_input()
                    .original_word()
                    .tokens()
                    .first()
                    .map(|first| first.span)
                else {
                    continue;
                };
                index.entry((first.start(), first.end())).or_insert(formal);
            }
            index
        })
        .get(&(tok.span.start(), tok.span.end()))
        .cloned()
        .or_else(|| {
            let input = ctx.operand?.input.as_ref()?.original_word_key()?;
            let dialect = match ctx.original_roles.formal_dialects.get(&tok.span.start()) {
                Some(dialect) => (*dialect)?,
                None => ctx.original_words?.dialect?,
            };
            SignatureSourceFormalParameters::from_original_input(input, dialect)
        });
    if let Some(formal) = formal {
        let mut emitted = formal.parameters().is_empty();
        for index in 0..formal.parameters().len() {
            if let Some(span) = formal
                .name_field(index)
                .and_then(|field| field.source_span())
            {
                emitted |= push_original_extent(
                    ctx,
                    span,
                    TokenKind::Parameter,
                    super::MOD_DECLARATION,
                    entries,
                );
            }
            if let Some(span) = formal
                .default_field(index)
                .and_then(|field| field.source_span())
            {
                let Some(text) = ctx.full_source.get(span.as_range()) else {
                    continue;
                };
                let kind = if tcl_syntax::number::is_whole_number(text, ctx.numbers) {
                    TokenKind::Number
                } else {
                    TokenKind::String
                };
                emitted |= push_original_extent(ctx, span, kind, 0, entries);
            }
        }
        return emitted;
    }
    if let Some(fields) = source_operand_word(ctx, tok)
        .and_then(tcl_syntax::word_rules::original_static_word_list_elements)
    {
        return collect_source_parameter_fields(ctx, &fields, entries);
    }
    let Some(word) = ctx.operand.and_then(|operand| operand.word.as_ref()) else {
        return collect_written_parameter_fields(ctx, tok, entries);
    };
    let mut emitted = false;
    for advice in analysis.original_vendor_variable_advice() {
        if let tcl_compiler::signature_scan::vendor_variable::VendorSourceVariableNameInput::Formal {parent,..}=advice.input()
            && parent.name_input().original_word()==word
            && advice.literal_units().is_some() {
            emitted |= push_original_extent(ctx,advice.span(),TokenKind::Parameter,super::MOD_DECLARATION,entries);
        }
    }
    emitted
}

pub(super) fn collect_lambda(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    entries: &mut Vec<super::Entry>,
    depth: u32,
) {
    if super::MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let Some(word) = source_operand_word(ctx, tok) else {
        super::classify_and_push_if(true, ctx, tok, entries);
        return;
    };
    let lexical_fields = if ctx.lexical {
        let Some(fields) = tcl_syntax::word_rules::original_static_word_list_elements(word) else {
            super::classify_and_push_if(true, ctx, tok, entries);
            return;
        };
        if !(2..=3).contains(&fields.len()) {
            super::classify_and_push_if(true, ctx, tok, entries);
            return;
        }
        Some(fields)
    } else {
        None
    };
    let Some(elements) = tcl_compiler::lambda_literal::split_original_lambda_literal(word) else {
        super::classify_and_push_if(true, ctx, tok, entries);
        return;
    };
    let formal = ctx.operand.and_then(|operand|operand.input.as_ref())
        .and_then(|input|input.original_list_element(0))
        .zip(ctx.original_words.and_then(|words|words.dialect))
        .and_then(|(input,dialect)|tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameterValue::from_original_input(&input,dialect));
    if let Some(formal) = formal {
        for index in 0..formal.parameters().len() {
            if let Some(span) = formal
                .name_field(index)
                .and_then(|field| field.source_span())
            {
                let _ = push_original_extent(
                    ctx,
                    span,
                    TokenKind::Parameter,
                    super::MOD_DECLARATION,
                    entries,
                );
            }
            if let Some(span) = formal
                .default_field(index)
                .and_then(|field| field.source_span())
            {
                let _ = push_original_extent(ctx, span, TokenKind::String, 0, entries);
            }
        }
    } else if ctx.lexical {
        let fields = lexical_fields
            .as_ref()
            .and_then(|fields| fields.first()?.elements());
        if !fields.is_some_and(|fields| collect_source_parameter_fields(ctx, &fields, entries)) {
            let _ = push_original_extent(ctx, elements.params, TokenKind::String, 0, entries);
        }
    } else {
        let _ = push_original_extent(ctx, elements.params, TokenKind::String, 0, entries);
    }
    if let Some(span) = elements.braced_body()
        && let Some(body) = ctx.full_source.get(span.as_range())
    {
        super::collect_script(
            ScriptCtx {
                oo_grammar: None,
                receiver_grammar: None,
                original_definition_parent: None,
                original_definition_members: None,
                enclosing_class: None,
                scoped_env: None,
                operand: None,
                original_words: None,
                declared_words: None,
                ..ctx
            },
            body,
            span.start(),
            entries,
            depth + 1,
            false,
        );
    } else if let Some(span) = elements.body {
        let _ = push_original_extent(ctx, span, TokenKind::String, 0, entries);
    }
    if let Some(span) = elements.namespace {
        let _ = push_original_extent(ctx, span, TokenKind::Namespace, 0, entries);
    }
}

pub(super) fn expanded_role_extents(
    words: &crate::original_invocation::OriginalRegistryWords,
) -> Vec<(Span, TokenKind, u32)> {
    let Some(roles) = words.roles.as_deref() else {
        return Vec::new();
    };
    roles
        .iter()
        .filter_map(|&(ordinal, role)| {
            let operand = words.operands.get(ordinal)?.as_ref()?;
            if operand.word.is_some()
                || operand.input.is_none()
                || words.arguments.get(ordinal)?.literal_bytes().is_none()
            {
                return None;
            }
            let kind = match role {
                tcl_registry::ArgRole::VarWrite => (TokenKind::Variable, super::MOD_DECLARATION),
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarCell => {
                    (TokenKind::Variable, 0)
                }
                tcl_registry::ArgRole::CommandName
                | tcl_registry::ArgRole::CommandNameProbe
                | tcl_registry::ArgRole::CommandPrefix => (TokenKind::Function, 0),
                tcl_registry::ArgRole::NamespaceName => (TokenKind::Namespace, 0),
                tcl_registry::ArgRole::Option | tcl_registry::ArgRole::OptionTerminator => {
                    (TokenKind::Decorator, 0)
                }
                tcl_registry::ArgRole::Keyword => (TokenKind::Keyword, 0),
                _ => return None,
            };
            Some((operand.span, kind.0, kind.1))
        })
        .collect()
}

pub(super) fn emit_expanded_role_extents(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    roles: &[(Span, TokenKind, u32)],
    entries: &mut Vec<super::Entry>,
) -> bool {
    if !matches!(
        tok.kind,
        tcl_lexer::TokenType::Str | tcl_lexer::TokenType::Esc
    ) {
        return false;
    }
    let end = super::end_over_terminator(ctx.full_source, tok.span.start(), tok.span.end());
    let mut owned = roles
        .iter()
        .filter(|(span, _, _)| span.start() >= tok.span.start() && span.end() <= end)
        .copied()
        .collect::<Vec<_>>();
    if owned.is_empty() {
        return false;
    }
    owned.sort_by_key(|(span, _, _)| span.start());
    let mut previous = tok.span.start();
    for (span, kind, modifier) in owned {
        if span.start() < previous {
            continue;
        }
        if previous < span.start() {
            let _ = push_original_extent(
                ctx,
                Span::new(previous, span.start()),
                TokenKind::String,
                0,
                entries,
            );
        }
        let _ = push_original_extent(ctx, span, kind, modifier, entries);
        previous = span.end();
    }
    if previous < end {
        let _ = push_original_extent(ctx, Span::new(previous, end), TokenKind::String, 0, entries);
    }
    true
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod readonly_member_tests;

#[cfg(test)]
mod original_missing_head_tests;

#[cfg(test)]
mod original_source_schema_tests;
