// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded readonly original command and body geometry for editor providers.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{
    ExecutablePart, ExecutableText, LexerConfig, NativeWord, SourceImage, Span, Token, WordKind,
};
use tcl_registry::{ArgRole, CommandRegistry};

/// Explicit compatibility ingress retains the supplied store and full grammar
/// before the analyser sees a presentation label. Existing analyses bypass it.
pub(crate) fn analyse_document(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> AnalysisResult {
    let generation = crate::context_for_dialect_profile(profile);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(profile, profile, context, config);
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[derive(Clone, Copy)]
struct BodyGeometry {
    extent: Span,
    content: Span,
}

/// Each region is original source geometry, with independently selected source
/// applicability. None of these spans grants execution, a frame or an edit.
pub(crate) struct SourceStructure {
    pub(crate) commands: Vec<SegmentedCommand>,
    pub(crate) bodies: Vec<Span>,
    pub(crate) scripts: Vec<(Span, u32)>,
    definition_members:
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
}

/// Readonly original script geometry from one complete retained analysis.
/// Source roles, lexical substitutions and declaration bodies supply syntax
/// regions only; no region proves dispatch, execution, a frame or an edit.
pub struct SourceSyntaxStructure {
    structure: SourceStructure,
    lexical_regions: Vec<SourceSyntaxRegion>,
}

/// One independently selected syntax interior and its original lexical spans.
/// All token offsets address the complete document, including nested regions.
/// Tokens supply syntax classification only, without execution or edit authority.
pub struct SourceSyntaxRegion {
    span: Span,
    tokens: Vec<Token>,
}

/// Readonly member selected by the genuine captured definition vocabulary.
/// This owns no global command identity, frame, formal value or edit permission.
pub(crate) struct SourceSyntaxDefinitionMember<'a> {
    selection: &'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion,
    words: &'a [NativeWord],
}

/// Missing correspondence within an otherwise genuine member source query.
#[derive(Debug)]
pub(crate) enum SourceSyntaxMemberUnavailable {
    /// More than one selected parent vocabulary reaches this original head.
    AmbiguousVocabulary,
    /// Supplied whole words or body extents differ from the retained member.
    WordCorrespondence,
}

impl SourceSyntaxDefinitionMember<'_> {
    /// Whole original member vector from the selected parent region.
    pub(crate) const fn original_words(&self) -> &[NativeWord] {
        self.words
    }

    /// Selected member scripts retain their original containers and ordinals.
    pub(crate) fn script_bodies(
        &self,
    ) -> Option<Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>>
    {
        self.selection.script_bodies(self.words)
    }
}

impl SourceSyntaxRegion {
    /// Original complete-document extent of this script interior.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Selected-grammar lexical tokens in the complete-document address space.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}

impl SourceSyntaxStructure {
    /// Capture the current document using its actual input, realm and grammar.
    /// Stale source or unavailable correspondence declines without recapture.
    #[must_use]
    pub fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let structure = SourceStructure::capture(source, Some(analysis), config)?;
        let image = SourceImage::document(source);
        let lexical_regions = structure
            .scripts
            .iter()
            .map(|&(span, depth)| {
                Some(SourceSyntaxRegion {
                    span,
                    tokens: tcl_lexer::source_region_tokens_in(
                        &image,
                        span,
                        config.at_depth(depth),
                    )
                    .ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            structure,
            lexical_regions,
        })
    }

    /// Original commands in the selected readonly syntax regions.
    #[must_use]
    pub fn commands(&self) -> &[SegmentedCommand] {
        &self.structure.commands
    }

    /// Select a member at its actual original head before any global command
    /// lookup. Ambiguous parent vocabularies decline without nominal fallback.
    pub(crate) fn definition_member_at(
        &self,
        head: u32,
    ) -> Result<Option<SourceSyntaxDefinitionMember<'_>>, SourceSyntaxMemberUnavailable> {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        let mut selected = None;
        for selection in &self.structure.definition_members {
            let Some(words) = selection.original_command_words_at(head) else {
                continue;
            };
            if selection.script_bodies(words).is_none() {
                continue;
            }
            if selected.is_some() {
                return Err(SourceSyntaxMemberUnavailable::AmbiguousVocabulary);
            }
            selected = Some(SourceSyntaxDefinitionMember { selection, words });
        }
        Ok(selected)
    }

    /// Original script interiors and their shared lexer nesting depth.
    #[must_use]
    pub fn script_regions(&self) -> &[(Span, u32)] {
        &self.structure.scripts
    }

    /// Original script interiors with already joined lexical source geometry.
    /// Consumers do not offset lexer-local tokens or recapture a child input.
    #[must_use]
    pub fn lexical_regions(&self) -> &[SourceSyntaxRegion] {
        &self.lexical_regions
    }
}

/// Source labels from one exact original variable component. These labels
/// describe selected syntax only; they grant no cell, runtime read or edit.
pub(crate) struct SourceVariableReference<'a> {
    pub(crate) root: &'a str,
    pub(crate) element_label: &'a str,
    pub(crate) whole_span: Span,
    pub(crate) token_span: Span,
}

fn reference_from_arena<'a>(
    source: &'a str,
    arena: &tcl_lexer::ExecutablePartArena,
    cursor: u32,
) -> Option<SourceVariableReference<'a>> {
    let part = arena.variable_part_at(cursor)?;
    let original = source.get(part.span.as_range())?;
    let config = arena.config();
    let root =
        tcl_syntax::naming::variable_reference_root_bytes(original.as_bytes(), config).ok()??;
    Some(SourceVariableReference {
        root: std::str::from_utf8(root).ok()?,
        element_label: tcl_syntax::naming::element_var_name_for_style(original, config.braced_var),
        whole_span: part.span,
        token_span: arena.source_span(part)?,
    })
}

fn variable_in_structure<'a>(
    source: &'a str,
    structure: &SourceStructure,
    config: LexerConfig,
    cursor: u32,
    expression_source: bool,
) -> Option<SourceVariableReference<'a>> {
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            let span = command.execution_span(source);
            span.start() <= cursor && cursor < span.end()
        })
        .min_by_key(|command| command.execution_span(source).len())?;
    if command.is_partial {
        return None;
    }
    let image = SourceImage::document(source);
    let plan =
        tcl_lexer::native_script_words_in(image.clone(), command.execution_span(source), config)
            .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let word = plan
        .commands
        .first()?
        .words
        .iter()
        .find(|word| word.span().start() <= cursor && cursor < word.span().end())?;
    if word.group().expand {
        return None;
    }
    if word.group().kind != WordKind::Braced {
        return reference_from_arena(source, word.executable_parts(), cursor);
    }
    // The actual source role/data classifier must already have proved this
    // position substituting. Braced data never reaches this expression path;
    // original script bodies have their own smaller selected command above.
    if !expression_source {
        return None;
    }
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        image,
        word.content_span().ok()?,
        tcl_lexer::SubstFlags::default(),
        config,
    )
    .ok()?;
    reference_from_arena(source, &arena, cursor)
}

/// Selected original substitution syntax in one complete retained analysis.
/// Unknown comment/body applicability or stale correspondence declines. The
/// caller independently chooses lexical or original-variable resolution.
pub(crate) fn original_variable_reference_at<'a>(
    source: &'a str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<SourceVariableReference<'a>> {
    let config = analysis.body_lexer_config?;
    if crate::definition::offset_is_inert(source, analysis, cursor)? {
        return None;
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)?;
    variable_in_structure(source, &structure, config, cursor, true)
}

/// Explicit standalone source syntax utility. It has no retained analysis and
/// offers no resolution, execution or edit authority. Actual editor consumers
/// use `original_variable_reference_at` instead of recapturing this input.
pub(crate) fn template_variable_reference_at(
    source: &str,
    config: LexerConfig,
    cursor: u32,
) -> Option<SourceVariableReference<'_>> {
    let structure = SourceStructure::capture(source, None, config)?;
    variable_in_structure(source, &structure, config, cursor, false)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapturePurpose {
    Readonly,
    SyntaxCompaction,
}

type PendingSourceRegion = (
    Span,
    u32,
    Option<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
);

struct SourceScriptContext<'a> {
    analysis: &'a AnalysisResult,
    depth: u32,
    purpose: CapturePurpose,
    definition_parent: Option<&'a tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    definition_members:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
}

struct SourceCaptureContext<'a> {
    source: &'a str,
    analysis: Option<&'a AnalysisResult>,
    config: LexerConfig,
    purpose: CapturePurpose,
    image: SourceImage,
}

impl SourceStructure {
    pub(crate) fn capture(
        source: &str,
        analysis: Option<&AnalysisResult>,
        config: LexerConfig,
    ) -> Option<Self> {
        Self::capture_with_purpose(source, analysis, config, CapturePurpose::Readonly)
    }

    /// Syntax compaction enters only independently closed original script
    /// roles. Readonly declarations and conditional schemas cannot authorise
    /// changes to descendant operand bytes.
    pub(crate) fn capture_for_syntax_compaction(
        source: &str,
        analysis: &AnalysisResult,
        config: LexerConfig,
    ) -> Option<Self> {
        Self::capture_with_purpose(
            source,
            Some(analysis),
            config,
            CapturePurpose::SyntaxCompaction,
        )
    }

    fn capture_with_purpose(
        source: &str,
        analysis: Option<&AnalysisResult>,
        config: LexerConfig,
        purpose: CapturePurpose,
    ) -> Option<Self> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        let image = SourceImage::document(source);
        let end = u32::try_from(source.len()).ok()?;
        if let Some(analysis) = analysis {
            if analysis.body_lexer_config != Some(config)
                || !analysis.matches_original_source_image(&image, config)
            {
                return None;
            }
        }
        let mut structure = Self {
            commands: Vec::new(),
            bodies: Vec::new(),
            scripts: Vec::new(),
            definition_members: Vec::new(),
        };
        let mut queue: VecDeque<PendingSourceRegion> =
            VecDeque::from([(Span::new(0, end), 0, None)]);
        if purpose == CapturePurpose::Readonly
            && let Some(analysis) = analysis
        {
            for body in declared_bodies(source, analysis, &image, config)? {
                structure.bodies.push(body.extent);
                queue.push_back((body.content, 1, None));
            }
            if let Some(realm) = analysis.retained_command_realm()
                && let Some(registry) = analysis.resolved_registry()
                && let Some(content) =
                    realm.original_incomplete_body_region_at(&image, config, end, registry)
            {
                structure.bodies.push(content);
                queue.push_back((content, 1, None));
            }
        }
        let capture = SourceCaptureContext {
            source,
            analysis,
            config,
            purpose,
            image,
        };
        let mut visited = BTreeSet::new();
        while let Some((region, depth, definition_parent)) = queue.pop_front() {
            let parent_span = definition_parent
                .as_ref()
                .map(|parent| (parent.content_span().start(), parent.content_span().end()));
            if depth > 256 || !visited.insert((region.start(), region.end(), parent_span)) {
                continue;
            }
            structure.capture_region(
                &capture,
                region,
                depth,
                definition_parent.as_ref(),
                &mut queue,
            )?;
        }
        structure
            .scripts
            .sort_by_key(|(span, depth)| (span.start(), span.end(), *depth));
        structure.scripts.dedup_by(|left, right| left.0 == right.0);
        structure
            .bodies
            .sort_by_key(|span| (span.start(), span.end()));
        structure.bodies.dedup();
        structure
            .commands
            .sort_by_key(|command| (command.span.start(), command.span.end()));
        structure
            .commands
            .dedup_by(|a, b| a.span == b.span && a.all_tokens == b.all_tokens);
        Some(structure)
    }

    fn capture_region(
        &mut self,
        capture: &SourceCaptureContext<'_>,
        region: Span,
        depth: u32,
        definition_parent: Option<&tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> Option<()> {
        let SourceCaptureContext {
            source,
            analysis,
            config,
            purpose,
            ..
        } = *capture;
        let script = source.get(region.as_range())?;
        self.scripts.push((region, depth));
        let commands =
            segment_commands_with_offset_and_config(script, region.start(), config.at_depth(depth));
        if let Ok(plan) = tcl_lexer::native_script_words_in(capture.image.clone(), region, config) {
            for command in &plan.commands {
                for word in &command.words {
                    for part in word.executable_parts().all_parts() {
                        if let ExecutablePart::Command { body } = part.part {
                            queue.push_back((body, depth + 1, definition_parent.cloned()));
                        }
                    }
                }
            }
        }
        if let Some(analysis) = analysis {
            let definition_members = analysis.resolved_input.as_ref().and_then(|input| {
                definition_parent?.definition_member_region(&input.context_registry(), region)
            });
            let context = SourceScriptContext {
                analysis,
                depth,
                purpose,
                definition_parent,
                definition_members: definition_members.as_ref(),
            };
            for command in &commands {
                self.command_bodies(source, command, &context, queue);
            }
            if purpose == CapturePurpose::Readonly
                && let Some(members) = definition_members
            {
                self.definition_members.push(members);
            }
        }
        self.commands.extend(commands);
        Some(())
    }

    pub(crate) fn command_at(&self, cursor: u32) -> Option<Span> {
        self.commands
            .iter()
            .map(|command| command.span)
            .filter(|span| span.start() <= cursor && cursor < span.end())
            .min_by_key(|span| span.end() - span.start())
    }

    fn readonly_command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) -> bool {
        let SourceScriptContext {
            analysis,
            depth,
            purpose,
            definition_members,
            ..
        } = *script;
        if purpose == CapturePurpose::Readonly
            && let Some(members) = definition_members
            && let Some(words) = command
                .argv
                .first()
                .and_then(|head| members.original_command_words_at(head.span.start()))
            && let Some(bodies) = members.script_bodies(words)
        {
            for body in bodies {
                self.bodies.push(body.extent_span());
                queue.push_back((
                    body.content_span(),
                    depth + 1,
                    body.definition_parent().cloned(),
                ));
            }
            return true;
        }
        if purpose == CapturePurpose::Readonly
            && let Some(declared) =
                tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                    source, analysis, command,
                )
        {
            for body in declared.source_script_bodies_for(
                tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::Syntax,
            ) {
                self.bodies.push(body.extent_span());
                queue.push_back((body.content_span(), depth + 1, None));
            }
            return true;
        }
        false
    }

    fn command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        let SourceScriptContext {
            analysis,
            depth,
            purpose,
            definition_parent,
            ..
        } = *script;
        let Some(config) = analysis.body_lexer_config else {
            return;
        };
        if self.readonly_command_bodies(source, command, script, queue) {
            return;
        }
        let Some(words) =
            crate::original_invocation::source_registry_words(source, analysis, command)
        else {
            return;
        };
        if purpose == CapturePurpose::SyntaxCompaction
            && (!matches!(
                words.source,
                crate::original_invocation::OriginalRegistrySource::Selected
            ) || !words.operands_preserve_source_lookup())
        {
            return;
        }
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        for body in words.source_script_bodies(&context) {
            self.bodies.push(body.container_extent_span());
            self.bodies.push(body.extent_span());
            let parent = (purpose == CapturePurpose::Readonly)
                .then(|| body.definition_parent_for(&context, definition_parent))
                .flatten();
            queue.push_back((body.content_span(), depth + 1, parent));
        }
        let list_argument = words
            .with_source_schema(&context, |schema| {
                schema
                    .authored_source_case_invocation()
                    .and_then(|(_, layout)| layout.clause_list_index)
            })
            .flatten();
        for &(argument, role) in words.roles.as_deref().unwrap_or_default() {
            if list_argument == Some(argument) {
                continue;
            }
            let Some(operand) = words.operands.get(argument).and_then(Option::as_ref) else {
                continue;
            };
            if role == ArgRole::Expr {
                if let Some(bodies) = words.source_expression_script_bodies(input) {
                    for body in bodies {
                        queue.push_back((
                            body.content_span(),
                            depth + 1,
                            definition_parent.cloned(),
                        ));
                    }
                }
            } else if role == ArgRole::LambdaLiteral {
                if let Some(body) = operand_body(source, operand, config) {
                    self.bodies.push(body.extent);
                }
                if let Some(word) = operand.word.as_ref()
                    && let Some(body) =
                        tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                            .and_then(|elements| elements.braced_body())
                {
                    queue.push_back((body, depth + 1, None));
                }
            } else if role.folds_as_block()
                && let Some(body) = operand_body(source, operand, config)
            {
                self.bodies.push(body.extent);
                // Script regions are issued by the shared Body/case owner.
                // Fold geometry for other authored roles remains separate.
            }
        }
    }
}

fn operand_body(
    source: &str,
    operand: &crate::original_invocation::OriginalOperandSource,
    config: LexerConfig,
) -> Option<BodyGeometry> {
    let image = SourceImage::document(source);
    if let Some(word) = &operand.word {
        (word.image() == &image && word.config() == config).then_some(())?;
        return word_body(word);
    }
    // An expanded child keeps its readonly extent, never a whole-word key.
    // Only the existing static-container facet and exact unchanged bytes can
    // lend this original extent to a source-only script-role walk.
    let input = operand.input.as_ref()?;
    let container = input.original_static_list_container()?;
    let parent = container.parent_word();
    if parent.image() != &image
        || parent.config() != config
        || image.bytes().get(operand.span.as_range()) != Some(input.bytes())
    {
        return None;
    }
    Some(BodyGeometry {
        extent: operand.span,
        content: operand.span,
    })
}

fn word_body(word: &NativeWord) -> Option<BodyGeometry> {
    if word.group().expand
        || (word.group().kind != WordKind::Braced
            && !(word.group().kind == WordKind::Quoted
                && word.executable_parts().all_parts().all(|part| {
                    matches!(part.part, ExecutablePart::Text(ExecutableText::Original))
                })))
    {
        return None;
    }
    let content = word.content_span().ok()?;
    // The fold ends at the original content boundary, keeping its closer visible.
    Some(BodyGeometry {
        extent: Span::new(word.group().span.start(), content.end()),
        content,
    })
}

/// Recover only lexical geometry of an independently owned declaration body.
/// The real source word is captured by the shared lexer; no name key is minted.
fn declared_body(image: &SourceImage, span: Span, config: LexerConfig) -> Option<BodyGeometry> {
    let whole = tcl_lexer::word_span_at(image.try_text().ok()?, span);
    let plan = tcl_lexer::native_script_words_in(image.clone(), whole, config).ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (plan.fatal_tail.is_none() && (word.group().span == span || word.span() == span))
        .then_some(())?;
    word_body(word)
}

fn declared_bodies(
    source: &str,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
) -> Option<Vec<BodyGeometry>> {
    use crate::original_declaration::OriginalDeclarationIdentity;
    let mut bodies = Vec::new();
    for declaration in analysis.original_procedure_declarations() {
        OriginalDeclarationIdentity::for_procedure("", source, analysis, declaration)?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    for class in analysis.original_class_declarations() {
        OriginalDeclarationIdentity::for_class("", source, analysis, class)?;
        if let Some(body) = declared_body(image, class.metadata().body_span, config) {
            bodies.push(body);
        }
        for method in class.metadata().original_members.declarations() {
            OriginalDeclarationIdentity::for_method("", source, analysis, class, method)?;
        }
        for special in class.metadata().original_special_members.declarations() {
            OriginalDeclarationIdentity::for_special("", source, analysis, class, special)?;
        }
        for property in class.metadata().original_properties.declarations() {
            OriginalDeclarationIdentity::for_property("", source, analysis, class, property)?;
        }
        retain_member_bodies(
            class.metadata().original_members.declarations(),
            class.metadata().original_special_members.declarations(),
            class.metadata().original_properties.declarations(),
            image,
            config,
            &mut bodies,
        )?;
    }
    for row in analysis.original_class_configurations() {
        (row.target().site().source.source_image() == image).then_some(())?;
        retain_member_bodies(
            row.members().declarations(),
            row.special_members().declarations(),
            row.properties().declarations(),
            image,
            config,
            &mut bodies,
        )?;
    }
    for row in analysis.original_object_configurations() {
        (row.target().site().source.source_image() == image).then_some(())?;
        retain_member_bodies(
            row.members().declarations(),
            row.special_members().declarations(),
            row.properties().declarations(),
            image,
            config,
            &mut bodies,
        )?;
    }
    for declaration in analysis.original_vendor_procedure_declarations() {
        declaration
            .name_input()
            .matches_source(image, config)
            .then_some(())?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    for declaration in analysis.original_vendor_class_declarations() {
        declaration
            .name_input()
            .matches_source(image, config)
            .then_some(())?;
        if let Some(body) = declared_body(image, declaration.metadata().body_span, config) {
            bodies.push(body);
        }
    }
    if analysis.allows_lexical_declaration_advice() {
        for span in analysis
            .all_procs
            .values()
            .map(|row| row.body_span)
            .chain(analysis.all_classes.values().map(|row| row.body_span))
        {
            if let Some(body) = declared_body(image, span, config) {
                bodies.push(body);
            }
        }
    }
    Some(bodies)
}

fn retain_member_bodies<'a>(
    methods: impl Iterator<Item = &'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata>,
    specials: impl Iterator<
        Item = &'a tcl_compiler::analyser::types::OriginalSourceSpecialMemberMetadata,
    >,
    properties: impl Iterator<Item = &'a tcl_compiler::analyser::types::OriginalSourcePropertyMetadata>,
    image: &SourceImage,
    config: LexerConfig,
    bodies: &mut Vec<BodyGeometry>,
) -> Option<()> {
    for method in methods {
        let declaration = method.declaration();
        let word = declaration.original_word();
        (word.image() == image
            && word.config() == config
            && declaration.site().source.source_image() == image)
            .then_some(())?;
        if method.forward_prefix().is_none()
            && let Some(word) = method.body_word()
        {
            (word.image() == image && word.config() == config).then_some(())?;
            if let Some(body) = word_body(word) {
                bodies.push(body);
            }
        }
    }
    for special in specials {
        let word = special.body_word();
        (word.image() == image
            && word.config() == config
            && special.site().source.source_image() == image)
            .then_some(())?;
        if let Some(body) = word_body(word) {
            bodies.push(body);
        }
    }
    for property in properties {
        let declaration = property.declaration();
        (declaration.site().source.source_image() == image
            && declaration.name_input().lexer_config() == config)
            .then_some(())?;
        for input in property.getter().into_iter().chain(property.setter()) {
            let tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) =
                input
            else {
                continue;
            };
            let word = key.original_word();
            (word.image() == image && word.config() == config).then_some(())?;
            if let Some(body) = word_body(word) {
                bodies.push(body);
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor_analysis(source: &str, style: tcl_dialect::BracedVarStyle) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let config = LexerConfig {
            braced_var: style,
            ..LexerConfig::for_profile(Some(profile))
        };
        Analyser::new()
            .with_resolved_input(ResolvedAnalysisInput::new(
                profile, profile, context, config,
            ))
            .analyse(source, "presentation-only")
    }

    #[test]
    fn original_variable_cursor_keeps_selected_roots_and_complete_source_geometry() {
        // Implementation contract: naming.core.selected-variable-cursor-syntax
        // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
        let source = concat!(
            "set seed café🙂\n",
            "puts ${a{b}c}\n",
            "puts ${scalar(open} ${scalar(open)tail} ${arr(k)} $arr($idx) ${cash$name} ${$n} ${café🙂}\n",
            "if {$seed} {puts $seed}\n",
            "puts \"$seed\" {literal $seed}\n",
            "# $seed\n",
        );
        for style in [
            tcl_dialect::BracedVarStyle::FirstClose,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ] {
            let analysis = cursor_analysis(source, style);
            assert!(analysis.allows_lexical_declaration_advice());
            for (needle, root, element) in [
                ("${scalar(open}", "scalar(open", "scalar(open"),
                (
                    "${scalar(open)tail}",
                    "scalar(open)tail",
                    "scalar(open)tail",
                ),
                ("${arr(k)}", "arr", "arr(k)"),
                ("$arr($idx)", "arr", "arr"),
                ("$idx", "idx", "idx"),
                ("${cash$name}", "cash$name", "cash$name"),
                ("$name", "cash$name", "cash$name"),
                ("${$n}", "$n", "$n"),
                ("${café🙂}", "café🙂", "café🙂"),
                ("$seed}", "seed", "seed"),
                ("$seed\"", "seed", "seed"),
            ] {
                let cursor = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
                let reference = original_variable_reference_at(source, &analysis, cursor)
                    .unwrap_or_else(|| panic!("missing {needle:?} under {style:?}"));
                assert_eq!(reference.root, root, "{needle}");
                assert_eq!(reference.element_label, element, "{needle}");
                let original = &source[reference.whole_span.as_range()];
                assert!(
                    tcl_lexer::word_parts::whole_var_ref(
                        original.as_bytes(),
                        analysis.body_lexer_config.unwrap()
                    )
                    .unwrap()
                    .is_some()
                );
                assert!(reference.whole_span.start() <= reference.token_span.start());
                assert!(reference.token_span.end() <= reference.whole_span.end());
            }
            let nested = u32::try_from(source.find("${a{b}c}").unwrap() + 2).unwrap();
            let reference = original_variable_reference_at(source, &analysis, nested).unwrap();
            assert_eq!(reference.root, if style.nests() { "a{b}c" } else { "a{b" });
            let tail = u32::try_from(source.find("}c}").unwrap() + 1).unwrap();
            assert_eq!(
                original_variable_reference_at(source, &analysis, tail).is_some(),
                style.nests()
            );
            for needle in ["literal $seed", "# $seed"] {
                let cursor =
                    u32::try_from(source.find(needle).unwrap() + needle.find('$').unwrap() + 1)
                        .unwrap();
                assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
            }
            assert!(
                original_variable_reference_at(&(source.to_owned() + " "), &analysis, nested)
                    .is_none()
            );
            let mut unavailable = analysis.clone();
            unavailable.resolved_input = None;
            assert!(original_variable_reference_at(source, &unavailable, nested).is_none());
            let mut foreign = analysis.clone();
            foreign.body_lexer_config.as_mut().unwrap().braced_var = if style.nests() {
                tcl_dialect::BracedVarStyle::FirstClose
            } else {
                tcl_dialect::BracedVarStyle::Tcl9Nesting
            };
            assert!(original_variable_reference_at(source, &foreign, nested).is_none());
        }
    }

    #[test]
    fn original_cursor_declines_malformed_data_and_native_resolution_promotion() {
        // Implementation contract: naming.core.selected-variable-cursor-syntax
        // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
        let source = "set seed 1\nputs ${missing\n";
        let analysis = cursor_analysis(source, tcl_dialect::BracedVarStyle::Tcl9Nesting);
        let cursor = u32::try_from(source.find("${missing").unwrap() + 2).unwrap();
        assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
        let source = "proc if args {}\nif 1 {puts $seed}\n";
        let analysis = cursor_analysis(source, tcl_dialect::BracedVarStyle::Tcl9Nesting);
        let cursor = u32::try_from(source.find("$seed").unwrap() + 1).unwrap();
        assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
        let source = "set seed 1\nputs $seed\n";
        let native = Analyser::new().analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.find("$seed").unwrap() + 1).unwrap();
        assert!(original_variable_reference_at(source, &native, cursor).is_some());
        assert!(!native.allows_lexical_declaration_advice());
        assert!(
            crate::definition::substituting_var_at_position(source, &native, 1, 6, cursor)
                .is_none()
        );
    }

    #[test]
    fn original_variable_cursor_retains_custom_reference_only_and_prefix_source_roles() {
        // Implementation contract: naming.core.selected-variable-cursor-syntax
        // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
        fn reference_only(
            _: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
        }
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "reference",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            script_timing_resolver: Some(reference_only),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        for (source, expected) in [
            ("reference {puts $value}", true),
            ("unknown {puts $value}", false),
            ("proc reference args {}\nreference {puts $value}", false),
            ("interp alias {} branch {} if 1\nbranch {puts $value}", true),
        ] {
            let analysis = analyse_document(
                source,
                profile,
                &registry,
                LexerConfig::for_profile(Some(profile)),
            );
            let cursor = u32::try_from(source.find("$value").unwrap() + 1).unwrap();
            let reference = original_variable_reference_at(source, &analysis, cursor);
            assert_eq!(
                reference.as_ref().map(|value| value.root),
                expected.then_some("value")
            );
            assert!(
                original_variable_reference_at(&(source.to_owned() + " "), &analysis, cursor)
                    .is_none()
            );
            let mut detached = analysis.clone();
            let input = detached.resolved_input.as_ref().unwrap();
            detached.resolved_input = Some(ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::new(
                    input
                        .context_registry()
                        .with_command_store(registry.snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(original_variable_reference_at(source, &detached, cursor).is_none());
        }
    }

    #[test]
    fn original_syntax_tokens_join_absolute_region_and_command_geometry() {
        // naming.cli.original-source-highlight-geometry
        // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
        let source =
            "set data {café🙂;\nnotACommand}\nif {1} {  puts [list café🙂]; puts $value  }";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let structure = SourceSyntaxStructure::capture(source, &analysis).unwrap();
            let body = structure
                .lexical_regions()
                .iter()
                .find(|region| {
                    &source[region.span().as_range()] == "  puts [list café🙂]; puts $value  "
                })
                .unwrap_or_else(|| panic!("missing genuine syntax body for {dialect}"));
            let start = u32::try_from(source.find("puts [list").unwrap()).unwrap();
            assert!(
                body.tokens()
                    .iter()
                    .any(|token| token.kind == tcl_lexer::TokenType::Esc
                        && token.span == Span::new(start, start + 4))
            );
            assert!(body.tokens().iter().any(|token| {
                token.kind == tcl_lexer::TokenType::Var
                    && &source[token.span.as_range()] == "$value"
            }));
            let nested = structure
                .lexical_regions()
                .iter()
                .find(|region| &source[region.span().as_range()] == "list café🙂")
                .unwrap();
            assert_eq!(&source[nested.tokens()[0].span.as_range()], "list");
            assert!(structure.commands().iter().any(|command| {
                command
                    .argv
                    .first()
                    .is_some_and(|head| head.span == nested.tokens()[0].span)
            }));
            assert!(
                !structure
                    .lexical_regions()
                    .iter()
                    .any(|region| &source[region.span().as_range()] == "café🙂;\nnotACommand")
            );
            for region in structure.lexical_regions() {
                assert!(
                    region
                        .tokens()
                        .iter()
                        .all(|token| region.span().start() <= token.span.start()
                            && token.span.end() <= region.span().end()
                            && source.get(token.span.as_range()).is_some())
                );
            }
            assert!(
                SourceSyntaxStructure::capture(&(source.to_owned() + " "), &analysis).is_none()
            );
        }
    }

    #[test]
    fn original_structure_retains_alias_body_and_bracket_geometry_after_ui_maps_clear() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = concat!(
            "proc p {} {puts [list inner]}\n",
            "interp alias {} branch {} if 1\n",
            "branch {\n    puts [list chosen]\n    puts again\n}\n",
        );
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.children.clear();
        let structure = SourceStructure::capture(source, Some(&analysis), config).unwrap();
        for name in ["list inner", "list chosen"] {
            let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
            let span = structure.command_at(cursor).unwrap();
            assert_eq!(&source[span.as_range()], name);
        }
        let start = u32::try_from(source.find("{\n    puts").unwrap()).unwrap();
        assert!(structure.bodies.iter().any(|body| body.start() == start));
        assert!(
            SourceStructure::capture(&source.replace("again", "other"), Some(&analysis), config,)
                .is_none()
        );
        let mut foreign = config;
        foreign.strict_quoting = !foreign.strict_quoting;
        assert!(SourceStructure::capture(source, Some(&analysis), foreign).is_none());
    }

    #[test]
    fn original_structure_uses_genuine_definition_wrappers_and_clears_ordinary_bodies() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "oo::class create C {self self {method café {} {puts wrapper}}; if 1 {method café {} {puts immediate}}; method m {} {method ordinary {} {puts data}}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let structure = SourceStructure::capture(source, Some(&analysis), config).unwrap();
        let wrapper = u32::try_from(source.find("{method café").unwrap()).unwrap();
        assert!(structure.bodies.iter().any(|body| body.start() == wrapper));
        for body in ["puts wrapper", "puts immediate"] {
            let offset = u32::try_from(source.find(body).unwrap()).unwrap();
            assert_eq!(
                source.get(structure.command_at(offset).unwrap().as_range()),
                Some(body)
            );
        }
        let offset = u32::try_from(source.find("puts data").unwrap()).unwrap();
        assert_ne!(
            source.get(structure.command_at(offset).unwrap().as_range()),
            Some("puts data")
        );
        let compact =
            SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
        assert!(
            !compact.bodies.iter().any(|body| body.start() == wrapper),
            "a readonly wrapper source role cannot grant syntax-compaction authority"
        );
        assert!(
            SourceStructure::capture(
                &source.replace("wrapper", "changed"),
                Some(&analysis),
                config
            )
            .is_none()
        );
        let mut foreign = config;
        foreign.strict_quoting ^= true;
        assert!(SourceStructure::capture(source, Some(&analysis), foreign).is_none());
    }

    #[test]
    fn original_case_and_lambda_geometry_skip_data_and_keep_dynamic_subject() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = concat!(
            "switch -- $choice {\n x {\n  puts [list arm]\n }\n}\n",
            "apply {{} {\n puts [list lambda]\n}}\n",
            "list {puts [list data]}\n",
        );
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let structure =
            SourceStructure::capture(source, Some(&analysis), analysis.body_lexer_config.unwrap())
                .unwrap();
        for name in ["list arm", "list lambda"] {
            let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
            assert_eq!(
                &source[structure.command_at(cursor).unwrap().as_range()],
                name,
            );
        }
        let cursor = u32::try_from(source.find("list data").unwrap()).unwrap();
        assert_ne!(
            &source[structure.command_at(cursor).unwrap().as_range()],
            "list data",
        );
    }
}

#[cfg(test)]
mod syntax_compaction_tests {
    use super::*;

    fn has_script(structure: &SourceStructure, source: &str, content: &str) -> bool {
        structure
            .scripts
            .iter()
            .any(|(span, _)| &source[span.as_range()] == content)
    }

    #[test]
    fn original_syntax_regions_keep_quiet_selected_bodies_and_source_currency() {
        // Implementation contract: naming.source.closed-syntax-script-regions
        // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
        let source = "proc p {} {if {1} {  puts [list quiet]  }}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.children.clear();
        let structure =
            SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
        assert!(has_script(
            &structure,
            source,
            "if {1} {  puts [list quiet]  }"
        ));
        assert!(has_script(&structure, source, "  puts [list quiet]  "));
        assert!(has_script(&structure, source, "list quiet"));
        assert!(
            SourceStructure::capture_for_syntax_compaction(
                &source.replace("quiet", "other"),
                &analysis,
                config,
            )
            .is_none()
        );
        assert!(
            SourceStructure::capture_for_syntax_compaction(
                source,
                &analysis,
                LexerConfig {
                    expand_syntax: !config.expand_syntax,
                    ..config
                },
            )
            .is_none()
        );
    }

    #[test]
    fn original_syntax_regions_do_not_promote_conditional_roles_or_declarations() {
        // Implementation contract: naming.source.closed-syntax-script-regions
        // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
        let source = "unavailable\nif {1} {  puts advisory  }\nputs [list lexical]\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let readonly = SourceStructure::capture(source, Some(&analysis), config).unwrap();
        assert!(has_script(&readonly, source, "  puts advisory  "));
        let closed =
            SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
        assert!(!has_script(&closed, source, "  puts advisory  "));
        assert!(has_script(&closed, source, "list lexical"));

        let source = "proc if {args} {}\nif {1} {  puts data  }\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let closed = SourceStructure::capture_for_syntax_compaction(
            source,
            &analysis,
            analysis.body_lexer_config.unwrap(),
        )
        .unwrap();
        assert!(!has_script(&closed, source, "  puts data  "));

        let source = "proc p {} \"puts \\n [list decoded]\"\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let closed = SourceStructure::capture_for_syntax_compaction(
            source,
            &analysis,
            analysis.body_lexer_config.unwrap(),
        )
        .unwrap();
        assert!(!has_script(&closed, source, "puts \\n [list decoded]"));
    }

    #[test]
    fn original_syntax_regions_reject_body_roles_when_arguments_replace_the_head() {
        // Implementation contract: naming.source.closed-syntax-script-regions
        // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
        // Native proof: naming.source.argument-replacement-body-data
        // docs/design/analysis/name-resolution-proofs/argument-replacement-body-data.md
        let source = concat!(
            "if [eval {rename if saved; proc if {condition body} ",
            "{set ::custom_ran CUSTOM; set ::custom_body $body; return $body}; list 1}] ",
            "{  set ::builtin_ran BUILTIN  ; list {  VALUE  }  }",
        );
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let readonly = SourceStructure::capture(source, Some(&analysis), config).unwrap();
        assert!(!has_script(
            &readonly,
            source,
            "  set ::builtin_ran BUILTIN  ; list {  VALUE  }  "
        ));
        let closed =
            SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
        assert!(!has_script(
            &closed,
            source,
            "  set ::builtin_ran BUILTIN  ; list {  VALUE  }  "
        ));
        assert!(has_script(
            &closed,
            source,
            concat!(
                "eval {rename if saved; proc if {condition body} ",
                "{set ::custom_ran CUSTOM; set ::custom_body $body; return $body}; list 1}",
            )
        ));
    }
}
