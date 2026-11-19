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

//! Current source/configuration eligibility and independent minifier purposes.

use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{LexerConfig, LineIndex, SourceImage, Span, Utf16Col};
use tcl_registry::CommandRegistry;

use super::{MinifyResult, SymbolMap};

/// Independently requested transformation tier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MinifyTier {
    /// Compact source separators while retaining original values and names.
    #[default]
    Default,
    /// Additionally request variable and procedure alpha renaming.
    Compact,
    /// Additionally request semantic rewrites, aliases and keyword shortening.
    Aggressive,
}

/// Requested passes; these flags supply no source or execution authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinifyOptions {
    /// Requested transformation tier.
    pub tier: MinifyTier,
    /// Authoring assertion of isolation, not a Native observer/absence receipt.
    pub isolated: bool,
    /// Request selected keyword shortening in the aggressive tier.
    pub abbreviations: bool,
}

impl Default for MinifyOptions {
    fn default() -> Self {
        Self {
            tier: MinifyTier::Default,
            isolated: false,
            abbreviations: true,
        }
    }
}

/// A requested pass cannot obtain its separate source or semantic contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinifyRefusal {
    /// Actual configuration, profile or command store was not retained.
    MissingAnalysisContext,
    /// The complete current source image does not match the retained analysis.
    StaleSource,
    /// Whole original lexical words could not be captured.
    OriginalSyntaxUnavailable,
    /// Authentic malformed syntax would change if only its prefix were emitted.
    IncompleteOriginalSyntax,
    /// Emission failed to preserve the exact original command/word sequence.
    SourceCorrespondenceChanged,
    /// Compatibility passes cannot retain this custom full configuration.
    UnsupportedLexicalConfiguration,
    /// Compatibility rewrites cannot retain the actual availability context.
    UnsupportedLexicalContext,
    /// Exact variable/procedure identity, coverage and observer closure is absent.
    MissingAlphaRenameContract,
    /// Scalar equivalence exists, but complete editable source coverage is absent.
    AlphaRenameGeometryUnavailable,
    /// Alias assignments lack independent store, insertion and frame permission.
    MissingAliasInsertionContract,
    /// Semantic/value rewrites lack their independent original transfer contract.
    MissingSemanticRewriteContract,
    /// Keywords lack an independently selected original point/handler protocol.
    MissingKeywordSelectionContract,
}

impl MinifyRefusal {
    /// Stable query-only code for CLI, editor and tool responses.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingAnalysisContext => "missing-analysis-context",
            Self::StaleSource => "stale-source",
            Self::OriginalSyntaxUnavailable => "original-syntax-unavailable",
            Self::IncompleteOriginalSyntax => "incomplete-original-syntax",
            Self::SourceCorrespondenceChanged => "source-correspondence-changed",
            Self::UnsupportedLexicalConfiguration => "unsupported-lexical-configuration",
            Self::UnsupportedLexicalContext => "unsupported-lexical-context",
            Self::MissingAlphaRenameContract => "missing-alpha-rename-contract",
            Self::AlphaRenameGeometryUnavailable => "alpha-rename-geometry-unavailable",
            Self::MissingAliasInsertionContract => "missing-alias-insertion-contract",
            Self::MissingSemanticRewriteContract => "missing-semantic-rewrite-contract",
            Self::MissingKeywordSelectionContract => "missing-keyword-selection-contract",
        }
    }

    /// Precise absent obligation, separate from any successfully compacted syntax.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::MissingAnalysisContext => {
                "The actual source configuration and Registry are unavailable"
            }
            Self::StaleSource => "The complete source changed after analysis",
            Self::OriginalSyntaxUnavailable => "Complete original executable words are unavailable",
            Self::IncompleteOriginalSyntax => "Malformed source is retained unchanged",
            Self::SourceCorrespondenceChanged => {
                "Compaction cannot preserve the exact original word sequence"
            }
            Self::UnsupportedLexicalConfiguration => {
                "Authoring passes do not support this retained full configuration"
            }
            Self::UnsupportedLexicalContext => {
                "Authoring rewrites do not support this retained availability context"
            }
            Self::MissingAlphaRenameContract => {
                "Name compaction requires exact source coverage and independent observer closure"
            }
            Self::AlphaRenameGeometryUnavailable => {
                "Scalar equivalence does not have complete editable original formal/read geometry"
            }
            Self::MissingAliasInsertionContract => {
                "Alias insertion requires an independent store and insertion contract in the actual frame"
            }
            Self::MissingSemanticRewriteContract => {
                "Semantic rewrites require an independent original value/effect transfer contract"
            }
            Self::MissingKeywordSelectionContract => {
                "Keyword shortening requires the selected original point and handler protocol"
            }
        }
    }
}

/// Eligibility of each name/effect-changing purpose is independent of syntax.
/// Scalar body equivalence is supplied independently from its edit geometry.
/// Isolation does not grant public procedure/global renaming or insertion.
/// Review N15 (naming-migration-review-20261022.md): the lexical
/// compatibility naming passes (`compact_names`, alias insertion) allocate
/// names without the shared transactional rename planner — occupied command
/// slots are not reserved and per-occurrence qualification is not preserved,
/// so `proc a …; proc longName …` can collide and `[::longName]` is left
/// behind. Until that planner exists the lexical path refuses those tiers
/// exactly as the native path does and runs syntax compaction only.
const LEXICAL_NAMING_PASSES: bool = false;

fn naming_refusals(options: MinifyOptions, scalar_alpha: bool) -> Vec<MinifyRefusal> {
    let mut refusals = Vec::new();
    if options.tier != MinifyTier::Default && (!scalar_alpha || options.isolated) {
        refusals.push(MinifyRefusal::MissingAlphaRenameContract);
    }
    if options.tier == MinifyTier::Aggressive {
        refusals.push(MinifyRefusal::MissingAliasInsertionContract);
        refusals.push(MinifyRefusal::MissingSemanticRewriteContract);
        if options.abbreviations {
            refusals.push(MinifyRefusal::MissingKeywordSelectionContract);
        }
    }
    refusals
}

/// Minify using the actual complete source image, full configuration and command
/// generation retained by `analysis`. Native syntax compaction preserves whole
/// word structure and independently selected script/expression token gaps.
/// This is source correspondence, not runtime reflection equivalence. Missing
/// naming, observer, store, movement or insertion contracts report refusals;
/// their passes never fall through to reporting String maps. Explicit lexical
/// advice retains the separate compatibility transformations.
#[must_use]
pub fn minify_with_analysis(
    source: &str,
    analysis: &AnalysisResult,
    options: MinifyOptions,
) -> MinifyResult {
    let mut result = MinifyResult {
        source: source.to_owned(),
        original_length: source.len(),
        ..MinifyResult::default()
    };
    if analysis.analysis_context_unavailable.is_some() {
        result.refusals.push(MinifyRefusal::MissingAnalysisContext);
        return result;
    }
    let (Some(config), Some(profile), Some(registry)) = (
        analysis.body_lexer_config,
        analysis.resolved_profile(),
        analysis.resolved_registry(),
    ) else {
        result.refusals.push(MinifyRefusal::MissingAnalysisContext);
        return result;
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config) {
        result.refusals.push(MinifyRefusal::StaleSource);
        return result;
    }
    if lexical_minify_route(source, analysis) {
        // Implementation contract: naming.minifier.lexical-authoring-passes
        // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
        // naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let Some(input) = analysis.resolved_input.as_ref() else {
            result.refusals.push(MinifyRefusal::MissingAnalysisContext);
            return result;
        };
        if config != input.lexer_config() || config != LexerConfig::for_profile(Some(profile)) {
            result
                .refusals
                .push(MinifyRefusal::UnsupportedLexicalConfiguration);
            return result;
        }
        // The legacy semantic and insertion passes accept one profile grammar.
        // Retain the actual full generation; refuse axes those passes cannot
        // transport instead of replacing them with the profile's defaults.
        let generation = crate::context_for_dialect_profile(profile);
        let expected = generation.with_command_store(registry.snapshot().shared_registry());
        let actual = input.context_registry();
        if input.analyser_profile().cache_key() != input.unit_profile().cache_key()
            || actual.context() != expected.context()
            || actual.commands().snapshot().semantic_key() != registry.snapshot().semantic_key()
        {
            result
                .refusals
                .push(MinifyRefusal::UnsupportedLexicalContext);
            return result;
        }
        let transformed = match options.tier {
            MinifyTier::Compact if LEXICAL_NAMING_PASSES => {
                super::lexical_compact(source, input, options.isolated).map(|(output, symbols)| {
                    result.source = output;
                    result.symbol_map = symbols;
                    result
                })
            }
            MinifyTier::Aggressive if LEXICAL_NAMING_PASSES => {
                super::lexical_aggressive(source, input, options.isolated, options.abbreviations)
            }
            tier => {
                if tier != MinifyTier::Default {
                    result.refusals.extend(naming_refusals(options, false));
                }
                super::lexical_default(source, analysis).map(|output| {
                    result.source = output;
                    result
                })
            }
        };
        return transformed.unwrap_or_else(|refusal| MinifyResult {
            source: source.to_owned(),
            original_length: source.len(),
            refusals: vec![refusal],
            ..MinifyResult::default()
        });
    }
    let mut scalar_alpha = false;
    if options.tier != MinifyTier::Default {
        match scalar_alpha_compaction(source, analysis, registry) {
            Ok(plan) => {
                result.source = plan.source;
                result.symbol_map = plan.symbol_map;
                result.optimisations_applied = plan.renamed_symbols;
                scalar_alpha = true;
            }
            Err(MinifyRefusal::MissingAlphaRenameContract) => {}
            Err(refusal) => result.refusals.push(refusal),
        }
    }
    result
        .refusals
        .extend(naming_refusals(options, scalar_alpha));
    match syntax_compaction(&result.source, analysis) {
        Ok(compacted) => result.source = compacted,
        Err(refusal) => {
            source.clone_into(&mut result.source);
            result.symbol_map = SymbolMap::default();
            result.optimisations_applied = 0;
            result.refusals.push(refusal);
        }
    }
    result
}

struct ScalarAlphaPlan {
    source: String,
    symbol_map: SymbolMap,
    renamed_symbols: usize,
}

fn scalar_alpha_compaction(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Result<ScalarAlphaPlan, MinifyRefusal> {
    use tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSlot;

    let config = analysis
        .body_lexer_config
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    let image = SourceImage::document(source);
    let declarations = analysis
        .original_procedure_declarations()
        .collect::<Vec<_>>();
    let [declaration] = declarations.as_slice() else {
        return Err(MinifyRefusal::MissingAlphaRenameContract);
    };
    // Generated ASCII names are proposals. Only the independent lower receipt
    // selects a valid same-frame replacement and closes its observer effects.
    let short = tcl_syntax::naming::AuthoredCompactNameSequence::new(config)
        .next_avoiding_observed(|_| false, Some(1))
        .ok_or(MinifyRefusal::MissingAlphaRenameContract)?
        .into_name();
    let formal =
        analysis.original_fixed_scalar_body_alpha_rename(declaration, short.as_bytes(), registry);
    let local = formal
        .is_none()
        .then(|| {
            analysis.original_local_store_alpha_rename(declaration, short.as_bytes(), registry)
        })
        .flatten();
    let symbol = if let Some(receipt) = formal.as_ref() {
        if !receipt.matches_source(&image, config)
            || !receipt.matches_registry(registry)
            || receipt.declaration_site() != declaration.declaration_site()
            || receipt.new_native_tail() != short.as_bytes()
        {
            return Err(MinifyRefusal::MissingAlphaRenameContract);
        }
        receipt.selected_symbol()
    } else if let Some(receipt) = local.as_ref() {
        if !receipt.matches_source(&image, config)
            || !receipt.matches_registry(registry)
            || receipt.declaration_site() != declaration.declaration_site()
            || receipt.new_native_tail() != short.as_bytes()
        {
            return Err(MinifyRefusal::MissingAlphaRenameContract);
        }
        receipt.selected_symbol()
    } else {
        return Err(MinifyRefusal::MissingAlphaRenameContract);
    };
    let SignatureSourceVariableSlot::Local { simple, .. } = symbol.slot() else {
        return Err(MinifyRefusal::MissingAlphaRenameContract);
    };
    let mut plan = ScalarAlphaPlan {
        source: source.to_owned(),
        symbol_map: SymbolMap::default(),
        renamed_symbols: 0,
    };
    if simple.as_bytes().len() <= short.len() {
        return Ok(plan);
    }
    // Equivalence and source geometry remain separate requirements. This edit
    // owner authenticates the selected declaration and every whole-object read.
    let edits =
        crate::variable_symbol::original_variable_rename_edits(source, analysis, symbol, &short)
            .map_err(|_| MinifyRefusal::AlphaRenameGeometryUnavailable)?;
    if edits.is_empty() {
        return Err(MinifyRefusal::AlphaRenameGeometryUnavailable);
    }
    let index = LineIndex::new(source);
    let byte_edits = scalar_alpha_byte_edits(source, &index, edits)?;
    // A native value can have several source-text inverses. Report the actual
    // declaration's written label after the receipt and edit owner select its cell.
    let mut original_formals = analysis.original_variable_symbols.iter().filter(|occurrence| {
        occurrence.symbol() == symbol && occurrence.is_declaration()
            && (local.is_some() || occurrence.receiver()
                == tcl_compiler::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::FormalDeclaration)
    });
    let original_formal = original_formals
        .next()
        .ok_or(MinifyRefusal::AlphaRenameGeometryUnavailable)?;
    if original_formals.next().is_some() {
        return Err(MinifyRefusal::AlphaRenameGeometryUnavailable);
    }
    let reported = source
        .get(
            original_formal
                .rename_span()
                .ok_or(MinifyRefusal::AlphaRenameGeometryUnavailable)?
                .as_range(),
        )
        .ok_or(MinifyRefusal::AlphaRenameGeometryUnavailable)?
        .to_owned();
    plan.source = super::apply_edits(source, byte_edits);
    // These labels only report a proved transformation; they never select a
    // declaration, variable, frame, native key or editable source occurrence.
    plan.symbol_map
        .variables
        .entry(declaration.metadata().qualified_name.clone())
        .or_default()
        .insert(reported, short);
    plan.renamed_symbols = 1;
    Ok(plan)
}

fn scalar_alpha_byte_edits(
    source: &str,
    index: &LineIndex,
    edits: Vec<crate::rename::TextEdit>,
) -> Result<Vec<super::Edit>, MinifyRefusal> {
    let mut byte_edits = Vec::new();
    for edit in edits {
        let start = index.offset_at_utf16(
            edit.range.start_line,
            Utf16Col::new(edit.range.start_character),
            source,
        );
        let end = index.offset_at_utf16(
            edit.range.end_line,
            Utf16Col::new(edit.range.end_character),
            source,
        );
        let written = source
            .get(start as usize..end as usize)
            .ok_or(MinifyRefusal::AlphaRenameGeometryUnavailable)?;
        byte_edits.push((start as usize, written.len(), edit.new_text));
    }
    Ok(byte_edits)
}

/// Explicit driver inputs: profile and store are retained before the analyser
/// sees the presentation label. Custom/editor analyses bypass this constructor.
pub(super) fn analysis_for_profile(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> AnalysisResult {
    // Implementation contract: naming.minifier.lexical-authoring-passes
    // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
    let generation = crate::context_for_dialect_profile(profile);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        LexerConfig::for_profile(Some(profile)),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

/// A compatibility rewrite changes the source, retaining every actual input axis.
pub(super) fn analysis_with_input(source: &str, input: &ResolvedAnalysisInput) -> AnalysisResult {
    // naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name)
}

/// Gaps belong to authenticated script regions or a separately issued original
/// expression action proof, never to displayed argument text.
#[derive(Clone, PartialEq, Eq)]
struct SourceGapEdit {
    span: Span,
    replacement: String,
}

fn source_gap_output(
    source: &str,
    region: Span,
    edits: &[SourceGapEdit],
) -> Result<String, MinifyRefusal> {
    let mut output = String::new();
    let mut cursor = region.start();
    for edit in edits.iter().filter(|edit| {
        edit.span.start() >= region.start()
            && edit.span.end() <= region.end()
            && (edit.span.start() != edit.span.end()
                || (edit.span.start() > region.start() && edit.span.end() < region.end()))
    }) {
        if edit.span.start() < cursor {
            return Err(MinifyRefusal::SourceCorrespondenceChanged);
        }
        output.push_str(
            source
                .get(Span::new(cursor, edit.span.start()).as_range())
                .ok_or(MinifyRefusal::SourceCorrespondenceChanged)?,
        );
        output.push_str(&edit.replacement);
        cursor = edit.span.end();
    }
    output.push_str(
        source
            .get(Span::new(cursor, region.end()).as_range())
            .ok_or(MinifyRefusal::SourceCorrespondenceChanged)?,
    );
    Ok(output)
}

fn emitted_boundary(offset: u32, edits: &[SourceGapEdit]) -> Result<u32, MinifyRefusal> {
    let mut emitted = i64::from(offset);
    for edit in edits.iter().take_while(|edit| edit.span.end() <= offset) {
        emitted += i64::try_from(edit.replacement.len())
            .map_err(|_| MinifyRefusal::SourceCorrespondenceChanged)?
            - i64::from(edit.span.end() - edit.span.start());
    }
    u32::try_from(emitted).map_err(|_| MinifyRefusal::SourceCorrespondenceChanged)
}

fn boolean_inversion_edit(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    argument: usize,
    inner: &str,
    env: super::MinifyEnv<'_>,
) -> Result<Option<SourceGapEdit>, MinifyRefusal> {
    // Renderer proposals supply no authority. The original owner accepts only
    // its narrow boolean-complement law and exact lexical leaf provenance.
    let candidate = super::shrink_expr_ast(inner, env, 0);
    if candidate == inner {
        return Ok(None);
    }
    let Some(proof) = tcl_compiler::registry_invocation::source_structure::original_expression_boolean_comparison_inversion(
        source, analysis, command, argument, &candidate,
    ) else { return Ok(None); };
    if !proof.matches_source(&SourceImage::document(source), env.config)
        || !proof.matches_input(env.input)
    {
        return Err(MinifyRefusal::SourceCorrespondenceChanged);
    }
    Ok(Some(SourceGapEdit {
        span: proof.original_operand().span(),
        replacement: proof.replacement_source_word().to_owned(),
    }))
}

/// Retain a complete original static source word's configured Unicode value.
/// This syntax-only spelling edit supplies no lookup, frame or runtime grant.
fn static_quoted_source_edits(word: &tcl_lexer::NativeWord) -> Option<[SourceGapEdit; 2]> {
    if word.group().kind != tcl_lexer::WordKind::Quoted || word.group().expand {
        return None;
    }
    let content = word.content_span().ok()?;
    let proposal = word.image().try_text().ok()?.get(content.as_range())?;
    // Keep comment-shaped and brace-shaped spellings conservative in all slots.
    if proposal.starts_with(['#', '{']) {
        return None;
    }
    let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
    let value = std::str::from_utf8(&value).ok()?;
    if !tcl_syntax::word_rules::lexical_unicode_source_word_matches(proposal, value, word.config())
        || !tcl_syntax::word_rules::lexical_source_word_preserves_components(word, proposal)
    {
        return None;
    }
    // Separate delimiter edits keep any independently selected child region's
    // original interior boundaries mappable through the shared emitter.
    Some([
        SourceGapEdit {
            span: Span::new(word.span().start(), content.start()),
            replacement: String::new(),
        },
        SourceGapEdit {
            span: Span::new(content.end(), word.span().end()),
            replacement: String::new(),
        },
    ])
}

fn lexical_minify_route(source: &str, analysis: &AnalysisResult) -> bool {
    let lexical = analysis.allows_lexical_declaration_advice();
    trace_minify_source(
        "branch.lexical",
        source.len(),
        None,
        Some(usize::from(lexical)),
    );
    lexical
}

fn trace_minify_source(
    stage: &'static str,
    bytes: usize,
    offset: Option<u32>,
    count: Option<usize>,
) {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1"))
        && std::env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some()
    {
        eprintln!("MINIFY_SOURCE stage={stage} bytes={bytes} offset={offset:?} count={count:?}");
    }
}

fn source_body_expression_gap_proofs(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    inventory: &tcl_compiler::registry_invocation::source_structure::OriginalBodySyntaxInventory,
    env: super::MinifyEnv<'_>,
    native_arguments: &[usize],
) -> Result<
    Vec<tcl_compiler::registry_invocation::source_structure::OriginalExpressionSyntaxGapCompaction>,
    MinifyRefusal,
> {
    let mut proofs = Vec::new();
    let operands = inventory.expression_operands(source, analysis, command);
    trace_minify_source(
        "source.operands",
        source.len(),
        command.argv.first().map(|word| word.span.start()),
        operands.as_ref().map(Vec::len),
    );
    for (argument, original) in operands.unwrap_or_default() {
        if native_arguments.contains(&argument)
            || original.group().kind != tcl_lexer::WordKind::Braced
        {
            continue;
        }
        let Ok(content) = original.content_span() else {
            continue;
        };
        let Some(inner) = source.get(content.as_range()) else {
            continue;
        };
        // A source Expr role supplies only token-gap correspondence. It never
        // enters boolean inversion, Native rewriting or a completion domain.
        let candidate = super::strip_expr_whitespace_with_script_compaction(inner, env, 0, false);
        let proof =
            inventory.source_expression_gaps(source, analysis, command, argument, &candidate);
        trace_minify_source(
            "source.gap-proof",
            source.len(),
            command.argv.first().map(|word| word.span.start()),
            Some(usize::from(proof.is_some())),
        );
        let Some(proof) = proof else {
            continue;
        };
        if proof.original_operand() != &original
            || !proof.matches_source(&SourceImage::document(source), env.config)
            || !proof.matches_registry(env.registry)
        {
            return Err(MinifyRefusal::SourceCorrespondenceChanged);
        }
        proofs.push(proof);
    }
    Ok(proofs)
}

fn syntax_compaction(source: &str, analysis: &AnalysisResult) -> Result<String, MinifyRefusal> {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let config = analysis
        .body_lexer_config
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    // Alpha edits produce a new complete source. Analyse that source with the
    // same retained inputs before selecting any body geometry.
    let current;
    let analysis = if analysis.matches_original_source_image(&SourceImage::document(source), config)
    {
        analysis
    } else {
        let input = analysis
            .resolved_input
            .clone()
            .ok_or(MinifyRefusal::MissingAnalysisContext)?;
        let label = input.analyser_profile().name;
        current = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, label);
        &current
    };
    let structure = crate::source_structure::SourceStructure::capture_for_syntax_compaction(
        source, analysis, config,
    )
    .ok_or(MinifyRefusal::OriginalSyntaxUnavailable)?;
    trace_minify_source(
        "structure.inventory",
        source.len(),
        None,
        Some(usize::from(structure.compiler_body_syntax.is_some())),
    );
    for command in &structure.commands {
        trace_minify_source(
            "structure.command",
            source.len(),
            command.argv.first().map(|word| word.span.start()),
            Some(command.argv.len()),
        );
    }
    let image = SourceImage::document(source);
    let mut plans = Vec::new();
    let mut edits = Vec::new();
    for (region, _) in structure.scripts {
        let plan = tcl_lexer::native_script_words_in(image.clone(), region, config)
            .map_err(|_| MinifyRefusal::OriginalSyntaxUnavailable)?;
        if plan.fatal_tail.is_some() {
            return Err(MinifyRefusal::IncompleteOriginalSyntax);
        }
        append_script_gap_edits(region, &plan, &mut edits);
        plans.push((region, plan));
    }
    let input = analysis
        .resolved_input
        .as_ref()
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    let context = input.context_registry();
    let env = super::MinifyEnv {
        original_source: source,
        analysis,
        input,
        dialect: input.unit_profile(),
        config,
        registry: context.commands(),
        context: &context,
        identities: analysis
            .retained_command_realm()
            .ok_or(MinifyRefusal::MissingAnalysisContext)?,
    };
    let mut expressions = Vec::new();
    for command in &structure.commands {
        append_command_expression_gaps(
            command,
            structure.compiler_body_syntax.as_ref(),
            env,
            &image,
            &mut edits,
            &mut expressions,
        )?;
    }
    edits.retain(|edit| source.get(edit.span.as_range()) != Some(edit.replacement.as_str()));
    edits.sort_by_key(|edit| (edit.span.start(), edit.span.end()));
    edits.dedup();
    if edits
        .windows(2)
        .any(|pair| pair[1].span.start() < pair[0].span.end() || pair[0].span == pair[1].span)
    {
        return Err(MinifyRefusal::SourceCorrespondenceChanged);
    }
    let whole = Span::new(
        0,
        u32::try_from(source.len()).map_err(|_| MinifyRefusal::OriginalSyntaxUnavailable)?,
    );
    let output = source_gap_output(source, whole, &edits)?;
    let emitted_image = SourceImage::document(&output);
    // Compose expression gaps with independently selected descendant scripts.
    // Every whole emitted word is checked again under the original grammar.
    for proof in expressions {
        validate_expression_gap_output(source, &output, &edits, &proof)?;
    }
    // Every selected script is checked independently. Parent words may differ
    // only by selected descendant script gaps or the expression proofs above.
    for (region, plan) in plans {
        validate_emitted_script_gap_plan(env, region, &plan, &edits, &emitted_image)?;
    }
    Ok(output)
}

fn validate_expression_gap_output(
    source: &str,
    output: &str,
    edits: &[SourceGapEdit],
    proof: &tcl_compiler::registry_invocation::source_structure::OriginalExpressionSyntaxGapCompaction,
) -> Result<(), MinifyRefusal> {
    let original = proof.original_operand().span();
    let emitted = Span::new(
        emitted_boundary(original.start(), edits)?,
        emitted_boundary(original.end(), edits)?,
    );
    if output.get(emitted.as_range()) != Some(source_gap_output(source, original, edits)?.as_str())
    {
        return Err(MinifyRefusal::SourceCorrespondenceChanged);
    }

    Ok(())
}

fn validate_emitted_script_gap_plan(
    env: super::MinifyEnv<'_>,
    region: Span,
    plan: &tcl_lexer::NativeScriptWordsPlan,
    edits: &[SourceGapEdit],
    emitted_image: &SourceImage,
) -> Result<(), MinifyRefusal> {
    let source = env.original_source;
    let config = env.config;
    let emitted_region = Span::new(
        emitted_boundary(region.start(), edits)?,
        emitted_boundary(region.end(), edits)?,
    );
    let emitted = tcl_lexer::native_script_words_in(emitted_image.clone(), emitted_region, config)
        .map_err(|_| MinifyRefusal::SourceCorrespondenceChanged)?;
    if emitted.fatal_tail.is_some() || emitted.commands.len() != plan.commands.len() {
        return Err(MinifyRefusal::SourceCorrespondenceChanged);
    }
    for (next, old) in emitted.commands.iter().zip(&plan.commands) {
        if next.words.len() != old.words.len() {
            return Err(MinifyRefusal::SourceCorrespondenceChanged);
        }
        for (next, old) in next.words.iter().zip(&old.words) {
            if next.group().expand != old.group().expand
                || next.written_bytes() != source_gap_output(source, old.span(), edits)?.as_bytes()
            {
                return Err(MinifyRefusal::SourceCorrespondenceChanged);
            }
        }
    }

    Ok(())
}

fn append_script_gap_edits(
    region: Span,
    plan: &tcl_lexer::NativeScriptWordsPlan,
    edits: &mut Vec<SourceGapEdit>,
) {
    let mut cursor = region.start();
    for (ordinal, command) in plan.commands.iter().enumerate() {
        for (index, word) in command.words.iter().enumerate() {
            let replacement = if index != 0 {
                " "
            } else if ordinal != 0 {
                ";"
            } else {
                ""
            };
            edits.push(SourceGapEdit {
                span: Span::new(cursor, word.span().start()),
                replacement: replacement.to_owned(),
            });
            if let Some(delimiters) = static_quoted_source_edits(word) {
                edits.extend(delimiters);
            }
            cursor = word.span().end();
        }
    }
    edits.push(SourceGapEdit {
        span: Span::new(cursor, region.end()),
        replacement: String::new(),
    });
}

/// Edits dropping the parentheses precedence makes redundant in one braced
/// expression, kept only when the expression emitted with them and its
/// whitespace `gaps` parses to the same tree under the document's grammar.
fn redundant_paren_edits(
    source: &str,
    inner: Span,
    gaps: &[SourceGapEdit],
    env: super::MinifyEnv<'_>,
) -> Vec<SourceGapEdit> {
    let Some(text) = source.get(inner.as_range()) else {
        return Vec::new();
    };
    let Some(expression) = super::expressions::original_expression(text, env) else {
        return Vec::new();
    };
    let Some(parens) = expression.redundant_paren_edits() else {
        return Vec::new();
    };
    let parens: Vec<SourceGapEdit> = parens
        .into_iter()
        .map(|(offset, replacement)| SourceGapEdit {
            span: Span::new(inner.start() + offset, inner.start() + offset + 1),
            replacement: replacement.to_owned(),
        })
        .collect();
    let mut combined: Vec<SourceGapEdit> = gaps.iter().chain(&parens).cloned().collect();
    combined.sort_by_key(|edit| (edit.span.start(), edit.span.end()));
    match source_gap_output(source, inner, &combined) {
        Ok(emitted) if expression.same_parse_as(&emitted) => parens,
        _ => Vec::new(),
    }
}

fn append_command_expression_gaps(
    command: &tcl_compiler::segmenter::SegmentedCommand,
    inventory: Option<
        &tcl_compiler::registry_invocation::source_structure::OriginalBodySyntaxInventory,
    >,
    env: super::MinifyEnv<'_>,
    image: &SourceImage,
    edits: &mut Vec<SourceGapEdit>,
    expressions: &mut Vec<
        tcl_compiler::registry_invocation::source_structure::OriginalExpressionSyntaxGapCompaction,
    >,
) -> Result<(), MinifyRefusal> {
    let source = env.original_source;
    let analysis = env.analysis;
    let config = env.config;
    let selected = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, command,
    );
    let selected = if selected.as_ref().is_none_or(|words| {
        !matches!(
            words.source(),
            tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Selected
        ) || !words.operands_preserve_source_lookup()
    }) {
        inventory.and_then(|inventory| inventory.words(source, analysis, command))
    } else {
        selected
    };
    let mut native_arguments = Vec::new();
    if let Some(words) = selected {
        for &(argument, role) in words.roles().unwrap_or_default() {
            if role != tcl_registry::ArgRole::Expr {
                continue;
            }
            let Some(word) = words
                .original_argument_word(argument)
                .filter(|word| word.group().kind == tcl_lexer::WordKind::Braced)
            else {
                continue;
            };
            let Ok(inner_span) = word.content_span() else {
                continue;
            };
            let Some(inner) = source.get(inner_span.as_range()) else {
                continue;
            };
            if let Some(edit) =
                boolean_inversion_edit(source, analysis, command, argument, inner, env)?
            {
                native_arguments.push(argument);
                edits.push(edit);
                continue;
            }
            // Child scripts keep their independently authenticated script gaps.
            let compacted_candidate =
                super::strip_expr_whitespace_with_script_compaction(inner, env, 0, false);
            let Some(proof) = tcl_compiler::registry_invocation::source_structure::original_expression_syntax_gap_compaction(
            source, analysis, command, argument, &compacted_candidate,
        ).or_else(|| inventory.and_then(|inventory| {
            inventory.expression_gaps(source, analysis, command, argument, &compacted_candidate)
        })) else { continue; };
            if !proof.matches_source(image, config)
                || !proof.matches_registry(env.context.commands())
            {
                return Err(MinifyRefusal::SourceCorrespondenceChanged);
            }
            let gaps: Vec<SourceGapEdit> = proof
                .removed_gaps()
                .iter()
                .map(|&span| SourceGapEdit {
                    span,
                    replacement: String::new(),
                })
                .collect();
            edits.extend(redundant_paren_edits(source, inner_span, &gaps, env));
            edits.extend(gaps);
            native_arguments.push(argument);
            expressions.push(proof);
        }
    }
    if let Some(inventory) = inventory {
        for proof in source_body_expression_gap_proofs(
            source,
            analysis,
            command,
            inventory,
            env,
            &native_arguments,
        )? {
            edits.extend(proof.removed_gaps().iter().map(|&span| SourceGapEdit {
                span,
                replacement: String::new(),
            }));
            expressions.push(proof);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
