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
    // Refusal-only source contexts: none establishes an entered or new frame.
    pub(crate) caller_context_unavailable_bodies: Vec<Span>,
    definition_members:
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
    package_census: Option<PackageCensusState>,
}

#[derive(Default)]
struct PackageCensusState {
    references:
        Vec<tcl_compiler::registry_invocation::source_structure::OriginalSourcePackageReference>,
    unavailable: Vec<Span>,
    syntax_unavailable: Vec<Span>,
    package_unavailable: Vec<Span>,
}

/// Completeness of independently selected original script syntax only.
/// Complete supplies no dispatch/call-graph closure, execution or Normal result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSyntaxCoverageApplicability {
    /// All independently selected script syntax is represented.
    Complete,
    /// Some selected script syntax is unavailable or unrepresented.
    Incomplete,
}

/// Checked original syntax inventory tied to one retained whole input.
/// This is separate from the ordinary readonly partial source inventory.
pub struct SourceSyntaxCoverage {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    structure: SourceStructure,
}

impl SourceSyntaxCoverage {
    /// Inspect genuine selected body roles, including deferred definition
    /// syntax, and preserve all unavailable/malformed/unrepresented regions.
    pub fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        if config != input.lexer_config() || !realm.matches_resolved_analysis_input(input) {
            return None;
        }
        let structure = SourceStructure::capture_with_purpose(
            source,
            Some(analysis),
            config,
            CapturePurpose::CheckedSyntax,
        )?;
        Some(Self {
            image: SourceImage::document(source),
            input: input.clone(),
            structure,
        })
    }

    fn matches(&self, source: &str, analysis: &AnalysisResult) -> bool {
        self.image == SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.matches_original_source_image(&self.image, self.input.lexer_config())
    }

    /// Captured commands remain useful even when selected syntax is incomplete.
    /// Callers retain independent actual call/declaration applicability checks.
    pub fn commands<'a>(
        &'a self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<&'a [SegmentedCommand]> {
        self.matches(source, analysis)
            .then_some(self.structure.commands.as_slice())
    }

    /// Complete means every represented selected script syntax body was
    /// inspected; missing schemas, including absent heads, stay incomplete.
    pub fn applicability(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<SourceSyntaxCoverageApplicability> {
        if !self.matches(source, analysis) {
            return None;
        }
        let state = self.structure.package_census.as_ref()?;
        Some(
            if state.unavailable.is_empty() && state.syntax_unavailable.is_empty() {
                SourceSyntaxCoverageApplicability::Complete
            } else {
                SourceSyntaxCoverageApplicability::Incomplete
            },
        )
    }
}

/// Source-only selected package syntax with explicit incomplete coverage.
/// A complete negative says nothing about installed/runtime package state.
pub(crate) struct SourcePackagePresenceCensus {
    coverage: SourceSyntaxCoverage,
}

impl SourcePackagePresenceCensus {
    pub(crate) fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        Some(Self {
            coverage: SourceSyntaxCoverage::capture(source, analysis)?,
        })
    }

    /// Positive genuine selected references survive incomplete other regions.
    /// Incomplete/no-representation is None, never a complete source negative.
    pub(crate) fn already_names(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        package: &str,
    ) -> Option<bool> {
        if !self.coverage.matches(source, analysis) {
            return None;
        }
        let state = self.coverage.structure.package_census.as_ref()?;
        if state
            .references
            .iter()
            .any(|reference| reference.matches_ascii(package))
        {
            return Some(true);
        }
        (state.unavailable.is_empty() && state.package_unavailable.is_empty()).then_some(false)
    }
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
    /// Original readonly command vectors without the independent lexical-token
    /// inventory. The complete current analysis/input/image/configuration owns
    /// the same selected script regions as `capture`; no binding, execution,
    /// completeness or edit authority follows from an individual command.
    #[must_use]
    pub fn capture_commands(
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<Vec<SegmentedCommand>> {
        let (_, config) = tcl_compiler::source_graph::current_analysis(source, analysis)?;
        Some(SourceStructure::capture(source, Some(analysis), config)?.commands)
    }

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
    pub(crate) name_span: Span,
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
    let tcl_lexer::ExecutablePart::Variable { name, .. } = part.part else {
        return None;
    };
    arena.bytes(name)?.starts_with(root).then_some(())?;
    let name_span = Span::new(
        name.start(),
        name.start().checked_add(u32::try_from(root.len()).ok()?)?,
    );
    Some(SourceVariableReference {
        root: std::str::from_utf8(root).ok()?,
        name_span,
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

/// Partial cursor syntax from the current source's innermost genuinely owned
/// lexical script region. Missing source/phase correspondence stays unavailable;
/// no completed name, variable frame or selected binding follows.
pub(crate) fn original_variable_cursor_prefix(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Result<
    Option<tcl_lexer::word_parts::VariableCursorPrefix>,
    tcl_lexer::word_parts::VariableCursorUnavailable,
> {
    use tcl_lexer::word_parts::{VariableCursorUnavailable, variable_cursor_prefix_in};
    let config = analysis
        .body_lexer_config
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) || cursor as usize > source.len() {
        return Err(VariableCursorUnavailable::SourceGeometry);
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)
        .ok_or(VariableCursorUnavailable::LexicalStream)?;
    let region = structure
        .scripts
        .iter()
        .filter(|(span, _)| span.start() <= cursor && cursor <= span.end())
        .min_by_key(|(span, _)| span.len())
        .map(|(span, _)| *span)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let prefix = variable_cursor_prefix_in(&image, region, cursor, config)?;
    if prefix.is_some() {
        return Ok(prefix);
    }
    // Complete selected expression references retain an independent original
    // component even when their enclosing word is braced source syntax.
    let Some(reference) = cursor
        .checked_sub(1)
        .and_then(|at| original_variable_reference_at(source, analysis, at))
    else {
        return Ok(None);
    };
    variable_cursor_prefix_in(&image, reference.whole_span, cursor, config)
}

/// Original word-cursor geometry in the current innermost selected script.
/// Braced data and unsupported expression/child phases cannot create a script
/// owner; the returned argv ordinal is lexical source data only.
pub(crate) fn original_word_cursor_prefix(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Result<
    Option<tcl_lexer::word_parts::WordCursorPrefix>,
    tcl_lexer::word_parts::WordCursorUnavailable,
> {
    use tcl_lexer::word_parts::{WordCursorUnavailable, word_cursor_prefix_in};
    let config = analysis
        .body_lexer_config
        .ok_or(WordCursorUnavailable::SourceGeometry)?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) || cursor as usize > source.len() {
        return Err(WordCursorUnavailable::SourceGeometry);
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)
        .ok_or(WordCursorUnavailable::LexicalStream)?;
    let region = structure
        .scripts
        .iter()
        .filter(|(span, _)| span.start() <= cursor && cursor <= span.end())
        .min_by_key(|(span, _)| span.len())
        .map(|(span, _)| *span)
        .ok_or(WordCursorUnavailable::SourceGeometry)?;
    word_cursor_prefix_in(&image, region, cursor, config)
}

/// Reborrowed original variable geometry for one immutable analysis source.
/// This view owns syntax only; callers retain name resolution and edit guards.
pub(crate) struct OriginalVariableReferences<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    config: LexerConfig,
    structure: SourceStructure,
}

impl<'a> OriginalVariableReferences<'a> {
    pub(crate) fn capture(source: &'a str, analysis: &'a AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let structure = SourceStructure::capture(source, Some(analysis), config)?;
        Some(Self {
            source,
            analysis,
            config,
            structure,
        })
    }

    pub(crate) fn reference_at(&self, cursor: u32) -> Option<SourceVariableReference<'a>> {
        if crate::definition::offset_is_inert(self.source, self.analysis, cursor)? {
            return None;
        }
        variable_in_structure(self.source, &self.structure, self.config, cursor, true)
    }
}

/// Original receiver and static selector syntax at a current command point.
/// These values grant no object identity, dispatch, visibility or edit coverage.
pub(crate) fn original_method_site_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<(String, String, bool)> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    if crate::inert_text::offset_in_physical_comment_in_analysis(source, analysis, cursor)? {
        return None;
    }
    let config = current.config();
    let structure = SourceStructure::capture(source, Some(analysis), config)?;
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
    if tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, command,
    )
    .is_some_and(|words| {
        words
            .written_argument_roles()
            .into_iter()
            .any(|(argument, role)| argument == 0 && role == ArgRole::Expr)
    }) {
        return None;
    }
    let image = SourceImage::document(source);
    let plan =
        tcl_lexer::native_script_words_in(image, command.execution_span(source), config).ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() {
        return None;
    }
    let head = command.words.first()?;
    let selector = command.words.get(1)?;
    if command.words.iter().any(|word| word.group().expand)
        || !(selector.span().start() <= cursor && cursor < selector.span().end())
    {
        return None;
    }
    let method = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        selector,
    )?)
    .ok()?;
    if method.contains('\0') {
        return None;
    }
    let arena = head.executable_parts();
    if let [part] = arena.list(arena.root()) {
        match part.part {
            ExecutablePart::Variable { .. } => {
                let written = arena.bytes(part.span)?;
                let reference = tcl_lexer::word_parts::whole_var_ref(written, config).ok()??;
                let name = if reference.index.is_some() {
                    let name_at =
                        (reference.name.as_ptr() as usize).checked_sub(written.as_ptr() as usize)?;
                    std::str::from_utf8(written.get(name_at..)?).ok()?
                } else {
                    std::str::from_utf8(reference.name).ok()?
                };
                if name.contains('\0') {
                    return None;
                }
                return Some((name.to_owned(), method, true));
            }
            ExecutablePart::Command { .. } => {
                let written = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
                return Some((written.to_owned(), method, false));
            }
            ExecutablePart::Text(_) => {}
            ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => return None,
        }
    }
    let receiver = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        head,
    )?)
    .ok()?;
    if receiver.contains('\0') {
        return None;
    }
    Some((receiver, method, false))
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
    CheckedSyntax,
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
            caller_context_unavailable_bodies: Vec::new(),
            definition_members: Vec::new(),
            package_census: (purpose == CapturePurpose::CheckedSyntax)
                .then(PackageCensusState::default),
        };
        let mut queue: VecDeque<PendingSourceRegion> =
            VecDeque::from([(Span::new(0, end), 0, None)]);
        if purpose == CapturePurpose::Readonly
            && let Some(analysis) = analysis
        {
            for body in declared_bodies(source, analysis, &image, config)? {
                structure.bodies.push(body.extent);
                structure
                    .caller_context_unavailable_bodies
                    .push(body.content);
                queue.push_back((body.content, 1, None));
            }
            if let Some(realm) = analysis.retained_command_realm()
                && let Some(registry) = analysis.resolved_registry()
                && let Some(content) =
                    realm.original_incomplete_body_region_at(&image, config, end, registry)
            {
                structure.bodies.push(content);
                structure.caller_context_unavailable_bodies.push(content);
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
            if depth > 256 {
                if let Some(census) = &mut structure.package_census {
                    census.unavailable.push(region);
                }
                continue;
            }
            if !visited.insert((region.start(), region.end(), parent_span)) {
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
        let original_plan =
            tcl_lexer::native_script_words_in(capture.image.clone(), region, config);
        if let Some(census) = &mut self.package_census {
            if !matches!(&original_plan, Ok(plan) if plan.fatal_tail.is_none()) {
                census.unavailable.push(region);
            }
        }
        if let Ok(plan) = original_plan {
            for command in &plan.commands {
                for word in &command.words {
                    for part in word.executable_parts().all_parts() {
                        if matches!(part.part, ExecutablePart::ParseError(_)) {
                            if let Some(census) = &mut self.package_census {
                                census.unavailable.push(word.span());
                            }
                        }
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
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
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
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
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
        if purpose == CapturePurpose::CheckedSyntax {
            self.census_command_bodies(source, command, script, queue);
            return;
        }
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
            if !body.inherits_immediate_caller_source_context(&context) {
                self.caller_context_unavailable_bodies
                    .push(body.content_span());
            }
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
                    self.caller_context_unavailable_bodies.push(body);
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
    fn census_command_bodies(
        &mut self,
        source: &str,
        command: &SegmentedCommand,
        script: &SourceScriptContext<'_>,
        queue: &mut VecDeque<PendingSourceRegion>,
    ) {
        use tcl_compiler::registry_invocation::source_structure::{
            OriginalSourcePackageReferenceSelection as Selection, source_registry_words,
        };
        let analysis = script.analysis;
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        let Some(words) = source_registry_words(source, analysis, command) else {
            if original_positioned_absent_head(source, analysis, command) {
                // A genuine positioned nonpackage leaf is sufficient only for
                // this source package census, never generic syntax closure.
                self.package_census
                    .as_mut()
                    .unwrap()
                    .syntax_unavailable
                    .push(command.span);
            } else {
                self.package_census
                    .as_mut()
                    .unwrap()
                    .unavailable
                    .push(command.span);
            }
            return;
        };
        match words.package_reference_selection(&context) {
            Selection::Reference(reference) => self
                .package_census
                .as_mut()
                .unwrap()
                .references
                .push(reference),
            Selection::Unavailable => self
                .package_census
                .as_mut()
                .unwrap()
                .package_unavailable
                .push(command.span),
            Selection::NotReference => {}
        }
        match words.source_script_bodies_for_syntax_coverage(&context) {
            Some(bodies) => {
                for body in bodies {
                    queue.push_back((
                        body.content_span(),
                        script.depth + 1,
                        body.definition_parent_for(&context, script.definition_parent),
                    ));
                }
            }
            None => self
                .package_census
                .as_mut()
                .unwrap()
                .unavailable
                .push(command.span),
        }
        if words
            .roles()
            .is_some_and(|roles| roles.iter().any(|(_, role)| *role == ArgRole::Expr))
        {
            match words.source_expression_script_bodies(input) {
                Some(bodies) => {
                    for body in bodies {
                        queue.push_back((
                            body.content_span(),
                            script.depth + 1,
                            script.definition_parent.cloned(),
                        ));
                    }
                }
                None => self
                    .package_census
                    .as_mut()
                    .unwrap()
                    .unavailable
                    .push(command.span),
            }
        }
    }
}

/// A genuine positioned source absence can exclude this exact static head
/// from selected Registry package syntax. This grants no unknown-handler,
/// runtime package, command execution or Native absence proof.
fn original_positioned_absent_head(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> bool {
    let Some(head) = command.argv.first() else {
        return false;
    };
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    if !realm.matches_resolved_analysis_input(input) {
        return false;
    }
    let image = SourceImage::document(source);
    let Some((original, owner)) =
        realm.original_written_word_at_span_in_source(&image, head.span, config)
    else {
        return false;
    };
    owner == head.span.start()
        && !original.group().expand
        && tcl_syntax::word_rules::original_static_word_unicode_value(&original).is_some()
        && realm.diagnostic_slot_presence_at(owner)
            == tcl_compiler::command_binding::SourceCommandSlotPresence::Absent
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
        bodies.extend(logical_declared_bodies(analysis, image)?);
    }
    Some(bodies)
}

fn logical_declared_bodies(
    analysis: &AnalysisResult,
    image: &SourceImage,
) -> Option<Vec<BodyGeometry>> {
    analysis.allows_lexical_declaration_advice().then_some(())?;
    let mut bodies = Vec::new();
    let input = analysis.resolved_input.as_ref()?;
    for procedure in analysis.all_procs.values() {
        let Some(word) = analysis
            .logical_source_procedure_declaration(procedure)
            .and_then(|declaration| declaration.original_body_word(image, input))
        else {
            continue;
        };
        if let Some(body) = word_body(word) {
            bodies.push(body);
        }
    }
    for class in analysis.all_classes.values() {
        let Some(word) = analysis
            .logical_source_class_declaration(class)
            .and_then(|declaration| declaration.original_body_word(image, input))
        else {
            continue;
        };
        if let Some(body) = word_body(word) {
            bodies.push(body);
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

    #[test]
    fn commands_only_capture_keeps_current_original_regions_without_token_inventory() {
        let source = "proc p {} {greet arg}; set value [greet arg]; set data {inert arg}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let commands = SourceSyntaxStructure::capture_commands(source, &analysis).unwrap();
        let full = SourceSyntaxStructure::capture(source, &analysis).unwrap();
        assert_eq!(commands, full.commands());
        let greet = commands
            .iter()
            .filter(|command| command.texts.first().is_some_and(|head| head == "greet"))
            .count();
        assert_eq!(greet, 2);
        assert!(
            !commands
                .iter()
                .any(|command| command.texts.first().is_some_and(|head| head == "inert"))
        );
        assert!(
            SourceSyntaxStructure::capture_commands(
                &source.replace("greet arg", "other arg"),
                &analysis,
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(SourceSyntaxStructure::capture_commands(source, &missing).is_none());
        let mut mismatched = analysis;
        let config = mismatched.body_lexer_config.as_mut().unwrap();
        config.strict_quoting = !config.strict_quoting;
        assert!(SourceSyntaxStructure::capture_commands(source, &mismatched).is_none());
    }

    #[test]
    fn logical_procedure_structure_uses_current_declared_body_word() {
        let source = "proc helper {value} {puts \"literal \\} ${value}\"}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let original = declared_bodies(source, &analysis, &image, config).unwrap();
        assert!(original.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        ));
        let bodies = logical_declared_bodies(&analysis, &image).unwrap();
        assert!(bodies.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        ));
        let mut changed = analysis.clone();
        changed
            .all_procs
            .get_mut("::helper")
            .unwrap()
            .params
            .clear();
        let bodies = logical_declared_bodies(&changed, &image).unwrap();
        assert!(bodies.is_empty());
        let declaration = changed.original_procedure_declarations().next().unwrap();
        assert_eq!(declaration.metadata().params.len(), 1);
        assert!(
            crate::original_declaration::OriginalDeclarationIdentity::for_procedure(
                "",
                source,
                &changed,
                declaration
            )
            .is_some()
        );
        let original = declared_bodies(source, &changed, &image, config).unwrap();
        assert!(original.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        ));
        assert!(
            declared_bodies(source, &analysis, &SourceImage::document("changed"), config)
                .is_none_or(|bodies| bodies.is_empty())
        );
        assert!(
            logical_declared_bodies(&analysis, &SourceImage::document("changed"))
                .is_none_or(|bodies| bodies.is_empty())
        );
    }

    #[test]
    fn original_method_sites_keep_complete_words_nested_commands_and_current_input() {
        for (source, expected) in [
            ("$object method", ("object", "method", true)),
            ("\"$object\" method", ("object", "method", true)),
            ("${$literal} {with space}", ("$literal", "with space", true)),
            ("$array($index) method", ("array($index)", "method", true)),
            ("puts [$object method]", ("object", "method", true)),
            (
                "{object name} \"with space\"",
                ("object name", "with space", false),
            ),
            ("[self object] method", ("[self object]", "method", false)),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
            let cursor = u32::try_from(source.rfind(expected.1).unwrap() + 1).unwrap();
            assert_eq!(
                original_method_site_at(source, &analysis, cursor),
                Some((expected.0.to_owned(), expected.1.to_owned(), expected.2)),
                "{source}"
            );
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(original_method_site_at(source, &missing, cursor).is_none());
            assert!(original_method_site_at("changed", &analysis, cursor).is_none());
        }
        for (source, receiver, selector) in
            [("{} {}", "", ""), ("\"\" \"\"", "", ""), ("${} {}", "", "")]
        {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
            let cursor = u32::try_from(source.len() - 1).unwrap();
            assert_eq!(
                original_method_site_at(source, &analysis, cursor),
                Some((
                    receiver.to_owned(),
                    selector.to_owned(),
                    source.starts_with('$')
                )),
                "{source}"
            );
        }
        let source = "puts seed; $object longMethod";
        for style in [
            tcl_dialect::BracedVarStyle::FirstClose,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ] {
            let analysis = cursor_analysis(source, style);
            let cursor = u32::try_from(source.find("longMethod").unwrap() + 1).unwrap();
            assert_eq!(
                original_method_site_at(source, &analysis, cursor),
                Some(("object".to_owned(), "longMethod".to_owned(), true))
            );
            let mut stale = analysis.clone();
            stale.body_lexer_config.as_mut().unwrap().braced_var =
                if style == tcl_dialect::BracedVarStyle::FirstClose {
                    tcl_dialect::BracedVarStyle::Tcl9Nesting
                } else {
                    tcl_dialect::BracedVarStyle::FirstClose
                };
            assert!(original_method_site_at(source, &stale, cursor).is_none());
        }
        let source = "puts {$object method}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.find("method").unwrap()).unwrap();
        assert_eq!(
            original_method_site_at(source, &analysis, cursor),
            Some(("puts".to_owned(), "$object method".to_owned(), false))
        );
        assert!(crate::definition::definition(source, 0, cursor, &analysis).is_empty());
        for (source, needle) in [
            ("# $object method", "method"),
            ("expr {$object < $other}", "<"),
            ("$object $computed", "$computed"),
            ("{*}$object method", "method"),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
            let cursor = u32::try_from(source.find(needle).unwrap()).unwrap();
            assert!(
                original_method_site_at(source, &analysis, cursor).is_none(),
                "{source}"
            );
        }
    }

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
                let original = &source[reference.whole_span.as_range()];
                assert_eq!(
                    tcl_syntax::naming::element_var_name_for_style(original, style),
                    element,
                    "{needle}"
                );
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
