// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
}

impl OriginalTokenRoles {
    pub(super) fn capture(source: &str, analysis: Option<&AnalysisResult>) -> Self {
        let Some(analysis) = analysis else {
            return Self::default();
        };
        let image = SourceImage::document(source);
        let mut roles = Self {
            lexical_oo: analysis.allows_lexical_declaration_advice(),
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
        for declaration in analysis.original_vendor_procedure_declarations() {
            roles.vendor_name(
                &image,
                declaration.name_input(),
                declaration.purpose(),
                ArgOverride::ProcNameDef,
            );
        }
        for declaration in analysis.original_vendor_class_declarations() {
            roles.vendor_name(
                &image,
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
            roles.vendor_name(
                &image,
                declaration.name_input(),
                declaration.purpose(),
                ArgOverride::Kind(kind),
            );
        }
        for declaration in analysis.original_procedure_declarations() {
            roles.name(&image, declaration.name_input(), ArgOverride::ProcNameDef);
        }
        for declaration in analysis.original_class_declarations() {
            roles.name(&image, declaration.name_input(), ArgOverride::ClassNameDef);
            for member in declaration.metadata().original_members.declarations() {
                roles.member(&image, config, member);
            }
            for property in declaration.metadata().original_properties.declarations() {
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
        for invocation in &analysis.command_invocations {
            if matches!(
                crate::original_oo::class_for_invocation(analysis, source, invocation),
                std::ops::ControlFlow::Break(Some(_))
            ) {
                roles.class_heads.insert(invocation.range.start());
            }
        }
        roles
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
            self.names.insert(token.span.start(), (token.span, kind));
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
                overrides.insert(token.span.start(), *kind);
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
    if !ctx.lexical {
        return definition_member_head_word(ctx, command).is_some();
    }
    let Some(grammar) = ctx.oo_grammar else {
        return false;
    };
    let Some(analysis) = ctx.analysis else {
        return false;
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    if input.lexer_config() != ctx.config || input.context_registry().context() != ctx.context {
        return false;
    }
    let Some(image) = analysis
        .retained_command_realm()
        .and_then(|realm| realm.original_source_image())
    else {
        return false;
    };
    if image.try_text().ok() != Some(ctx.full_source)
        || !analysis.matches_original_source_image(image, ctx.config)
    {
        return false;
    }
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(ctx.full_source),
        ctx.config,
        command,
    );
    let Some(words) = tcl_compiler::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        command.span.start(),
        ctx.config,
    ) else {
        return false;
    };
    let Some(name) = words
        .first()
        .and_then(tcl_syntax::word_rules::original_static_word_ascii_presentation)
    else {
        return false;
    };
    std::str::from_utf8(&name)
        .ok()
        .is_some_and(|name| crate::oo_body::is_member(grammar, name))
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

pub(super) fn definition_member_bodies(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
) -> Option<Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>> {
    ctx.original_definition_members?
        .script_bodies(original_definition_member_words(ctx, command)?)
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
    let Some(roles) = words.roles.as_deref() else {
        return overrides;
    };
    match &words.source {
        crate::original_invocation::OriginalRegistrySource::Selected => {}
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
    insert_source_roles(
        roles,
        &words.arguments,
        |ordinal| original_argument_token(command, words, ordinal),
        &mut overrides,
    );
    insert_format_overrides(ctx, command, words, &mut overrides);
    insert_pattern_overrides(ctx, command, words, &mut overrides);
    let _ = words.with_source_schema(ctx.generation, |schema| {
        insert_source_schema_overrides(ctx, command, words, schema, &mut overrides);
    });
    overrides
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
            &roles,
            words.arguments(),
            |ordinal| {
                let word = words.argument_word(ordinal)?;
                let token = *word.tokens().first()?;
                (command.argv.get(ordinal.checked_add(1)?)?.span == token.span).then_some(token)
            },
            &mut overrides,
        );
    }
    overrides
}

fn insert_source_roles(
    roles: &[(usize, tcl_registry::ArgRole)],
    arguments: &[tcl_compiler::registry_invocation::EffectiveInvocationWord],
    token_at: impl Fn(usize) -> Option<tcl_lexer::Token>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    for &(ordinal, role) in roles {
        let Some(token) = token_at(ordinal) else {
            continue;
        };
        let static_value = arguments
            .get(ordinal)
            .is_some_and(|word| word.literal_bytes().is_some());
        let kind = match role {
            ArgRole::Body if token.kind == tcl_lexer::TokenType::Str => {
                Some(ArgOverride::BodyScript)
            }
            ArgRole::Expr if token.kind == tcl_lexer::TokenType::Str => {
                Some(ArgOverride::ExprScript)
            }
            ArgRole::VarWrite if static_value => Some(ArgOverride::VarDecl),
            ArgRole::VarRead if static_value => Some(ArgOverride::VarRef),
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
    words: &crate::original_invocation::OriginalRegistryWords,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let token_at = |ordinal| original_argument_token(command, words, ordinal);
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
        for option in scan.options.into_iter().filter(|option| option.available) {
            if let Some(token) = token_at(option.argument) {
                overrides.insert(token.span.start(), ArgOverride::Decorator);
            }
            if let Some(values) = option.values {
                for ordinal in values {
                    if words
                        .arguments
                        .get(ordinal)
                        .is_some_and(|value| value.literal_bytes().is_some())
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
    insert_logical_definition_overrides(ctx, command, words, schema, overrides);
    if ctx.dialect.is_irules() {
        for object in tcl_irules::original_source_object_operands(schema, None) {
            if let Some(token) = token_at(object.argument())
                && words
                    .arguments
                    .get(object.argument())
                    .is_some_and(|value| value.literal_bytes().is_some())
            {
                overrides.insert(token.span.start(), ArgOverride::Kind(TokenKind::Object));
            }
        }
    }
}

fn insert_source_case_overrides(
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let token_at = |ordinal| original_argument_token(command, words, ordinal);
    if let Some((spec, case)) = schema.authored_source_case_invocation()
        && let Some(ordinal) = case.clause_list_index
        && let Some(token) = token_at(ordinal)
        && token.kind == tcl_lexer::TokenType::Str
    {
        overrides.insert(
            token.span.start(),
            ArgOverride::CaseList(spec, case.mode == tcl_registry::spec::CaseMatchMode::Regexp),
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
                ArgOverride::RegexPattern,
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
    words: &crate::original_invocation::OriginalRegistryWords,
    schema: &tcl_registry::resolved_invocation::ResolvedInvocation<'_, '_>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::ArgRole;
    if original_logical_head(ctx, words).is_none() {
        return;
    }
    let selected = schema.authored_source_descriptors();
    if schema
        .semantics
        .traits
        .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
        && selected.command.definition_body.is_none()
    {
        for &(ordinal, role) in words.roles.as_deref().unwrap_or(&[]) {
            if role == ArgRole::Name
                && words
                    .arguments
                    .get(ordinal)
                    .is_some_and(|value| value.literal_bytes().is_some())
                && let Some(token) = original_argument_token(command, words, ordinal)
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
        && ctx
            .irules_top_level_declaration_heads
            .contains(&command.argv[0].span.start())
        && let Some(token) = original_argument_token(command, words, 0)
        && words
            .arguments
            .first()
            .and_then(|value| std::str::from_utf8(value.literal_bytes()?).ok())
            .is_some_and(super::is_event_name)
    {
        overrides.insert(token.span.start(), ArgOverride::Kind(TokenKind::Event));
    }
    if let Some((ordinal, declares)) = source_class_ordinal(schema)
        && words
            .arguments
            .get(ordinal)
            .is_some_and(|value| value.literal_bytes().is_some())
        && let Some(token) = original_argument_token(command, words, ordinal)
    {
        overrides.entry(token.span.start()).or_insert(if declares {
            ArgOverride::ClassNameDef
        } else {
            ArgOverride::ClassNameRef
        });
    }
    insert_logical_inline_member_overrides(ctx, command, words, schema, overrides);
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
    if matches!(
        grammar.family,
        tcl_registry::definer::DefinerFamily::Snit | tcl_registry::definer::DefinerFamily::Itcl
    ) {
        let (roles, complete) = schema.authored_source_argument_roles();
        if complete {
            return roles
                .into_iter()
                .find(|(_, role)| *role == tcl_registry::ArgRole::Name)
                .and_then(|(ordinal, _)| {
                    schema
                        .semantics
                        .argument_offset
                        .checked_add(usize::from(ordinal))
                })
                .map(|ordinal| (ordinal, true));
        }
    }
    None
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
    if !grammar.is_member(member) {
        return;
    }
    let Some(token) = original_argument_token(command, words, 1) else {
        return;
    };
    overrides
        .entry(token.span.start())
        .or_insert(ArgOverride::Kind(TokenKind::Keyword));
    let count = schema.words.arguments().exact_argv_len().unwrap_or(0);
    let Some(arguments) = (2..count)
        .map(|index| schema.words.arguments().literal_at(index))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    super::insert_oo_member_overrides(
        command,
        grammar,
        member,
        &arguments,
        2,
        Some(ctx.context.authoring_query()),
        overrides,
    );
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
                    tcl_registry::FormatType::Sprintf => ArgOverride::SprintfFormat,
                    tcl_registry::FormatType::Clock => ArgOverride::ClockFormat,
                    tcl_registry::FormatType::Binary => ArgOverride::BinaryFormat,
                    tcl_registry::FormatType::Regsub => ArgOverride::RegsubReplace,
                };
                super::mark_literal_fragments(command, token.span, kind, overrides);
            }
        }
    });
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
                    ArgOverride::RegexPattern,
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

fn source_operand_word<'a>(
    ctx: ScriptCtx<'a>,
    tok: tcl_lexer::Token,
) -> Option<&'a tcl_lexer::NativeWord> {
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
        .and_then(|input| input.original_list_elements_with_source_spans())
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
        .map(|field| field.elements())
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
            let kind = if super::is_number_literal(field.value(), ctx.numbers) {
                TokenKind::Number
            } else {
                TokenKind::String
            };
            emitted |= push_original_extent(ctx, span, kind, 0, entries);
        }
    }
    emitted
}

pub(super) fn collect_parameter_fields(
    ctx: ScriptCtx<'_>,
    tok: tcl_lexer::Token,
    entries: &mut Vec<super::Entry>,
) -> bool {
    use tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameters;
    let Some(analysis) = ctx.analysis else {
        return false;
    };
    let formal = analysis
        .original_procedure_declarations()
        .find_map(|declaration| {
            let formal = analysis.original_procedure_formals(declaration, ctx.registry)?;
            formal
                .original_input()
                .original_word()
                .tokens()
                .first()
                .is_some_and(|first| first.span == tok.span)
                .then_some(formal)
        })
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
                let kind = if super::is_number_literal(text, ctx.numbers) {
                    TokenKind::Number
                } else {
                    TokenKind::String
                };
                emitted |= push_original_extent(ctx, span, kind, 0, entries);
            }
        }
        return emitted;
    }
    if ctx.lexical {
        let Some(fields) = source_operand_word(ctx, tok)
            .and_then(tcl_syntax::word_rules::original_static_word_list_elements)
        else {
            return false;
        };
        return collect_source_parameter_fields(ctx, &fields, entries);
    }
    let Some(word) = ctx.operand.and_then(|operand| operand.word.as_ref()) else {
        return false;
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
                tcl_registry::ArgRole::VarRead => (TokenKind::Variable, 0),
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
mod tests {
    use super::*;
    use crate::semantic_tokens::tests::decode_semantic;
    use crate::semantic_tokens::{MOD_DEFINITION, full_with_cu_and_analysis};
    use tcl_compiler::analyser::Analyser;

    fn kind_at(
        source: &str,
        tokens: &[(u32, u32, u32, u32, u32)],
        at: usize,
        wanted: TokenKind,
        modifier: u32,
    ) -> bool {
        let position =
            tcl_lexer::LineIndex::new(source).position_at_utf16(u32::try_from(at).unwrap(), source);
        tokens.iter().any(|&(line, character, _, kind, modifiers)| {
            line == position.line
                && character == position.character.get()
                && kind == wanted as u32
                && modifiers == modifier
        })
    }

    #[test]
    fn original_vendor_token_roles_require_supported_header_units_and_current_source() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "proc helper {} {}\nproc p\\uD800 {} {}\nwhen HTTP_REQUEST {}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-irules");
        analysis.all_procs.clear();
        analysis.all_defined_symbols.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.defined_symbols.clear();
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        assert!(roles.current);
        let helper = u32::try_from(source.find("helper").unwrap()).unwrap();
        assert!(matches!(
            roles.names.get(&helper),
            Some((_, ArgOverride::ProcNameDef))
        ));
        let event = u32::try_from(source.find("HTTP_REQUEST").unwrap()).unwrap();
        assert!(matches!(
            roles.names.get(&event),
            Some((_, ArgOverride::Kind(TokenKind::Event)))
        ));
        let unsupported = u32::try_from(source.find(r"p\uD800").unwrap()).unwrap();
        assert!(!roles.names.contains_key(&unsupported));
        assert!(
            analysis
                .original_vendor_procedure_declarations()
                .any(|row| row.name_input().span().start() == unsupported
                    && row.name_input().literal_units(row.purpose()).is_none())
        );
        let stale = OriginalTokenRoles::capture(&format!("# displaced\n{source}"), Some(&analysis));
        assert!(!stale.current && stale.names.is_empty());
    }

    #[test]
    fn original_oo_roles_keep_opaque_declarations_and_actual_calls_without_ui_maps() {
        let source = "oo::class create C\\uD800 {method m\\uD800 {} {return A}}\noo::class create C\\uD801 {method m\\uD801 {} {return B}}\nC\\uD800 create a\nC\\uD801 create b\na m\\uD800\nb m\\uD801\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(analysis.original_class_declarations().count(), 2);
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        analysis.instance_classes.clear();
        analysis.created_instance_commands.clear();
        let profile = crate::environment_for_dialect("tcl8.6").analyser_profile();
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            source,
            profile,
            crate::registry_for_dialect("tcl8.6"),
            None,
            Some(&analysis),
        ));
        for written in ["C\\uD800", "C\\uD801"] {
            assert!(kind_at(
                source,
                &tokens,
                source.find(written).unwrap(),
                TokenKind::Class,
                MOD_DEFINITION
            ));
        }
        for written in ["m\\uD800", "m\\uD801"] {
            assert!(kind_at(
                source,
                &tokens,
                source.find(written).unwrap(),
                TokenKind::Method,
                MOD_DEFINITION
            ));
            assert!(kind_at(
                source,
                &tokens,
                source.rfind(written).unwrap(),
                TokenKind::Method,
                0
            ));
        }
    }

    #[test]
    fn original_oo_roles_do_not_use_final_members_for_an_earlier_or_stale_call() {
        let source = "oo::class create C {}; C create object\nobject later\noo::define C {method later {} {return OK}}\nobject later\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        assert!(roles.current);
        let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        let first = commands
            .iter()
            .find(|command| command.name() == "object")
            .unwrap();
        assert!(crate::receiver_identity::method_at_command(&analysis, source, first).is_none());
        analysis.all_classes.clear();
        let stale = OriginalTokenRoles::capture(&format!("{source} "), Some(&analysis));
        assert!(!stale.current);
        assert!(stale.names.is_empty());
        assert!(stale.class_heads.is_empty());
    }

    #[test]
    fn original_property_roles_keep_counted_declarations_separate_from_accessors() {
        let source =
            r"oo::configurable create C {property p\uD800 -kind readable p\uD801 -kind writable}";
        let mut analysis = Analyser::new().analyse(source, "tcl9.1");
        assert_eq!(
            analysis
                .original_class_declarations()
                .next()
                .unwrap()
                .metadata()
                .original_properties
                .declarations()
                .count(),
            2
        );
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        assert!(roles.current);
        for written in [r"p\uD800", r"p\uD801"] {
            let start = u32::try_from(source.find(written).unwrap()).unwrap();
            let (span, kind) = roles.names.get(&start).unwrap();
            assert_eq!(
                span,
                &Span::new(start, start + u32::try_from(written.len()).unwrap())
            );
            assert!(matches!(kind, ArgOverride::MemberName));
        }
        let stale = OriginalTokenRoles::capture(&format!("{source} "), Some(&analysis));
        assert!(!stale.current);
        assert!(stale.names.is_empty());
    }
}

#[cfg(test)]
mod readonly_member_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    // Implementation contract: naming.core.readonly-member-source-candidates
    // docs/design/analysis/name-resolution-proofs/readonly-member-source-candidates.md
    fn original_readonly_worker_value_does_not_retag_variable_syntax_as_member_declaration() {
        let source =
            "oo::class create C {foreach item {left right} {method $item {} {return body}}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let methods = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .original_members
            .declarations()
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2);
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        for method in methods {
            assert!(method.declaration().static_occurrence().is_none());
            assert!(
                !roles
                    .names
                    .contains_key(&method.declaration().original_word().span().start())
            );
        }
    }
}

#[cfg(test)]
mod original_missing_head_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_native_missing_head_never_authorises_reporting_oo_overlays() {
        // Implementation contract: naming.core.original-semantic-missing-head-terminal
        // docs/design/analysis/name-resolution-proofs/core-original-semantic-missing-head-terminal.md
        let source = "oo::class create C {method pick {} {}}; C create object; object pick";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("object pick").unwrap()).unwrap();
        let invocation = analysis
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap();
        invocation.original_name_input = None;
        invocation.original_lookup = None;
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        assert!(!roles.heads.contains(&offset));
        assert!(
            roles.has_head(offset),
            "missing Native producer cannot select a String OO table"
        );
        let stale = OriginalTokenRoles::capture(&format!("#{source}"), Some(&analysis));
        assert!(!stale.current);
        assert!(stale.has_head(offset));
        let hosted = Analyser::new().analyse("when HTTP_REQUEST {}", "f5-irules");
        assert!(!hosted.allows_lexical_declaration_advice());
        assert!(hosted.has_original_vendor_source_names());
        assert!(OriginalTokenRoles::capture("when HTTP_REQUEST {}", Some(&hosted)).has_head(0));
    }
}

#[cfg(test)]
mod original_source_schema_tests {
    use super::*;
    use crate::semantic_tokens::tests::decode_semantic;
    use crate::semantic_tokens::{MOD_DECLARATION, full_with_cu_and_analysis};
    use tcl_compiler::analyser::{Analyser, AnalysisResult};

    fn colour_at(
        source: &str,
        analysis: &AnalysisResult,
        offset: usize,
        kind: TokenKind,
        modifier: u32,
    ) -> bool {
        let profile = analysis.resolved_profile().unwrap();
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            source,
            profile,
            analysis.resolved_registry().unwrap(),
            None,
            Some(analysis),
        ));
        let position = tcl_lexer::LineIndex::new(source)
            .position_at_utf16(u32::try_from(offset).unwrap(), source);
        tokens.iter().any(|&(line, column, _, actual, mods)| {
            line == position.line
                && column == position.character.get()
                && actual == kind as u32
                && mods == modifier
        })
    }

    fn logical_role_input(
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> tcl_compiler::analyser::ResolvedAnalysisInput {
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-source-role-colours";
        let profile = profile.intern();
        tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    #[test]
    fn produced_prefix_colours_keep_owned_deferred_roles_and_static_target_words() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        for logical in [false, true] {
            let context = tcl_registry::model::ingress::resolve_environment("tcl9.1")
                .default_context_registry();
            let profile = if logical {
                tcl_dialect::DialectProfile::plain_tcl()
            } else {
                crate::profile_for_dialect("tcl9.1")
            };
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            for (source, needle, kind, modifier) in [
                (
                    "uplevel #0 [list upvar #0 originalName localName]",
                    "localName",
                    TokenKind::Variable,
                    MOD_DECLARATION,
                ),
                (
                    "after idle [list apply {{longname} {puts $longname}} 5]",
                    "longname",
                    TokenKind::Parameter,
                    MOD_DECLARATION,
                ),
                (
                    "after idle [list ::apply {{x} {puts $x}} 5]",
                    "puts",
                    TokenKind::Function,
                    0,
                ),
            ] {
                let analysis = Analyser::new()
                    .with_resolved_input(input.clone())
                    .analyse(source, profile.name);
                assert!(
                    colour_at(
                        source,
                        &analysis,
                        source.find(needle).unwrap(),
                        kind,
                        modifier
                    ),
                    "logical={logical}: {source}"
                );
            }
            for source in [
                "set data [list apply {{longname} {puts $longname}} 5]",
                "proc list args {}; after idle [list apply {{longname} {puts $longname}} 5]",
                "proc apply args {}; after idle [list apply {{longname} {puts $longname}} 5]",
                "after idle [list $head {{longname} {puts $longname}} 5]",
            ] {
                let analysis = Analyser::new()
                    .with_resolved_input(input.clone())
                    .analyse(source, profile.name);
                assert!(
                    !colour_at(
                        source,
                        &analysis,
                        source.find("longname").unwrap(),
                        TokenKind::Parameter,
                        MOD_DECLARATION
                    ),
                    "logical={logical}: {source}"
                );
            }
        }
    }

    #[test]
    fn logical_source_role_colours_keep_actual_option_context_and_reserved_data() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        for (environment, expected) in [("tcl8.4", false), ("tcl9.1", true)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let input = logical_role_input(std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            ));
            let source = "lsearch -stride 2 -regexp {a b} a+";
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, "reporting-only");
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find("-stride").unwrap(),
                    TokenKind::Decorator,
                    0
                ),
                expected,
                "{environment}"
            );
        }
        let input = logical_role_input(driver);
        for source in [
            "string equal -nocase -nocase -nocase",
            "string equal -length 3 -nocase -nocase",
            "interp alias {} compare {} string equal -nocase; compare -nocase -nocase",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            let tails = source
                .match_indices("-nocase")
                .map(|(offset, _)| offset)
                .collect::<Vec<_>>();
            for &offset in tails.iter().rev().take(2) {
                assert!(
                    !colour_at(source, &analysis, offset, TokenKind::Decorator, 0),
                    "{source}: {offset}"
                );
            }
            if source.starts_with("string equal -nocase") {
                assert!(colour_at(
                    source,
                    &analysis,
                    tails[0],
                    TokenKind::Decorator,
                    0
                ));
            }
        }
    }

    #[test]
    fn logical_source_role_colours_keep_effective_alias_operands_and_shadow_refusal() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let input = logical_role_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        for (source, expected) in [
            ("string equal -nocase A a", true),
            (
                "interp alias {} compare {} string equal; compare -nocase A a",
                true,
            ),
            ("proc string args {}; string equal -nocase A a", false),
            ("string $operation -nocase A a", false),
        ] {
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.rfind("-nocase").unwrap(),
                    TokenKind::Decorator,
                    0
                ),
                expected,
                "{source}"
            );
        }
        let expanded_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            tcl_lexer::LexerConfig {
                expand_syntax: true,
                ..input.lexer_config()
            },
        );
        let source = "string equal {*}$arguments -nocase A a";
        let analysis = Analyser::new()
            .with_resolved_input(expanded_input)
            .analyse(source, "reporting-only");
        assert!(!colour_at(
            source,
            &analysis,
            source.find("-nocase").unwrap(),
            TokenKind::Decorator,
            0
        ));
    }

    #[test]
    fn logical_source_role_colours_keep_nested_selectors_values_and_oo_members() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let input = logical_role_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        for (source, needle, kind, modifier) in [
            (
                "namespace ensemble configure ::E -namespace",
                "configure",
                TokenKind::Keyword,
                super::super::MOD_DEFAULT_LIBRARY,
            ),
            (
                "namespace ensemble configure ::E -namespace",
                "-namespace",
                TokenKind::Decorator,
                0,
            ),
            ("string is alnum abc", "alnum", TokenKind::EnumMember, 0),
            (
                "oo::define C method pick {arg} {return $arg}",
                "pick",
                TokenKind::Method,
                super::super::MOD_DEFINITION,
            ),
            (
                "oo::class create C {method pick {arg} {return $arg}}",
                "pick",
                TokenKind::Method,
                super::super::MOD_DEFINITION,
            ),
            (
                "switch $subject one {set local 1} default {set local 2}",
                "local",
                TokenKind::Variable,
                MOD_DECLARATION,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(
                colour_at(
                    source,
                    &analysis,
                    source.find(needle).unwrap(),
                    kind,
                    modifier
                ),
                "{source}: {needle}"
            );
        }
    }

    #[test]
    fn logical_format_colours_keep_actual_context_and_original_alias_geometry() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-format-colours";
        let profile = profile.intern();
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let source = "clock scan 2020 -format {%Y}";
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find('Y').unwrap(),
                    TokenKind::ClockSpec,
                    0
                ),
                expected,
                "{environment}"
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                full_with_cu_and_analysis(
                    source,
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
        }
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            driver,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for (source, expected) in [
            ("proc p {} {format {%d} 7}", true),
            ("interp alias {} fmt {} format; fmt {%d} 7", true),
            ("interp alias {} fmt {} format {%d}; fmt 7", false),
            ("proc format args {}; format {%d} 7", false),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find("%d").unwrap() + 1,
                    TokenKind::FormatSpec,
                    0
                ),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn retained_head_colours_keep_the_selected_full_context_descriptor_traits() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let modern = crate::profile_for_dialect("tcl9.1");
        let mut registry =
            tcl_registry::CommandRegistry::build_default().project_for_profile(modern);
        for (surface, traits) in [
            (
                tcl_dialect::model::SpecSurface::TCL84,
                tcl_registry::Traits::LANGUAGE_KEYWORD,
            ),
            (
                tcl_dialect::model::SpecSurface::TCL91,
                tcl_registry::Traits::OPERATOR_COMMAND,
            ),
        ] {
            registry.insert(tcl_registry::CommandSpec {
                name: "source-head-fixture",
                surface: Some(surface),
                traits,
                arity: tcl_registry::Arity::exact(0),
                ..tcl_registry::CommandSpec::DEFAULT
            });
        }
        assert!(
            registry
                .get("source-head-fixture")
                .unwrap()
                .traits
                .contains(tcl_registry::Traits::OPERATOR_COMMAND)
        );
        let registry = std::sync::Arc::new(registry);
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-head-traits";
        let profile = profile.intern();
        for (environment, kind) in [
            ("tcl8.4", TokenKind::Keyword),
            ("tcl9.1", TokenKind::Operator),
        ] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(actual.with_command_store(registry.clone()));
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let source = "source-head-fixture";
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, "presentation-only");
            let words = crate::original_invocation::source_registry_words(
                source,
                &analysis,
                &tcl_compiler::segmenter::segment_commands_with_offset_and_config(
                    source,
                    0,
                    analysis.body_lexer_config.unwrap(),
                )[0],
            )
            .unwrap();
            assert!(
                words
                    .with_source_schema(
                        &analysis.resolved_input.as_ref().unwrap().context_registry(),
                        |schema| schema.authored_source_descriptors().command.traits
                    )
                    .unwrap()
                    .contains(if kind == TokenKind::Keyword {
                        tcl_registry::Traits::LANGUAGE_KEYWORD
                    } else {
                        tcl_registry::Traits::OPERATOR_COMMAND
                    })
            );
            assert!(colour_at(source, &analysis, 0, kind, 0), "{environment}");
        }
    }

    fn original_head_evidence(source: &str, analysis: &AnalysisResult) -> String {
        let config = analysis.body_lexer_config.unwrap();
        let segment =
            tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words = crate::original_invocation::source_registry_words(source, analysis, &segment);
        let owned = words.as_ref().and_then(|words| words.head_source());
        let word = owned.and_then(|owned| owned.word());
        let value = word.and_then(tcl_syntax::word_rules::original_static_word_ascii_presentation);
        let boundaries = word.map(|word| {
            [0, 2, 8].map(|offset| {
                tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, offset)
            })
        });
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(analysis),
        ));
        format!(
            "schema={} original={word:?} value={value:?} boundaries={boundaries:?} tokens={tokens:?}",
            words.is_some()
        )
    }

    #[test]
    fn logical_qualified_head_colours_keep_original_decoded_boundaries() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-head-qualifiers";
        let profile = profile.intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for (source, tail) in [("::format {%d} 7", 2), (r"\u003a\u003aformat {%d} 7", 12)] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert!(
                colour_at(
                    source,
                    &analysis,
                    0,
                    TokenKind::Namespace,
                    super::super::MOD_DEFAULT_LIBRARY
                ),
                "{source}: {:?}",
                original_head_evidence(source, &analysis)
            );
            assert!(
                colour_at(
                    source,
                    &analysis,
                    tail,
                    TokenKind::Function,
                    super::super::MOD_DEFAULT_LIBRARY
                ),
                "{source}: {:?}",
                original_head_evidence(source, &analysis)
            );
        }
    }

    #[test]
    fn original_format_colours_keep_selected_availability_and_generation() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "clock scan 2020 -format {%Y}";
        let profile = crate::profile_for_dialect("tcl9.1");
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let offset = source.find("%Y").unwrap() + 1;
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(source, &analysis, offset, TokenKind::ClockSpec, 0),
                expected,
                "{environment}"
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                full_with_cu_and_analysis(
                    source,
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
        }
    }

    #[test]
    fn original_pattern_colours_keep_selected_availability_and_generation() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let source = "lsearch -regexp -stride 2 {a b} {a+}";
        let profile = crate::profile_for_dialect("tcl9.1");
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let offset = source.find("a+").unwrap();
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(source, &analysis, offset, TokenKind::Regexp, 0),
                expected,
                "{environment}"
            );
            assert!(
                full_with_cu_and_analysis(
                    &source.replace("a+", "b+"),
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                full_with_cu_and_analysis(
                    source,
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
        }
    }

    #[test]
    fn original_pattern_colours_keep_alias_written_operands_and_captured_refusal() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        for (source, expected) in [
            (
                "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
                true,
            ),
            (
                "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
                false,
            ),
            ("lsearch $mode {a b} {a+}", false),
            ("lsearch {*}$arguments {a+}", false),
            ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl9.1");
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find("a+").unwrap(),
                    TokenKind::Regexp,
                    0
                ),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_pattern_colours_keep_actual_context_over_a_profiled_store() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let source = "lsearch -regexp -stride 2 {a b} {a+}";
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-pattern-colours";
        let profile = profile.intern();
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(analysis.allows_lexical_declaration_advice());
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find("a+").unwrap(),
                    TokenKind::Regexp,
                    0
                ),
                expected,
                "{environment}"
            );
            assert!(
                full_with_cu_and_analysis(
                    &source.replace("a+", "b+"),
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                full_with_cu_and_analysis(
                    source,
                    profile,
                    context.commands(),
                    None,
                    Some(&analysis)
                )
                .data
                .is_empty()
            );
        }
    }

    #[test]
    fn logical_pattern_colours_keep_nested_and_alias_original_operand_anchors() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-pattern-anchors";
        let profile = profile.intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for (source, expected) in [
            ("proc p {} {regexp {a+} $value}", true),
            (
                "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
                true,
            ),
            (
                "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
                false,
            ),
            ("lsearch $mode {a b} {a+}", false),
            ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
        ] {
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(analysis.allows_lexical_declaration_advice());
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            assert_eq!(
                colour_at(
                    source,
                    &analysis,
                    source.find("a+").unwrap(),
                    TokenKind::Regexp,
                    0
                ),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn original_semantic_command_qualifiers_use_decoded_source_geometry() {
        // naming.core.original-command-head-colours
        // docs/design/analysis/name-resolution-proofs/original-command-head-colours.md
        for source in [r"::set marker 1", r"\u003a\u003aset marker 1"] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.command_invocations.clear();
            analysis.global_scope.variables.clear();
            let tokens = decode_semantic(&full_with_cu_and_analysis(
                source,
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
                None,
                Some(&analysis),
            ));
            let tail = u32::try_from(source.find("set").unwrap()).unwrap();
            assert!(
                tokens.iter().any(|&(line, column, length, kind, _)| {
                    line == 0
                        && column == 0
                        && length == tail
                        && kind == TokenKind::Namespace as u32
                }),
                "{source}: {tokens:?}"
            );
            assert!(
                tokens.iter().any(|&(line, column, length, kind, _)| {
                    line == 0 && column == tail && length == 3 && kind == TokenKind::Function as u32
                }),
                "{source}: {tokens:?}"
            );
        }
        let source = r"set\u0000::other marker 1";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0");
        analysis.command_invocations.clear();
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis),
        ));
        assert!(
            !tokens
                .iter()
                .any(|&(_, _, _, kind, _)| kind == TokenKind::Namespace as u32),
            "{tokens:?}"
        );
        assert!(tokens.iter().any(|&(line, column, _, kind, _)| {
            line == 0 && column == 0 && kind == TokenKind::Function as u32
        }));
    }

    #[test]
    fn original_semantic_method_roles_keep_canonical_worker_words() {
        // naming.core.original-member-body-colours
        // docs/design/analysis/name-resolution-proofs/original-member-body-colours.md
        let source = "oo::class create C {method p {argument {optional 7}} {set local $argument; return $local}}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let class = analysis.original_class_declarations().next().unwrap();
        let member = class
            .metadata()
            .original_members
            .declarations()
            .next()
            .unwrap();
        assert!(member.parameters_word().is_some() && member.body_word().is_some());
        assert!(member.source_dialect().is_some());
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        analysis.command_invocations.clear();
        for name in ["argument", "optional"] {
            assert!(
                colour_at(
                    source,
                    &analysis,
                    source.find(name).unwrap(),
                    TokenKind::Parameter,
                    MOD_DECLARATION
                ),
                "{name}"
            );
        }
        assert!(colour_at(
            source,
            &analysis,
            source.find("local").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
        assert!(
            full_with_cu_and_analysis(
                &format!("#{source}"),
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
                None,
                Some(&analysis)
            )
            .data
            .is_empty()
        );
    }

    #[test]
    fn original_semantic_source_roles_survive_reporting_map_erasure() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "set marker 1\nif {1} {set inside 2}\nproc p {a {b 5}} {return $a}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.variables.clear();
        for name in ["marker", "inside"] {
            assert!(
                colour_at(
                    source,
                    &analysis,
                    source.find(name).unwrap(),
                    TokenKind::Variable,
                    MOD_DECLARATION
                ),
                "{name}"
            );
        }
        let params = source.find("{a {b 5}}").unwrap();
        assert!(colour_at(
            source,
            &analysis,
            params + 1,
            TokenKind::Parameter,
            MOD_DECLARATION
        ));
        assert!(colour_at(
            source,
            &analysis,
            params + 4,
            TokenKind::Parameter,
            MOD_DECLARATION
        ));
        assert!(
            full_with_cu_and_analysis(
                &source.replace("marker", "absent"),
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
                None,
                Some(&analysis)
            )
            .data
            .is_empty()
        );
    }

    #[test]
    fn original_semantic_roles_decline_custom_and_deleted_command_cells() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        for source in [
            "proc set {name value} {return value}\nset marker 1\n",
            "rename set saved\nset marker 1\n",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(!colour_at(
                source,
                &analysis,
                source.find("marker").unwrap(),
                TokenKind::Variable,
                MOD_DECLARATION
            ));
        }
        let source = "set {*}{marker 1}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(colour_at(
            source,
            &analysis,
            source.find("marker").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
    }

    #[test]
    fn original_semantic_source_grammar_and_hosted_object_roles_remain_independent() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "\u{feff}set marker 1\n";
        let profile = crate::environment_for_dialect("tcl9.0").analyser_profile();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let analyse = |leading_bom| {
            let config = tcl_lexer::LexerConfig {
                leading_bom,
                ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
            };
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::clone(&context),
                config,
            );
            Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name)
        };
        assert!(colour_at(
            source,
            &analyse(tcl_lexer::LeadingBom::Skip),
            source.find("marker").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
        assert!(!colour_at(
            source,
            &analyse(tcl_lexer::LeadingBom::Content),
            source.find("marker").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
        let hosted = "when HTTP_REQUEST {pool safe; set target 1}\n";
        let analysis = Analyser::new().analyse(hosted, "f5-irules");
        assert!(colour_at(
            hosted,
            &analysis,
            hosted.find("safe").unwrap(),
            TokenKind::Object,
            0
        ));
        assert!(colour_at(
            hosted,
            &analysis,
            hosted.find("target").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
        let shadow = "proc pool {name} {return $name}\nwhen HTTP_REQUEST {pool safe}\n";
        let custom = Analyser::new().analyse(shadow, "f5-irules");
        assert!(!colour_at(
            shadow,
            &custom,
            shadow.find("safe").unwrap(),
            TokenKind::Object,
            0
        ));
    }
    #[test]
    fn original_expression_colours_require_actual_source_roles_and_lambda_parent_geometry() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let data = "set data {1 + 2}\n";
        let analysis = Analyser::new().analyse(data, "tcl8.6");
        assert!(!colour_at(
            data,
            &analysis,
            data.find('+').unwrap(),
            TokenKind::Operator,
            0
        ));
        let expression = "expr {1 + 2}\n";
        let analysis = Analyser::new().analyse(expression, "tcl8.6");
        assert!(colour_at(
            expression,
            &analysis,
            expression.find('+').unwrap(),
            TokenKind::Operator,
            0
        ));
        let shadow = "proc expr {value} {return $value}\nexpr {1 + 2}\n";
        let analysis = Analyser::new().analyse(shadow, "tcl8.6");
        assert!(!colour_at(
            shadow,
            &analysis,
            shadow.find('+').unwrap(),
            TokenKind::Operator,
            0
        ));
        let lambda = "apply {x {return $x}} 1\n";
        let analysis = Analyser::new().analyse(lambda, "tcl8.6");
        assert!(colour_at(
            lambda,
            &analysis,
            lambda.find("{x").unwrap() + 1,
            TokenKind::Parameter,
            MOD_DECLARATION
        ));
        assert!(colour_at(
            lambda,
            &analysis,
            lambda.find("$x").unwrap(),
            TokenKind::Variable,
            0
        ));
    }
    #[test]
    fn refused_logical_head_schema_keeps_generic_colour_without_library_flags() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let input = logical_role_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        for (source, head) in [
            ("proc string args {}; string equal A a", "string"),
            ("rename if {}; if 1 {}", "if"),
            ("unprovidedCommand argument", "unprovidedCommand"),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            let offset = source.rfind(head).unwrap();
            assert!(
                colour_at(source, &analysis, offset, TokenKind::Function, 0),
                "{source}"
            );
            assert!(
                !colour_at(source, &analysis, offset, TokenKind::Keyword, 0),
                "{source}"
            );
            assert!(
                !colour_at(
                    source,
                    &analysis,
                    offset,
                    TokenKind::Function,
                    super::super::MOD_DEFAULT_LIBRARY
                ),
                "{source}"
            );
        }
        let source = "unknown argument";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "reporting-only");
        assert!(colour_at(
            source,
            &analysis,
            0,
            TokenKind::Function,
            super::super::MOD_DEFAULT_LIBRARY
        ));
    }
    #[test]
    fn logical_list_roles_keep_complete_unicode_escaped_and_semicolon_fields() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let input = logical_role_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        for (source, field, kind) in [
            (
                "proc p {é {lo\\u006eng 5} semi;colon} {}",
                "é",
                TokenKind::Parameter,
            ),
            (
                "proc p {é {lo\\u006eng 5} semi;colon} {}",
                r"lo\u006eng",
                TokenKind::Parameter,
            ),
            ("proc p {semi;colon} {}", "semi;colon", TokenKind::Parameter),
            ("foreach {é long} {} {}", "é", TokenKind::Variable),
            (
                "apply {{é {lo\\u006eng 5}} {return $é $long}}",
                r"lo\u006eng",
                TokenKind::Parameter,
            ),
            (
                "after idle [list apply {{é {lo\\u006eng 5}} {return $é $long}} 5]",
                r"lo\u006eng",
                TokenKind::Parameter,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(
                colour_at(
                    source,
                    &analysis,
                    source.find(field).unwrap(),
                    kind,
                    MOD_DECLARATION
                ),
                "{source}: {field}"
            );
        }
    }

    #[test]
    fn logical_list_roles_decline_cooked_subranges_opaque_and_malformed_lambdas() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let input = logical_role_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        for (source, field) in [
            (r"proc p {escaped\ name} {}", "escaped"),
            (r"proc p {\uD800} {}", r"\uD800"),
            (
                "apply {{argument} {return $argument} extra fourth}",
                "argument",
            ),
            ("apply $lambda argument", "argument"),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(
                !colour_at(
                    source,
                    &analysis,
                    source.find(field).unwrap(),
                    TokenKind::Parameter,
                    MOD_DECLARATION
                ),
                "{source}"
            );
        }
        let source = "proc p {argument} {}";
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "reporting-only");
        analysis.body_lexer_config = Some(tcl_lexer::LexerConfig {
            list_parse: tcl_dialect::ListParse::Lenient,
            ..analysis.body_lexer_config.unwrap()
        });
        assert!(!colour_at(
            source,
            &analysis,
            source.find("argument").unwrap(),
            TokenKind::Parameter,
            MOD_DECLARATION
        ));
    }
}
