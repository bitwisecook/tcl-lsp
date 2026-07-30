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

//! Source-transformation verbs: `format`, `minify`, `unminify-error`.
//!
//! Drive the formatter / minifier engines in `tcl-lsp-core`.

use std::path::Path;

use anyhow::Context;
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, read_input_documents,
    registry_for_dialect, write_highlighted_output, write_text_output,
};
use tcl_lsp_core::formatting::{FormatterConfig, IndentStyle, formatting_with};
use tcl_lsp_core::minify::{
    SymbolMap, minify_tcl, minify_tcl_aggressive, minify_tcl_compact, remap_line_references,
    unminify_error,
};

use std::collections::HashSet;

use tcl_compiler::optimiser::optimise_source_multipass_filtered;
use tcl_compiler::optimiser::profiles::{OptimisationProfile, profile_to_disabled};

use crate::cli::{ColourArgs, InputArgs};

/// Default tab-expansion width used on stdout (the CLI default).
const DEFAULT_TAB_WIDTH: usize = 4;

/// `tcl format` — pretty-print each input with canonical style rules.
pub fn run_format(
    input: &InputArgs,
    indent_size: Option<usize>,
    indent_style: Option<&str>,
    max_line_length: Option<usize>,
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);

    let mut config = FormatterConfig {
        // Tokenise with the dialect's rules so, e.g., an iRule's `}{` (valid in
        // TMM) parses as two words and is re-emitted as `} {`.
        lexer_config: tcl_lexer::LexerConfig::for_dialect(&dialect),
        ..Default::default()
    };
    if let Some(size) = indent_size {
        config.indent_size = size;
    }
    if let Some(style) = indent_style {
        config.indent_style = if style == "tabs" {
            IndentStyle::Tabs
        } else {
            IndentStyle::Spaces
        };
    }
    if let Some(max) = max_line_length {
        config.max_line_length = max;
    }

    // `formatting_with` returns a single whole-document edit, or an empty Vec
    // when the source is already canonical.
    let edits = formatting_with(&source, &config, registry);
    let formatted = edits
        .into_iter()
        .next()
        .map_or(source, |edit| edit.new_text);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);
    write_highlighted_output(&target, &formatted, use_colour, DEFAULT_TAB_WIDTH, &dialect)?;
    Ok(0)
}

/// `tcl opt` — run the optimiser and emit rewritten Tcl.
///
/// Profile semantics: `full` (the default) is a single
/// pass; only `aggressive` runs multi-pass to a fixpoint (max 5 iterations).
pub fn run_opt(
    input: &InputArgs,
    profile: &str,
    disable: &[String],
    enable: &[String],
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);

    let profile = OptimisationProfile::parse(profile);
    let mut disabled: HashSet<String> = profile_to_disabled(profile)
        .into_iter()
        .map(str::to_owned)
        .collect();
    for raw in disable {
        for code in raw.split(',') {
            let code = code.trim();
            if !code.is_empty() {
                disabled.insert(code.to_ascii_uppercase());
            }
        }
    }
    for raw in enable {
        for code in raw.split(',') {
            let code = code.trim();
            if !code.is_empty() {
                disabled.remove(&code.to_ascii_uppercase());
            }
        }
    }

    // Profile spec (`profile_spec`): only `aggressive` is multi-pass (max 5 iters);
    // every other profile (including `full`) is a single pass. Both honour the
    // disabled set on every pass (matching `optimise_source_multipass(disabled=…)`).
    let (optimised, optimisations, _iterations) = optimise_source_multipass_filtered(
        &source,
        registry,
        Some(dialect.as_str()),
        profile.max_iterations(),
        &disabled,
    );

    let target = OutputTarget::from_arg(input.output.as_deref());
    let mut rendered = optimised;
    // On stdout a comment block summarising the rewrites is appended.
    if target.is_stdout() && !optimisations.is_empty() {
        let mut lines = vec![
            "\n\n# -------------".to_owned(),
            format!("# optimised: {} rewrite(s)", optimisations.len()),
        ];
        for o in &optimisations {
            lines.push(format!("# {}  {}", o.code, o.message));
        }
        rendered = format!(
            "{}\n{}\n",
            rendered.trim_end_matches('\n'),
            lines.join("\n")
        );
    }

    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);
    write_highlighted_output(&target, &rendered, use_colour, DEFAULT_TAB_WIDTH, &dialect)?;

    if !target.is_stdout() {
        eprintln!(
            "optimised {} input(s); rewrites={}",
            documents.len(),
            optimisations.len()
        );
    }
    Ok(0)
}

/// `tcl minify` — strip comments, collapse whitespace, join commands.
pub fn run_minify(
    input: &InputArgs,
    compact: bool,
    symbol_map: Option<&Path>,
    aggressive: bool,
    isolated: bool,
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);

    let (rendered, map) = if aggressive {
        let result = minify_tcl_aggressive(&source, &dialect, isolated, registry);
        (result.source, Some(result.symbol_map))
    } else if compact {
        let (minified, sm) = minify_tcl_compact(&source, &dialect, isolated, registry);
        (minified, Some(sm))
    } else {
        (minify_tcl(&source, &dialect, registry), None)
    };

    write_highlighted_output(&target, &rendered, use_colour, DEFAULT_TAB_WIDTH, &dialect)?;

    if let Some(path) = symbol_map {
        // Always honour `--symbol-map FILE`, even for plain minify (which does
        // no renaming and so produces an empty, identity symbol map).
        // Skipping the write left the file uncreated, so a later
        // `unminify-error` failed on a missing path — and the flag's help
        // documents no dependency on `--compact`/`--aggressive` (issue 198).
        let map_text = map.unwrap_or_default().format();
        write_text_output(&OutputTarget::File(path.to_path_buf()), &map_text)?;
    }
    Ok(0)
}

/// `tcl unminify-error` — translate a minified-code error back to original
/// names (and, with both sources, remap line references).
pub fn run_unminify_error(
    symbol_map_path: &Path,
    error: Option<&str>,
    error_file: Option<&Path>,
    minified: Option<&Path>,
    original: Option<&Path>,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let symbol_map_text = std::fs::read_to_string(symbol_map_path)
        .with_context(|| format!("failed to read {}", symbol_map_path.display()))?;
    let symbol_map = SymbolMap::parse(&symbol_map_text);

    let error_text = if let Some(text) = error {
        text.to_owned()
    } else if let Some(file) = error_file {
        if file.as_os_str() == "-" {
            std::io::read_to_string(std::io::stdin()).context("failed to read stdin")?
        } else {
            std::fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?
        }
    } else {
        eprintln!("error: provide --error TEXT or --error-file FILE");
        return Ok(1);
    };

    let mut translated = error_text;
    if let (Some(min_path), Some(orig_path)) = (minified, original) {
        let minified_source = std::fs::read_to_string(min_path)
            .with_context(|| format!("failed to read {}", min_path.display()))?;
        let original_source = std::fs::read_to_string(orig_path)
            .with_context(|| format!("failed to read {}", orig_path.display()))?;
        translated = remap_line_references(&translated, &minified_source, &original_source);
    }
    translated = unminify_error(&translated, &symbol_map);

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &translated)?;
    Ok(0)
}
