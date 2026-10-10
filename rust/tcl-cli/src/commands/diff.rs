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

//! Diff verb: compare two sources at the AST / IR / CFG layers.
//!
//! Compares two sources across three layers, byte-for-byte with the captured
//! golden output (modulo a residual CFG/SSA construction gap on complex scripts):
//!
//! - **AST** — segments each side (`tcl-compiler` segmenter) and serialises
//!   the command list to canonical JSON (`sort_keys`).
//! - **IR** — reproduces the `serialise_result(compiled)["ir"]` view via the
//!   `tcl-explorer` `serialise::serialise_ir`, so the diff calls it directly
//!   rather than carrying a second IR serialiser.
//! - **CFG** — `{preSsa, postSsa}` from the explorer's `serialise_result`
//!   `cfgPreSsa`/`cfgPostSsa` views.
//!
//! Every layer renders through the shared `value_to_json` →
//! `snapshot::Json::dumps_indent2` adapter and a
//! `difflib.unified_diff`-faithful diff (`tcl_cli_support::difflib`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, difflib, ensure_ascii,
    read_input_documents, registry_for_dialect, write_text_output,
};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::segmenter::{
    SegmentedCommand, UnclosedDelimiter, segment_commands_with_offset_and_config,
};
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::CommandRegistry;
use tcl_registry::snapshot::Json;

/// Unified-diff context lines. Exposes `--context` (default
/// 3); the Rust CLI surface does not, so the default is used.
const CONTEXT: usize = 3;

/// Expand / validate the `--show` layer list.
/// All three layers (`ast`, `ir`, `cfg`) are implemented, so `all` expands to
/// the full set. Duplicates are dropped, preserving order.
fn parse_layers(show: &[String]) -> anyhow::Result<Vec<&'static str>> {
    const KNOWN: [&str; 3] = ["ast", "ir", "cfg"];
    // All three layers are implemented (`cfg` rides on the
    // CFG/SSA serialisers).
    const ALL_IMPLEMENTED: [&str; 3] = ["ast", "ir", "cfg"];
    let mut selected: Vec<&'static str> = Vec::new();
    for raw in show {
        for token in raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let names: &[&str] = match token {
                "all" => &ALL_IMPLEMENTED,
                "ast" => &["ast"],
                "ir" => &["ir"],
                "cfg" => &["cfg"],
                other => anyhow::bail!("unknown diff layer: {other} (expected ast,ir,cfg,all)"),
            };
            for name in names {
                let canonical = KNOWN.iter().find(|n| *n == name).copied().expect("known");
                if !selected.contains(&canonical) {
                    selected.push(canonical);
                }
            }
        }
    }
    if selected.is_empty() {
        anyhow::bail!("no diff layers selected; pass --show ast,ir,cfg or --show all");
    }
    Ok(selected)
}

/// `range_dict` over a command span: inclusive→exclusive end (`+1`), which is
/// exactly the segmenter's exclusive `span.end()`. The whole-command range is
/// never a single braced word, so `widen_for_highlight` is a no-op here.
fn range_json(span: tcl_lexer::Span, line_index: &LineIndex, source: &str) -> Json {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    let mut m = BTreeMap::new();
    m.insert("startLine".to_owned(), Json::Int(i64::from(start.line)));
    m.insert(
        "startCol".to_owned(),
        Json::Int(i64::from(start.character.get())),
    );
    m.insert("startOffset".to_owned(), Json::Int(i64::from(span.start())));
    m.insert("endLine".to_owned(), Json::Int(i64::from(end.line)));
    m.insert(
        "endCol".to_owned(),
        Json::Int(i64::from(end.character.get())),
    );
    m.insert("endOffset".to_owned(), Json::Int(i64::from(span.end())));
    Json::Object(m)
}

/// `partial_delimiter.name.lower()` or `""`.
fn partial_delimiter_str(cmd: &SegmentedCommand) -> &'static str {
    match cmd.partial_delimiter {
        Some(UnclosedDelimiter::Brace) => "brace",
        Some(UnclosedDelimiter::Bracket) => "bracket",
        Some(UnclosedDelimiter::Quote) => "quote",
        None => "",
    }
}

/// Canonical selected source subcommand under the complete current input.
/// A spelling alone supplies no handler, runtime argv or dispatch receipt.
fn resolve_subcommand(
    source: &str,
    cmd: &SegmentedCommand,
    registry: &CommandRegistry,
    analysis: &AnalysisResult,
) -> Option<&'static str> {
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    if input.lexer_config() != config
        || !analysis
            .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        || !analysis
            .retained_command_realm()?
            .matches_resolved_analysis_input(input)
        || registry.snapshot().semantic_key()
            != input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
    {
        return None;
    }
    let words = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, cmd,
    )?;
    words.with_source_schema(input.borrowed_context_registry(), |schema| {
        schema
            .subcommand
            .resolved()
            .map(|subcommand| subcommand.canonical_name)
    })?
}

/// One original source and checked input shared by every requested layer.
/// AST-only comparisons keep the structure tier; IR/CFG share one unit.
struct DiffDocument<'a> {
    source: &'a str,
    analysis: AnalysisResult,
    explorer: Option<tcl_explorer::ExplorerResult>,
}

impl<'a> DiffDocument<'a> {
    fn capture(source: &'a str, input: &ResolvedAnalysisInput, build_unit: bool) -> Option<Self> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let analysis = Analyser::new()
            .structure_only()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let config = input.lexer_config();
        if analysis.resolved_input.as_ref() != Some(input)
            || analysis.body_lexer_config != Some(config)
            || !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
            || !analysis
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        let explorer = build_unit.then(|| {
            let registry = input.borrowed_context_registry().commands();
            let profile = input.unit_profile();
            let declared = tcl_compiler::analyser::utils::document_declared_surface(
                source,
                None,
                profile.name,
            );
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: Some(&declared),
                },
                None,
                input,
            )
            .with_interprocedural(registry, Some(profile))
            .with_retained_memory_ssa(registry)
            .with_retained_deep_semantic_analysis(registry);
            tcl_explorer::ExplorerResult {
                source: source.to_owned(),
                dialect: profile.name.to_owned(),
                unit,
            }
        });
        Some(Self {
            source,
            analysis,
            explorer,
        })
    }
}

/// The `ast` layer payload (`{"commands": [...]}`), as canonical
/// 2-space-indented JSON (keys sorted).
fn serialise_command_ast(
    source: &str,
    registry: &CommandRegistry,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    config: LexerConfig,
) -> String {
    let commands: Vec<Json> = segment_commands_with_offset_and_config(source, 0, config)
        .iter()
        .map(|cmd| {
            let mut m = BTreeMap::new();
            m.insert("name".to_owned(), Json::s(cmd.name()));
            m.insert(
                "subcommand".to_owned(),
                Json::s(resolve_subcommand(source, cmd, registry, analysis).unwrap_or_default()),
            );
            m.insert(
                "args".to_owned(),
                Json::Array(cmd.args().iter().map(|a| Json::s(a.clone())).collect()),
            );
            m.insert("isPartial".to_owned(), Json::Bool(cmd.is_partial));
            m.insert(
                "partialDelimiter".to_owned(),
                Json::s(partial_delimiter_str(cmd)),
            );
            m.insert(
                "precedingComment".to_owned(),
                Json::s(cmd.preceding_comment.clone().unwrap_or_default()),
            );
            m.insert(
                "expandWord".to_owned(),
                Json::Array(
                    cmd.expand_word
                        .as_ref()
                        .map(|v| v.iter().map(|&b| Json::Bool(b)).collect())
                        .unwrap_or_default(),
                ),
            );
            m.insert("range".to_owned(), range_json(cmd.span, line_index, source));
            Json::Object(m)
        })
        .collect();
    let mut root = BTreeMap::new();
    root.insert("commands".to_owned(), Json::Array(commands));
    Json::Object(root).dumps_indent2()
}

/// Compute one layer's diff: equal flag +
/// the raw `unified_diff` line list (each line keeps its terminator).
fn compute_layer_diff(
    layer: &str,
    left_text: &str,
    right_text: &str,
    left_name: &str,
    right_name: &str,
) -> (bool, Vec<String>) {
    if left_text == right_text {
        return (true, Vec::new());
    }
    let left_owned = format!("{left_text}\n");
    let right_owned = format!("{right_text}\n");
    let la = difflib::splitlines_keepends(&left_owned);
    let lb = difflib::splitlines_keepends(&right_owned);
    let from = format!("{layer}:{left_name}");
    let to = format!("{layer}:{right_name}");
    let lines = difflib::unified_diff(&la, &lb, &from, &to, CONTEXT);
    (false, lines)
}

/// Build one layer's canonical-JSON payload text for `src`.
fn layer_payload(
    layer: &str,
    document: &DiffDocument<'_>,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    config: LexerConfig,
) -> anyhow::Result<String> {
    match layer {
        "ast" => Ok(serialise_command_ast(
            document.source,
            registry,
            &document.analysis,
            line_index,
            config,
        )),
        "ir" => {
            let result = document
                .explorer
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("the diff compilation unit is unavailable"))?;
            let ir = tcl_explorer::serialise::serialise_ir(
                &result.unit.ir_module,
                line_index,
                document.source,
            );
            Ok(value_to_json(&ir).dumps_indent2())
        }
        "cfg" => {
            let result = document
                .explorer
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("the diff compilation unit is unavailable"))?;
            Ok(cfg_layer_payload(result, line_index))
        }
        other => anyhow::bail!("unknown diff layer: {other} (expected ast,ir,cfg)"),
    }
}

/// Both CFG views come from the same unit used by the IR layer, under its
/// complete retained input. Serialisation creates no new source generation.
fn cfg_layer_payload(result: &tcl_explorer::ExplorerResult, line_index: &LineIndex) -> String {
    let mut payload: BTreeMap<String, Json> = BTreeMap::new();
    for (key, view) in [
        (
            "preSsa",
            tcl_explorer::serialise::serialise_cfg_pre_ssa(result, line_index, &result.source),
        ),
        (
            "postSsa",
            tcl_explorer::serialise::serialise_cfg_post_ssa(result, line_index, &result.source),
        ),
    ] {
        payload.insert(key.to_owned(), value_to_json(&view));
    }
    Json::Object(payload).dumps_indent2()
}

/// Convert a `serde_json::Value` (the explorer serialisers' output) into the
/// snapshot [`Json`] so it renders through the shared `dumps_indent2`
/// (2-space-indented JSON, keys sorted) adapter the AST/IR layers use. The
/// CFG payload is integer-only (offsets / versions / counts), so a JSON number
/// is always an `i64`.
fn value_to_json(value: &serde_json::Value) -> Json {
    match value {
        serde_json::Value::Null => Json::Null,
        serde_json::Value::Bool(b) => Json::Bool(*b),
        // CFG payloads are integer-only; a non-`i64` number would be a bug, so
        // surface it as a string rather than silently truncating through `f64`.
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Json::Str(n.to_string()), Json::Int),
        serde_json::Value::String(s) => Json::Str(s.clone()),
        serde_json::Value::Array(items) => Json::Array(items.iter().map(value_to_json).collect()),
        serde_json::Value::Object(map) => Json::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect(),
        ),
    }
}

/// `tcl diff` — AST/IR/CFG structural diff.
// Each CLI diff option is threaded through individually; a params struct would
// only re-wrap the same flags this `tcl diff` entry point already receives.
#[allow(clippy::too_many_arguments)]
pub fn run_diff(
    left: Option<&Path>,
    right: Option<&Path>,
    left_source: Option<&str>,
    right_source: Option<&str>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    show: &[String],
    json_out: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let layers = parse_layers(show)?;

    // A side may be supplied as a positional path OR as inline source
    // (`--left-source` / `--right-source`) — the inline flags are usable on
    // their own, not only appended to a path.
    if left.is_none() && left_source.is_none() {
        anyhow::bail!("diff requires a left input (a path or --left-source)");
    }
    if right.is_none() && right_source.is_none() {
        anyhow::bail!("diff requires a right input (a path or --right-source)");
    }
    let left_name = left.map_or_else(
        || "<left-source>".to_owned(),
        |p| p.to_string_lossy().into_owned(),
    );
    let right_name = right.map_or_else(
        || "<right-source>".to_owned(),
        |p| p.to_string_lossy().into_owned(),
    );

    let left_paths: Vec<PathBuf> = left.map(Path::to_path_buf).into_iter().collect();
    let right_paths: Vec<PathBuf> = right.map(Path::to_path_buf).into_iter().collect();
    let left_inline: Vec<String> = left_source.map(|s| vec![s.to_owned()]).unwrap_or_default();
    let right_inline: Vec<String> = right_source.map(|s| vec![s.to_owned()]).unwrap_or_default();
    let left_docs = read_input_documents(&left_paths, &left_inline, true)?;
    let right_docs = read_input_documents(&right_paths, &right_inline, true)?;
    let left_src = combine_sources(&left_docs);
    let right_src = combine_sources(&right_docs);

    // One dialect for the whole comparison — both sides are rendered through
    // the same pipeline, so a per-side dialect would diff two different
    // pipelines rather than two sources. Detected from the left-hand (baseline)
    // input unless `--dialect` names one.
    let dialect = combined_effective_dialect(&left_docs, dialect);
    let registry = registry_for_dialect(dialect.name);
    let left_index = LineIndex::new(&left_src);
    let right_index = LineIndex::new(&right_src);
    // Both sides are the same whole-document kind (a combined `--left`/
    // `--right` source), so both lex under the one resolved grammar.
    let config = LexerConfig::for_file_grammar(dialect.grammar);
    let context = tcl_lsp_core::context_for_dialect_profile(dialect);
    let input = ResolvedAnalysisInput::new(
        dialect,
        dialect,
        std::sync::Arc::new(context.with_command_store(registry.clone())),
        config,
    );
    let build_unit = layers.iter().any(|layer| matches!(*layer, "ir" | "cfg"));
    let left_document = DiffDocument::capture(&left_src, &input, build_unit)
        .ok_or_else(|| anyhow::anyhow!("the left source analysis context is unavailable"))?;
    let right_document = DiffDocument::capture(&right_src, &input, build_unit)
        .ok_or_else(|| anyhow::anyhow!("the right source analysis context is unavailable"))?;

    let mut results: Vec<(String, bool, Vec<String>)> = Vec::new();
    for layer in &layers {
        let lp = layer_payload(layer, &left_document, &registry, &left_index, config)?;
        let rp = layer_payload(layer, &right_document, &registry, &right_index, config)?;
        let (equal, lines) = compute_layer_diff(layer, &lp, &rp, &left_name, &right_name);
        results.push(((*layer).to_owned(), equal, lines));
    }

    let target = OutputTarget::from_arg(output);
    write_diff_result(
        &target,
        json_out,
        dialect.name,
        &left_name,
        &right_name,
        &left_docs,
        &right_docs,
        &results,
    )
}

/// Emit the multi-layer diff result as JSON or unified-diff text, returning the
/// exit code (1 when any layer differs).
// Takes the already-parsed diff inputs and names individually; bundling them
// into a struct would add a type without reducing what the emitter needs.
#[allow(clippy::too_many_arguments)]
fn write_diff_result(
    target: &OutputTarget,
    json_out: bool,
    dialect: &str,
    left_name: &str,
    right_name: &str,
    left_docs: &[tcl_cli_support::InputDocument],
    right_docs: &[tcl_cli_support::InputDocument],
    results: &[(String, bool, Vec<String>)],
) -> anyhow::Result<u8> {
    let has_differences = results.iter().any(|(_, equal, _)| !equal);

    if json_out {
        let mut layer_obj = Map::new();
        for (layer, equal, lines) in results {
            let stripped: Vec<String> = lines
                .iter()
                .map(|l| l.trim_end_matches('\n').to_owned())
                .collect();
            layer_obj.insert(
                layer.clone(),
                json!({
                    "equal": equal,
                    "diff": stripped,
                    "diffLineCount": lines.len(),
                }),
            );
        }
        let payload = json!({
            "equal": !has_differences,
            "dialect": dialect,
            "layers": Value::Object(layer_obj),
            "leftInput": left_name,
            "rightInput": right_name,
            "leftDocuments": left_docs.iter().map(|d| d.label.clone()).collect::<Vec<_>>(),
            "rightDocuments": right_docs.iter().map(|d| d.label.clone()).collect::<Vec<_>>(),
        });
        write_text_output(
            target,
            &ensure_ascii(&serde_json::to_string_pretty(&payload)?),
        )?;
        return Ok(u8::from(has_differences));
    }

    let mut chunks: Vec<String> = Vec::new();
    for (layer, equal, lines) in results {
        if *equal {
            chunks.push(format!("{layer}: identical\n"));
            continue;
        }
        chunks.push(format!("=== {layer} diff ===\n"));
        let stripped: Vec<&str> = lines.iter().map(|l| l.trim_end_matches('\n')).collect();
        if stripped.is_empty() {
            chunks.push(
                "(differences detected, but no unified diff lines were produced)\n".to_owned(),
            );
        } else {
            for line in stripped {
                chunks.push(format!("{line}\n"));
            }
        }
    }
    let text = chunks.concat();
    write_text_output(target, text.trim_end_matches('\n'))?;
    Ok(u8::from(has_differences))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical_input(
        registry: std::sync::Arc<CommandRegistry>,
        availability: &str,
        config: LexerConfig,
    ) -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::resolve_environment(availability).default_context_registry();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context.with_command_store(registry)),
            config,
        )
    }

    fn ast_rows(
        source: &str,
        analysis: &AnalysisResult,
        registry: &CommandRegistry,
        config: LexerConfig,
    ) -> Value {
        serde_json::from_str(&serialise_command_ast(
            source,
            registry,
            analysis,
            &LineIndex::new(source),
            config,
        ))
        .unwrap()
    }

    #[test]
    fn original_diff_subcommands_use_current_source_targets_and_captured_prefixes() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let registry = registry_for_dialect("tcl9.0");
        let input = logical_input(registry.clone(), "tcl9.0", config);
        let source = "::string len abc; interp alias {} chars {} string length; chars $value; string $operation abc";
        let document = DiffDocument::capture(source, &input, false).unwrap();
        assert!(
            document.explorer.is_none(),
            "AST selection does not require a deep unit"
        );
        let rows = ast_rows(source, &document.analysis, &registry, config);
        assert_eq!(rows["commands"][0]["subcommand"], "length");
        assert_eq!(rows["commands"][2]["subcommand"], "length");
        assert_eq!(rows["commands"][3]["subcommand"], "");
        assert_eq!(rows["commands"][2]["name"], "chars");
        let shadow = "proc string args {}; string length abc";
        let document = DiffDocument::capture(shadow, &input, false).unwrap();
        assert_eq!(
            ast_rows(shadow, &document.analysis, &registry, config)["commands"][1]["subcommand"],
            ""
        );
    }

    #[test]
    fn original_diff_pack_subcommands_keep_availability_and_refuse_withdrawn_inputs() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let mut registry = CommandRegistry::build_default();
        let mut packed = registry.get("dict").unwrap().clone();
        packed.name = "packed";
        registry.insert(packed);
        let registry = std::sync::Arc::new(registry);
        let input = logical_input(registry.clone(), "tcl9.0", config);
        let source = "packed create key value";
        let mut current = DiffDocument::capture(source, &input, false).unwrap();
        assert_eq!(
            ast_rows(source, &current.analysis, &registry, config)["commands"][0]["subcommand"],
            "create"
        );
        let older_input = logical_input(registry.clone(), "tcl8.4", config);
        let older = DiffDocument::capture(source, &older_input, false).unwrap();
        assert_eq!(
            input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key(),
            older_input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
        );
        assert_eq!(
            ast_rows(source, &older.analysis, &registry, config)["commands"][0]["subcommand"],
            ""
        );
        let segment = segment_commands_with_offset_and_config(source, 0, config).remove(0);
        assert!(
            resolve_subcommand(
                "packed create other values",
                &segment,
                &registry,
                &current.analysis
            )
            .is_none()
        );
        let retained = current.analysis.resolved_input.take().unwrap();
        assert!(resolve_subcommand(source, &segment, &registry, &current.analysis).is_none());
        current.analysis.resolved_input = Some(retained.clone());
        current
            .analysis
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = !config.strict_quoting;
        assert!(resolve_subcommand(source, &segment, &registry, &current.analysis).is_none());
        current.analysis.body_lexer_config = Some(config);
        current.analysis.resolved_input = Some(older_input);
        assert!(resolve_subcommand(source, &segment, &registry, &current.analysis).is_none());
        current.analysis.resolved_input = Some(retained);
        let foreign = CommandRegistry::build_default();
        assert!(resolve_subcommand(source, &segment, &foreign, &current.analysis).is_none());
        current.analysis.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl9.0".to_owned(),
            overlay: u64::MAX,
        });
        assert!(resolve_subcommand(source, &segment, &registry, &current.analysis).is_none());
    }

    #[test]
    fn original_diff_ir_and_cfg_share_the_exact_checked_source_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut config = LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let registry = registry_for_dialect("tcl9.0");
        let input = logical_input(registry.clone(), "tcl9.0", config);
        let source = "proc p {} {set value 1; return $value}; p";
        let document = DiffDocument::capture(source, &input, true).unwrap();
        let explorer = document.explorer.as_ref().unwrap();
        assert!(
            explorer.unit.function("::p").is_some(),
            "genuine Logical source procedure missing"
        );
        assert_eq!(document.analysis.resolved_input.as_ref(), Some(&input));
        let metadata = tcl_compiler::registry_invocation::InvocationMetadataContext::for_module(
            &registry,
            &explorer.unit.ir_module,
        )
        .unwrap();
        assert_eq!(metadata.source_analysis_input(), Some(&input));
        let index = LineIndex::new(source);
        assert_eq!(
            layer_payload("ir", &document, &registry, &index, config).unwrap(),
            value_to_json(&tcl_explorer::serialise::serialise_ir(
                &explorer.unit.ir_module,
                &index,
                source
            ))
            .dumps_indent2()
        );
        assert_eq!(
            layer_payload("cfg", &document, &registry, &index, config).unwrap(),
            cfg_layer_payload(explorer, &index)
        );
    }

    #[test]
    fn inline_sources_are_usable_without_paths() {
        // `--left-source` / `--right-source` work on their own,
        // not only appended to a positional path.
        let show = vec!["ast".to_owned()];
        let r = run_diff(
            None,
            None,
            Some("set x 1"),
            Some("set x 2"),
            Some(tcl_cli_support::environment::profile_for_dialect("tcl8.6")),
            &show,
            true,
            None,
        );
        assert!(r.is_ok(), "inline-only diff must not bail: {r:?}");

        // A side with neither a path nor inline source still errors.
        let err = run_diff(
            None,
            None,
            None,
            Some("set x 2"),
            Some(tcl_cli_support::environment::profile_for_dialect("tcl8.6")),
            &show,
            true,
            None,
        );
        assert!(
            err.is_err(),
            "a side with neither path nor source must error",
        );
    }
}
