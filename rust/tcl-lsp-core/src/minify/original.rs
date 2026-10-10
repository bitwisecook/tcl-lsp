// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
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
/// generation retained by `analysis`. Native syntax compaction keeps each whole
/// original word unchanged and independently checks emitted lexical structure.
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
    if analysis.allows_lexical_declaration_advice() {
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
            MinifyTier::Default => super::lexical_default(source, analysis).map(|output| {
                result.source = output;
                result
            }),
            MinifyTier::Compact => {
                super::lexical_compact(source, input, options.isolated).map(|(output, symbols)| {
                    result.source = output;
                    result.symbol_map = symbols;
                    result
                })
            }
            MinifyTier::Aggressive => {
                super::lexical_aggressive(source, input, options.isolated, options.abbreviations)
            }
        };
        return transformed.unwrap_or_else(|refusal| MinifyResult {
            source: source.to_owned(),
            original_length: source.len(),
            refusals: vec![refusal],
            ..MinifyResult::default()
        });
    } else {
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
    }
    match syntax_compaction(&result.source, analysis) {
        Ok(compacted) => result.source = compacted,
        Err(refusal) => {
            result.source = source.to_owned();
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
    let short = super::NameGenerator::new().next_name();
    let receipt = analysis
        .original_scalar_body_alpha_rename(declaration, short.as_bytes(), registry)
        .ok_or(MinifyRefusal::MissingAlphaRenameContract)?;
    if !receipt.matches_source(&image, config)
        || !receipt.matches_registry(registry)
        || receipt.declaration_site() != declaration.declaration_site()
        || receipt.new_native_tail() != short.as_bytes()
    {
        return Err(MinifyRefusal::MissingAlphaRenameContract);
    }
    let symbol = receipt.selected_symbol();
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
    // owner authenticates the formal list child and every whole-object read.
    let edits =
        crate::variable_symbol::original_variable_rename_edits(source, analysis, symbol, &short)
            .map_err(|_| MinifyRefusal::AlphaRenameGeometryUnavailable)?;
    if edits.is_empty() {
        return Err(MinifyRefusal::AlphaRenameGeometryUnavailable);
    }
    let index = LineIndex::new(source);
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
    // A native value can have several source-text inverses. Report the actual
    // formal's written label after the receipt and edit owner select its cell.
    let mut original_formals = analysis.original_variable_symbols.iter().filter(|occurrence| {
        occurrence.symbol() == symbol
            && occurrence.is_declaration()
            && occurrence.receiver()
                == tcl_compiler::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::FormalDeclaration
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

/// Gaps belong to authenticated script regions, never to displayed argument text.
#[derive(Clone, Copy, PartialEq, Eq)]
struct SourceGapEdit {
    span: Span,
    replacement: &'static str,
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
        output.push_str(edit.replacement);
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
    let image = SourceImage::document(source);
    let mut plans = Vec::new();
    let mut edits = Vec::new();
    for (region, _) in structure.scripts {
        let plan = tcl_lexer::native_script_words_in(image.clone(), region, config)
            .map_err(|_| MinifyRefusal::OriginalSyntaxUnavailable)?;
        if plan.fatal_tail.is_some() {
            return Err(MinifyRefusal::IncompleteOriginalSyntax);
        }
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
                    replacement,
                });
                cursor = word.span().end();
            }
        }
        edits.push(SourceGapEdit {
            span: Span::new(cursor, region.end()),
            replacement: "",
        });
        plans.push((region, plan));
    }
    edits.retain(|edit| source.get(edit.span.as_range()) != Some(edit.replacement));
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
    // Every selected script is checked independently. Parent words may differ
    // only by the already selected descendant script's separator edits.
    for (region, plan) in plans {
        let emitted_region = Span::new(
            emitted_boundary(region.start(), &edits)?,
            emitted_boundary(region.end(), &edits)?,
        );
        let emitted =
            tcl_lexer::native_script_words_in(emitted_image.clone(), emitted_region, config)
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
                    || next.written_bytes()
                        != source_gap_output(source, old.span(), &edits)?.as_bytes()
                {
                    return Err(MinifyRefusal::SourceCorrespondenceChanged);
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minify::SymbolMap;

    fn analyse(source: &str, dialect: &str) -> AnalysisResult {
        Analyser::new().analyse(source, dialect)
    }

    #[test]
    fn original_syntax_compaction_keeps_opaque_words_nested_scripts_and_expansion() {
        // Implementation contract: naming.minifier.original-syntax-correspondence
        // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
        let source = "# comment\n  proc p\\uD800 {} { set x  1; return $x }\n  puts  ${x}  [list {# data} café]\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = analyse(source, dialect);
            let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
            assert!(
                result.refusals.is_empty(),
                "{dialect}: {:?}",
                result.refusals
            );
            assert_eq!(
                result.source,
                "proc p\\uD800 {} {set x 1;return $x};puts ${x} [list {# data} café]"
            );
            let again = minify_with_analysis(
                &result.source,
                &analyse(&result.source, dialect),
                MinifyOptions::default(),
            );
            assert_eq!(again.source, result.source);
        }
        let expansion = "  {*}{puts p\\uD800}  café\n";
        let result = minify_with_analysis(
            expansion,
            &analyse(expansion, "tcl8.6"),
            MinifyOptions::default(),
        );
        assert!(result.refusals.is_empty());
        assert_eq!(result.source, "{*}{puts p\\uD800} café");
    }

    #[test]
    fn original_recursive_minification_uses_only_current_authenticated_script_regions() {
        // Implementation contract: naming.minifier.original-syntax-correspondence
        // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let source = "proc p {argument} {\n    set literal {  # data  }\n    if {1} {\n        return [list  $argument  $literal]\n    }\n}\np VALUE\n";
            let mut analysis = analyse(source, dialect);
            analysis.all_procs.clear();
            analysis.command_invocations.clear();
            let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
            assert!(
                result.refusals.is_empty(),
                "{dialect}: {:?}",
                result.refusals
            );
            assert_eq!(
                result.source,
                "proc p {argument} {set literal {  # data  };if {1} {return [list $argument $literal]}};p VALUE",
                "{dialect}"
            );
            let shadowed = "proc if {args} {}\nif {1} {\n  set opaque  1\n}\n";
            let result = minify_with_analysis(
                shadowed,
                &analyse(shadowed, dialect),
                MinifyOptions::default(),
            );
            assert!(
                result.refusals.is_empty(),
                "{dialect}: {:?}",
                result.refusals
            );
            assert_eq!(
                result.source, "proc if {args} {};if {1} {\n  set opaque  1\n}",
                "{dialect}"
            );
            let quoted = r#"proc p {} "set local \u0031""#;
            let result =
                minify_with_analysis(quoted, &analyse(quoted, dialect), MinifyOptions::default());
            assert_eq!(
                result.source, quoted,
                "noncontiguous cooked body remains unchanged for {dialect}"
            );
        }
    }

    #[test]
    fn original_hosted_syntax_compaction_preserves_zero_gap_word_and_bracket_boundaries() {
        // Implementation contract: naming.minifier.original-syntax-correspondence
        // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
        let source = "  {list}{A}{}[list {C}{D}{}]\n";
        let analysis = analyse(source, "f5-irules");
        let config = analysis.body_lexer_config.unwrap();
        assert!(config.irules_brace_separator);
        assert!(analysis.has_original_vendor_source_names());
        assert!(!analysis.allows_lexical_declaration_advice());
        let original = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(original.commands[0].words.len(), 4);
        assert_eq!(
            original.commands[0].words[0].span().end(),
            original.commands[0].words[1].span().start()
        );
        assert_eq!(
            original.commands[0].words[2].span().end(),
            original.commands[0].words[3].span().start()
        );
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        assert!(result.refusals.is_empty(), "{:?}", result.refusals);
        assert_eq!(result.source, "{list} {A} {} [list {C} {D} {}]");
        let again = minify_with_analysis(
            &result.source,
            &analyse(&result.source, "f5-irules"),
            MinifyOptions::default(),
        );
        assert!(again.refusals.is_empty(), "{:?}", again.refusals);
        assert_eq!(again.source, result.source);
    }

    #[test]
    fn original_minifier_refuses_naming_insertion_and_keywords_without_promoting_isolation() {
        // Implementation contract: naming.minifier.original-naming-permission
        // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
        let source = "proc longname {longvar} {return $longvar}\nset existing 1\ntrace add variable existing write observe\nlongname 7\n";
        let mut analysis = analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.variables.clear();
        analysis.dialect = "f5-irules".to_owned();
        for isolated in [false, true] {
            let result = minify_with_analysis(
                source,
                &analysis,
                MinifyOptions {
                    tier: MinifyTier::Aggressive,
                    isolated,
                    abbreviations: true,
                },
            );
            assert_eq!(result.symbol_map, SymbolMap::default());
            assert_eq!(result.optimisations_applied, 0);
            assert_eq!(
                result.refusals,
                vec![
                    MinifyRefusal::MissingAlphaRenameContract,
                    MinifyRefusal::MissingAliasInsertionContract,
                    MinifyRefusal::MissingSemanticRewriteContract,
                    MinifyRefusal::MissingKeywordSelectionContract
                ]
            );
            assert!(
                result
                    .source
                    .starts_with("proc longname {longvar} {return $longvar};set existing 1;")
            );
            assert!(!result.source.starts_with("set a "));
        }
    }

    #[test]
    fn original_scalar_minification_requires_equivalence_and_complete_edit_geometry() {
        // Implementation contract: naming.minifier.original-scalar-body-alpha
        // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for name in ["longargument", "longé😀"] {
                let read = if name.is_ascii() {
                    format!("${name}")
                } else {
                    format!("${{{name}}}")
                };
                let source = format!("proc publicname {{{name}}} {{return {read}}}\n");
                let mut analysis = analyse(&source, dialect);
                analysis.all_procs.clear();
                analysis.global_scope.procs.clear();
                analysis.dialect = "f5-irules".to_owned();
                let declaration = analysis.original_procedure_declarations().next().unwrap();
                let receipt = analysis
                    .original_scalar_body_alpha_rename(
                        declaration,
                        b"a",
                        analysis.resolved_registry().unwrap(),
                    )
                    .unwrap_or_else(|| {
                        panic!("independent scalar equivalence missing: {dialect}: {name}")
                    });
                assert!(receipt.matches_source(
                    &SourceImage::document(&source),
                    analysis.body_lexer_config.unwrap()
                ));
                let options = MinifyOptions {
                    tier: MinifyTier::Compact,
                    ..MinifyOptions::default()
                };
                let result = minify_with_analysis(&source, &analysis, options);
                assert!(
                    result.refusals.is_empty(),
                    "{dialect}: {name}: {:?}",
                    result.refusals
                );
                let expected = if read.starts_with("${") {
                    "proc publicname {a} {return ${a}}"
                } else {
                    "proc publicname {a} {return $a}"
                };
                assert_eq!(result.source, expected);
                assert_eq!(result.optimisations_applied, 1);
                assert_eq!(result.symbol_map.variables["::publicname"][name], "a");
                assert!(result.symbol_map.procs.is_empty());
                let mut missing_geometry = analysis.clone();
                missing_geometry.original_variable_symbols.clear();
                let unavailable = minify_with_analysis(&source, &missing_geometry, options);
                assert!(
                    unavailable
                        .refusals
                        .contains(&MinifyRefusal::MissingAlphaRenameContract)
                );
                assert_eq!(unavailable.symbol_map, SymbolMap::default());
            }
        }
    }

    #[test]
    fn original_scalar_minification_keeps_isolation_and_other_pass_permissions_separate() {
        // Implementation contract: naming.minifier.original-scalar-body-alpha
        // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
        // Implementation contract: naming.minifier.original-naming-permission
        // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
        let source = "proc publicname {longargument} {return $longargument}\n";
        let analysis = analyse(source, "tcl8.6");
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Aggressive,
                isolated: true,
                abbreviations: true,
            },
        );
        assert_eq!(result.source, "proc publicname {a} {return $a}");
        assert_eq!(result.optimisations_applied, 1);
        assert!(result.symbol_map.procs.is_empty());
        assert_eq!(
            result.refusals,
            vec![
                MinifyRefusal::MissingAlphaRenameContract,
                MinifyRefusal::MissingAliasInsertionContract,
                MinifyRefusal::MissingSemanticRewriteContract,
                MinifyRefusal::MissingKeywordSelectionContract
            ]
        );
    }

    #[test]
    fn original_scalar_minification_declines_observers_stores_and_open_source() {
        // Implementation contract: naming.minifier.original-scalar-body-alpha
        // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
        for source in [
            "proc p {longargument} {return [info locals]}\n",
            "proc p {longargument} {set local $longargument; return $local}\n",
            "proc p {longargument} {upvar 0 longargument alias; return $alias}\n",
            "proc p {longargument} {trace add variable longargument read callback; return $longargument}\n",
            "proc p {longargument} {return x$longargument}\n",
            "proc p {{longargument DEFAULT}} {return $longargument}\n",
            "proc p {args} {return $args}\n",
            "proc p {longargument} {return $longargument}\ninfo args p\n",
            "proc p {longargument} {return $longargument}\nproc return {args} {}\n",
        ] {
            let analysis = analyse(source, "tcl8.6");
            let result = minify_with_analysis(
                source,
                &analysis,
                MinifyOptions {
                    tier: MinifyTier::Compact,
                    ..MinifyOptions::default()
                },
            );
            assert!(
                result
                    .refusals
                    .contains(&MinifyRefusal::MissingAlphaRenameContract),
                "{source}"
            );
            assert_eq!(result.symbol_map, SymbolMap::default(), "{source}");
            assert_eq!(result.optimisations_applied, 0, "{source}");
        }
    }

    #[test]
    fn original_minifier_declines_stale_images_full_configuration_changes_and_malformed_tail() {
        // Implementation contract: naming.minifier.original-syntax-correspondence
        // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
        let source = "set café 1\nputs $café\n";
        let mut analysis = analyse(source, "tcl8.6");
        let stale = format!("# changed\n{source}");
        let result = minify_with_analysis(&stale, &analysis, MinifyOptions::default());
        assert_eq!(result.source, stale);
        assert_eq!(result.refusals, vec![MinifyRefusal::StaleSource]);
        let mut config = analysis.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        analysis.body_lexer_config = Some(config);
        assert_eq!(
            minify_with_analysis(source, &analysis, MinifyOptions::default()).refusals,
            vec![MinifyRefusal::StaleSource]
        );
        let malformed = "puts first\nset x {\n";
        let result = minify_with_analysis(
            malformed,
            &analyse(malformed, "tcl8.6"),
            MinifyOptions::default(),
        );
        assert_eq!(result.source, malformed);
        assert!(
            result
                .refusals
                .contains(&MinifyRefusal::IncompleteOriginalSyntax)
        );
    }

    #[test]
    fn original_minifier_holds_custom_profile_and_registry_instead_of_resolving_its_label() {
        // Implementation contract: naming.minifier.original-syntax-correspondence
        // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
        let mut profile = crate::profile_for_dialect("tcl8.4").clone();
        profile.name = "f5-irules";
        let profile = profile.intern();
        let registry = CommandRegistry::build_default();
        let source = "puts { spaced  data }\n";
        let analysis = analysis_for_profile(source, profile, &registry);
        assert_eq!(
            analysis.resolved_profile().unwrap().cache_key(),
            profile.cache_key()
        );
        assert_eq!(
            analysis.resolved_registry().unwrap().snapshot(),
            registry.snapshot()
        );
        assert!(!analysis.allows_lexical_declaration_advice());
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                ..MinifyOptions::default()
            },
        );
        assert_eq!(result.source, "puts { spaced  data }");
        assert_eq!(
            result.refusals,
            vec![MinifyRefusal::MissingAlphaRenameContract]
        );
    }

    #[test]
    fn original_hosted_minifier_keeps_source_syntax_without_native_alpha_permission() {
        // Implementation contract: naming.minifier.original-naming-permission
        // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
        let source = "proc longname {longvar} {return $longvar}\n";
        let analysis = analyse(source, "f5-irules");
        assert!(analysis.has_original_vendor_source_names());
        assert!(!analysis.allows_lexical_declaration_advice());
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert_eq!(result.source, "proc longname {longvar} {return $longvar}");
        assert_eq!(
            result.refusals,
            vec![MinifyRefusal::MissingAlphaRenameContract]
        );
        assert_eq!(result.symbol_map, SymbolMap::default());
    }

    #[test]
    fn lexical_minifier_keeps_explicit_authoring_compaction_separate() {
        // Implementation contract: naming.minifier.lexical-authoring-passes
        // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
        let source = "proc longname {longvar} {return $longvar}\n";
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-lexical-authoring";
        let registry = CommandRegistry::build_default();
        let analysis = analysis_for_profile(source, profile.intern(), &registry);
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(!analysis.has_original_vendor_source_names());
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert!(result.refusals.is_empty(), "{:?}", result.refusals);
        assert!(!result.symbol_map.variables.is_empty());
        assert!(!result.source.contains("longvar"));
    }
    #[test]
    fn isolated_logical_formal_alpha_uses_general_bindings_and_preserves_defaults() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source = "proc general {longleft {longright longleft}} {list $longleft $longright $longleft; return $longright}\n";
        let analysis =
            analysis_for_profile(source, lexical_profile(), &CommandRegistry::build_default());
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert!(result.refusals.is_empty(), "{:?}", result.refusals);
        let mapping = result
            .symbol_map
            .variables
            .values()
            .next()
            .expect("general formal binding map");
        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains_key("longleft"));
        assert!(mapping.contains_key("longright"));
        assert!(!result.source.contains("$longleft"), "{}", result.source);
        assert!(!result.source.contains("$longright"), "{}", result.source);
        assert!(
            result.source.contains("longleft}"),
            "literal default is unchanged: {}",
            result.source
        );
    }

    #[test]
    fn logical_formal_alpha_requires_isolation_and_closed_binding_observability() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source =
            "proc general {longleft longright} {list $longleft $longright; return $longleft}";
        let analysis =
            analysis_for_profile(source, lexical_profile(), &CommandRegistry::build_default());
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: false,
                ..MinifyOptions::default()
            },
        );
        assert!(result.symbol_map.variables.is_empty());
        for body in [
            "mystery $longleft",
            "info locals; return $longleft",
            "global longleft; return $longleft",
            "trace add variable longleft read hook; return $longleft",
            "set longleft changed; return $longleft",
        ] {
            let source = format!("proc general {{longleft longright}} {{{body}}}");
            let analysis = analysis_for_profile(
                &source,
                lexical_profile(),
                &CommandRegistry::build_default(),
            );
            let result = minify_with_analysis(
                &source,
                &analysis,
                MinifyOptions {
                    tier: MinifyTier::Compact,
                    isolated: true,
                    ..MinifyOptions::default()
                },
            );
            assert!(
                result.symbol_map.variables.is_empty(),
                "{body}: {}",
                result.source
            );
            assert!(result.source.contains("longleft"));
        }
    }

    fn lexical_profile() -> &'static tcl_dialect::DialectProfile {
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-lexical-authoring";
        profile.intern()
    }

    #[test]
    fn lexical_roles_use_retained_context_instead_of_heterogeneous_last_spec() {
        // Implementation contract: naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        let source = "expr { 1 + 2 }\n";
        let analysis = analysis_for_profile(source, profile, &registry);
        assert!(analysis.allows_lexical_declaration_advice());
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let env = super::super::MinifyEnv {
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities: analysis.retained_command_realm().unwrap(),
        };
        let arguments = [tcl_registry::InvocationWord::Literal("1 + 2")];
        let (traits, roles, complete) =
            super::super::lexical_source_schema(env, "expr", &arguments, |schema| {
                let (roles, complete) = schema.authored_source_argument_roles();
                (schema.semantics.traits, roles, complete)
            })
            .unwrap();
        assert!(traits.contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS));
        assert!(complete);
        assert!(roles.contains(&(0, tcl_registry::ArgRole::Expr)));
        assert!(
            !registry
                .get("expr")
                .unwrap()
                .traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        );
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        assert!(result.refusals.is_empty(), "{:?}", result.refusals);
        assert_eq!(result.source, "expr {1+2}");
    }

    #[test]
    fn lexical_context_refusal_retains_source_for_every_tier() {
        // Implementation contract: naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        let context = Arc::new(
            crate::context_for_dialect("tcl8.6")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            LexerConfig::for_profile(Some(profile)),
        );
        let source = "proc longname {longvar} {return $longvar}\n";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_lexical_declaration_advice());
        for tier in [
            MinifyTier::Default,
            MinifyTier::Compact,
            MinifyTier::Aggressive,
        ] {
            let result = minify_with_analysis(
                source,
                &analysis,
                MinifyOptions {
                    tier,
                    isolated: true,
                    ..MinifyOptions::default()
                },
            );
            assert_eq!(result.source, source);
            assert_eq!(result.original_length, source.len());
            assert_eq!(result.symbol_map, SymbolMap::default());
            assert_eq!(result.optimisations_applied, 0);
            assert_eq!(
                result.refusals,
                vec![MinifyRefusal::UnsupportedLexicalContext]
            );
        }
    }

    #[test]
    fn lexical_custom_configuration_refuses_every_tier_without_fallback() {
        // Implementation contract: naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        let context = Arc::new(
            crate::context_for_dialect_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let mut config = LexerConfig::for_profile(Some(profile));
        config.strict_quoting = !config.strict_quoting;
        let input = ResolvedAnalysisInput::new(profile, profile, context, config);
        let source = "set longname  1\n";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(analysis.matches_original_source_image(&SourceImage::document(source), config));
        for tier in [
            MinifyTier::Default,
            MinifyTier::Compact,
            MinifyTier::Aggressive,
        ] {
            let result = minify_with_analysis(
                source,
                &analysis,
                MinifyOptions {
                    tier,
                    isolated: true,
                    ..MinifyOptions::default()
                },
            );
            assert_eq!(result.source, source);
            assert_eq!(result.symbol_map, SymbolMap::default());
            assert_eq!(result.optimisations_applied, 0);
            assert_eq!(
                result.refusals,
                vec![MinifyRefusal::UnsupportedLexicalConfiguration]
            );
        }
    }

    #[test]
    fn lexical_intermediate_source_retains_actual_input_generation() {
        // Implementation contract: naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        let source = "set longname  1\n";
        let analysis = analysis_for_profile(source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let rewritten = "set a  1\n";
        let current = analysis_with_input(rewritten, input);
        let retained = current.resolved_input.as_ref().unwrap();
        assert_eq!(retained, input);
        assert!(Arc::ptr_eq(
            &retained.context_registry(),
            &input.context_registry()
        ));
        assert!(current.matches_original_source_image(
            &SourceImage::document(rewritten),
            input.lexer_config(),
        ));
        assert!(
            !current.matches_original_source_image(
                &SourceImage::document(source),
                input.lexer_config(),
            )
        );
        assert_eq!(
            super::super::lexical_default(rewritten, &current).unwrap(),
            "set a 1"
        );
    }
    #[test]
    fn lexical_case_shape_retains_dynamic_subject_and_expansion_uncertainty() {
        // Implementation contract: naming.minifier.retained-lexical-context
        // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        let analysis =
            analysis_for_profile("switch -exact $value {a {puts A}}\n", profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let env = super::super::MinifyEnv {
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities: analysis.retained_command_realm().unwrap(),
        };
        let mut arguments = [
            tcl_registry::InvocationWord::Literal("-exact"),
            tcl_registry::InvocationWord::Dynamic,
            tcl_registry::InvocationWord::Literal("a {puts A}"),
        ];
        let selected = super::super::lexical_source_schema(env, "switch", &arguments, |schema| {
            schema.authored_source_case_invocation()
        })
        .flatten()
        .unwrap();
        assert_eq!(selected.1.clause_list_index, Some(2));
        arguments[1] = tcl_registry::InvocationWord::Expanded;
        assert!(
            super::super::lexical_source_schema(env, "switch", &arguments, |schema| schema
                .authored_source_case_invocation(),)
            .flatten()
            .is_none()
        );
    }

    #[test]
    fn lexical_source_headers_keep_runtime_and_metadata_separate() {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        let registry = CommandRegistry::build_default();
        let source = "if {1} {  puts  payload  }\n";
        let analysis = analysis_for_profile(source, lexical_profile(), &registry);
        let realm = analysis.retained_command_realm().unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        assert!(realm.invocation_at_source("if", 0).unknown);
        assert_eq!(
            realm
                .lexical_source_header_unpositioned(&image, input, "if")
                .as_deref(),
            Some("if"),
        );
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        assert!(result.refusals.is_empty());
        assert_eq!(result.source, "if {1} {puts payload}");
        // This readonly projection did not replace the retained execution fact.
        assert!(realm.invocation_at_source("if", 0).unknown);
    }

    #[test]
    fn lexical_source_headers_refuse_dynamic_argv_and_prior_mutations() {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        let registry = CommandRegistry::build_default();
        for source in [
            "if $condition {  puts  payload  }\n",
            "mystery; if {1} {  puts  payload  }\n",
            "proc if {c b} {return $b}; if {1} {  puts  payload  }\n",
            "rename if old; if {1} {  puts  payload  }\n",
        ] {
            let analysis = analysis_for_profile(source, lexical_profile(), &registry);
            let realm = analysis.retained_command_realm().unwrap();
            let input = analysis.resolved_input.as_ref().unwrap();
            assert!(realm.lexical_source_header_unpositioned(
                &SourceImage::document(source), input, "if",
            ).is_none(), "{source}");
        }
        let native = crate::profile_for_dialect("tcl8.6");
        let source = "if {1} {puts payload}\n";
        let analysis = analysis_for_profile(source, native, &registry);
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .lexical_source_header_unpositioned(
                    &SourceImage::document(source),
                    analysis.resolved_input.as_ref().unwrap(),
                    "if",
                )
                .is_none()
        );
    }

    #[test]
    fn lexical_source_header_checks_retained_input_source_and_configuration() {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        let registry = CommandRegistry::build_default();
        let source = "expr {1 + 2}\n";
        let analysis = analysis_for_profile(source, lexical_profile(), &registry);
        let realm = analysis.retained_command_realm().unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        assert!(
            realm
                .lexical_source_header_unpositioned(&image, &input.clone(), "expr")
                .is_some()
        );
        assert!(
            realm
                .lexical_source_header_unpositioned(
                    &SourceImage::document("expr {2 + 3}\n"),
                    input,
                    "expr",
                )
                .is_none()
        );
        let mut wrong_config = input.lexer_config();
        wrong_config.strict_quoting = !wrong_config.strict_quoting;
        let wrong = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            wrong_config,
        );
        assert!(
            realm
                .lexical_source_header_unpositioned(&image, &wrong, "expr")
                .is_none()
        );
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(
                crate::context_for_dialect_profile(input.analyser_profile())
                    .with_command_store(registry.snapshot().shared_registry()),
            ),
            input.lexer_config(),
        );
        assert!(
            realm
                .lexical_source_header_unpositioned(&image, &foreign, "expr")
                .is_none()
        );
    }

    #[test]
    fn lexical_complete_heads_preserve_computed_body_data() {
        // Implementation contract: naming.minifier.complete-lexical-head
        // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        for head in [
            "$if",
            "if${suffix}",
            "[list if]",
            "\"$if\"",
            "\"if${suffix}\"",
        ] {
            let source = format!("{head} {{1}} {{  puts  payload  }}\n");
            let analysis = analysis_for_profile(&source, profile, &registry);
            assert!(analysis.allows_lexical_declaration_advice());
            let result = minify_with_analysis(&source, &analysis, MinifyOptions::default());
            assert!(result.refusals.is_empty(), "{head}: {:?}", result.refusals);
            assert!(
                result.source.ends_with("{  puts  payload  }"),
                "{head}: {}",
                result.source
            );
        }
    }

    #[test]
    fn lexical_complete_static_heads_keep_selected_body_roles() {
        // Implementation contract: naming.minifier.complete-lexical-head
        // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
        let profile = lexical_profile();
        let registry = CommandRegistry::build_default();
        for head in ["if", "{if}", "\"if\"", r"i\x66"] {
            let source = format!("{head} {{1}} {{  puts  payload  }}\n");
            let analysis = analysis_for_profile(&source, profile, &registry);
            let result = minify_with_analysis(&source, &analysis, MinifyOptions::default());
            assert!(result.refusals.is_empty(), "{head}: {:?}", result.refusals);
            assert!(
                result.source.ends_with("{puts payload}"),
                "{head}: {}",
                result.source
            );
        }
    }

    #[test]
    fn lexical_head_value_checks_whole_image_config_and_tokens() {
        // Implementation contract: naming.minifier.complete-lexical-head
        // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
        let source = "if {1} {puts payload}\n";
        let config = LexerConfig::for_profile(Some(lexical_profile()));
        let plan = super::super::lexical_script_words(source, config).unwrap();
        let tokens = tcl_lexer::Lexer::with_config(source, config)
            .tokenise_all()
            .unwrap();
        let commands = super::super::parse_commands(source, &tokens, config);
        let head = &commands[0][0];
        let word = &plan.commands[0].words[0];
        let sm = tcl_lexer::SourceMap::new(source);
        assert_eq!(
            super::super::lexical_head_value(&sm, head, word, config).as_deref(),
            Some("if")
        );
        let foreign_source = "if {1} {puts another}\n";
        let foreign = super::super::lexical_script_words(foreign_source, config).unwrap();
        assert!(
            super::super::lexical_head_value(&sm, head, &foreign.commands[0].words[0], config)
                .is_none()
        );
        let mut foreign_config = config;
        foreign_config.strict_quoting = !config.strict_quoting;
        assert!(super::super::lexical_head_value(&sm, head, word, foreign_config).is_none());
        assert!(super::super::lexical_head_value(&sm, &commands[0][1], word, config).is_none());
    }

    #[test]
    fn lexical_head_value_keeps_expansion_and_opaque_units_unavailable() {
        // Implementation contract: naming.minifier.complete-lexical-head
        // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
        let mut config = LexerConfig::for_profile(Some(lexical_profile()));
        config.expand_syntax = true;
        for source in [
            "{*}{if} {1} {puts payload}\n",
            r"i\uD800 {1} {puts payload}",
            "é {1} {puts payload}\n",
        ] {
            let plan = super::super::lexical_script_words(source, config).unwrap();
            let tokens = tcl_lexer::Lexer::with_config(source, config)
                .tokenise_all()
                .unwrap();
            let commands = super::super::parse_commands(source, &tokens, config);
            let sm = tcl_lexer::SourceMap::new(source);
            assert!(
                super::super::lexical_head_value(
                    &sm,
                    &commands[0][0],
                    &plan.commands[0].words[0],
                    config
                )
                .is_none(),
                "{source}"
            );
        }
        let source = "{*} {1} {puts payload}\n";
        config.expand_syntax = false;
        let plan = super::super::lexical_script_words(source, config).unwrap();
        let tokens = tcl_lexer::Lexer::with_config(source, config)
            .tokenise_all()
            .unwrap();
        let commands = super::super::parse_commands(source, &tokens, config);
        let sm = tcl_lexer::SourceMap::new(source);
        assert_eq!(
            super::super::lexical_head_value(
                &sm,
                &commands[0][0],
                &plan.commands[0].words[0],
                config
            )
            .as_deref(),
            Some("*")
        );
    }

    fn with_lexical_metadata<T>(
        source: &str,
        project: impl FnOnce(super::super::MinifyEnv<'_>) -> T,
    ) -> T {
        let registry = CommandRegistry::build_default();
        let analysis = analysis_for_profile(source, lexical_profile(), &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        project(super::super::MinifyEnv {
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities: analysis.retained_command_realm().unwrap(),
        })
    }

    #[test]
    fn logical_keyword_metadata_edits_whole_selected_controls_only() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "string equal -nocase A a\n";
        let (output, count) = with_lexical_metadata(source, |env| {
            super::super::lexical_keywords::abbreviate(source, env)
        });
        assert!(count > 0, "{output}");
        assert!(output.contains("-n A a"), "{output}");
        for source in [
            "string length$tail value\n",
            "string [set selector length] value\n",
            "string equal -nocase $value other\n",
            "string equal -nocase[set suffix {}] A a\n",
            "string equal -nocase -nocase other\n",
            "string equal -- -nocase other\n",
            "string equal A -nocase\n",
            r"string equal -no\x63ase A a",
        ] {
            let (output, count) = with_lexical_metadata(source, |env| {
                super::super::lexical_keywords::abbreviate(source, env)
            });
            if source.ends_with(" -nocase other\n") || source.ends_with(" A -nocase\n") {
                assert!(
                    output.ends_with("-nocase other\n") || output.ends_with("A -nocase\n"),
                    "{source} -> {output}"
                );
            }
            if source.contains("length$") || source.contains("[set") || source.contains("$value") {
                assert_eq!((output.as_str(), count), (source, 0));
            }
        }
    }

    #[test]
    fn logical_keyword_walk_distinguishes_data_from_selected_script_regions() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let data = "list {string equal -nocase A a}\n";
        with_lexical_metadata(data, |env| {
            let commands = super::super::lexical_metadata::commands(data, env).unwrap();
            assert_eq!(commands.len(), 1);
            assert_eq!(
                super::super::lexical_keywords::abbreviate(data, env),
                (data.to_owned(), 0)
            );
        });
        let body = "if {1} {string equal -nocase A a}\n";
        with_lexical_metadata(body, |env| {
            let commands = super::super::lexical_metadata::commands(body, env).unwrap();
            assert_eq!(commands.len(), 2);
            assert!(commands[1].span.start() > 0);
        });
        let bracket = "list [string equal -nocase A a]\n";
        with_lexical_metadata(bracket, |env| {
            assert_eq!(
                super::super::lexical_metadata::commands(bracket, env)
                    .unwrap()
                    .len(),
                2
            );
        });
    }

    #[test]
    fn logical_positioned_metadata_retains_unknown_values_and_exact_owner() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "string equal -nocase $value other\n";
        with_lexical_metadata(source, |env| {
            let image = SourceImage::document(source);
            let plan = super::super::lexical_script_words(source, env.config).unwrap();
            let command = &plan.commands[0];
            assert_eq!(
                env.identities
                    .lexical_source_header_for_words(&image, env.input, &command.words)
                    .as_deref(),
                Some("string")
            );
            let dynamic =
                super::super::lexical_metadata::with_command_schema(env, command, |schema| {
                    matches!(
                        schema.words.arguments().get(2),
                        Some(tcl_registry::InvocationWord::Dynamic)
                    )
                });
            assert_eq!(dynamic, Some(true));
            assert!(
                env.identities
                    .lexical_source_header_for_words(
                        &SourceImage::document("string equal -nocase changed other\n"),
                        env.input,
                        &command.words
                    )
                    .is_none()
            );
            let mut config = env.input.lexer_config();
            config.strict_quoting = !config.strict_quoting;
            let changed = ResolvedAnalysisInput::new(
                env.input.analyser_profile(),
                env.input.unit_profile(),
                env.input.context_registry(),
                config,
            );
            assert!(
                env.identities
                    .lexical_source_header_for_words(&image, &changed, &command.words)
                    .is_none()
            );
        });
        let unknown = "mystery; string equal -nocase A a\n";
        with_lexical_metadata(unknown, |env| {
            let image = SourceImage::document(unknown);
            let plan = tcl_lexer::native_script_words_in(
                image,
                tcl_lexer::Span::new(0, u32::try_from(unknown.len()).unwrap()),
                env.config,
            )
            .unwrap();
            let command = &plan.commands[1];
            let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
                env.identities, env.input, command,
            ).expect("conditional source metadata remains available after an unknown operation");
            let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("missing positively selected Logical source purpose");
            };
            assert!(advice.obligations().contains(
                &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
            ));
            assert!(!advice.uncertain_operations().is_empty());
            assert!(
                super::super::lexical_metadata::with_command_schema(env, command, |_| ()).is_some()
            );
            assert!(
                super::super::lexical_metadata::with_editable_command_schema(env, command, |_| ())
                    .is_none()
            );
        });
        for source in [
            "mystery; string equal -nocase A a\n",
            "proc string {args} {return value}; string equal -nocase A a\n",
            "rename string old; string equal -nocase A a\n",
            "string[set suffix {}] equal -nocase A a\n",
        ] {
            with_lexical_metadata(source, |env| {
                assert_eq!(
                    super::super::lexical_keywords::abbreviate(source, env),
                    (source.to_owned(), 0)
                );
            });
        }
    }

    #[test]
    fn logical_barriers_use_complete_selected_form_and_unknown_coverage() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        for source in ["info locals\n", "info locals$tail\n", "mystery\n"] {
            let registry = CommandRegistry::build_default();
            let analysis = analysis_for_profile(source, lexical_profile(), &registry);
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let env = super::super::MinifyEnv {
                input,
                dialect: input.unit_profile(),
                config: input.lexer_config(),
                registry: context.commands(),
                context: &context,
                identities: analysis.retained_command_realm().unwrap(),
            };
            let commands = super::super::lexical_metadata::commands(source, env).unwrap();
            let barriers = super::super::find_rename_barriers(&analysis, env, &commands, true);
            assert!(!barriers.allows_scope("::"), "{source}");
            if source.contains('$') || source.contains("mystery") {
                assert!(barriers.procs);
            }
        }
        let source = "list {info locals}\n";
        with_lexical_metadata(source, |env| {
            let commands = super::super::lexical_metadata::commands(source, env).unwrap();
            assert_eq!(commands.len(), 1);
        });
    }

    #[test]
    fn logical_metadata_never_supplies_native_or_hosted_keyword_permission() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "string equal -nocase A a\n";
        for name in ["tcl8.6", "f5-irules"] {
            let registry = CommandRegistry::build_default();
            let analysis =
                analysis_for_profile(source, crate::profile_for_dialect(name), &registry);
            let input = analysis.resolved_input.as_ref().unwrap();
            let image = SourceImage::document(source);
            let plan = super::super::lexical_script_words(source, input.lexer_config()).unwrap();
            assert!(
                analysis
                    .retained_command_realm()
                    .unwrap()
                    .lexical_source_header_for_words(&image, input, &plan.commands[0].words)
                    .is_none()
            );
            let result = minify_with_analysis(
                source,
                &analysis,
                MinifyOptions {
                    tier: MinifyTier::Aggressive,
                    abbreviations: true,
                    ..MinifyOptions::default()
                },
            );
            assert!(result.source.contains("-nocase"));
            assert!(
                result
                    .refusals
                    .contains(&MinifyRefusal::MissingKeywordSelectionContract)
            );
        }
    }
}
