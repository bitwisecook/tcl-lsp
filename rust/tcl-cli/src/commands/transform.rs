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
    SymbolMap, minify_tcl, minify_tcl_aggressive_with, minify_tcl_compact, remap_line_references,
    unminify_error,
};

use std::collections::HashSet;

use tcl_compiler::optimiser::optimise_source_multipass_filtered;
use tcl_compiler::optimiser::profiles::{OptimisationProfile, profile_to_disabled};

use crate::cli::{ColourArgs, InputArgs};

/// Default tab-expansion width used on stdout (the CLI default).
const DEFAULT_TAB_WIDTH: usize = 4;

/// The `tcl format` formatter configuration for `dialect`, with the CLI's
/// style overrides applied.
///
/// The resolved dialect is the formatter's whole dialect story (issue #1465):
/// its lexer grammar (so, e.g., an iRule's `}{` parses as two words and is
/// re-emitted as `} {`), the release its rewrite candidates are filtered
/// against, and the forward range those rewrites must stay correct across all
/// come from this one profile. `--dialect` names it; otherwise it is the
/// documents' detected dialect.
fn format_config(
    dialect: &str,
    indent_size: Option<usize>,
    indent_style: Option<&str>,
    max_line_length: Option<usize>,
) -> FormatterConfig {
    let mut config = FormatterConfig::for_dialect(dialect);
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
    config
}

/// `tcl format` — pretty-print each input with canonical style rules.
pub fn run_format(
    input: &InputArgs,
    indent_size: Option<usize>,
    indent_style: Option<&str>,
    max_line_length: Option<usize>,
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(dialect.name);

    let config = format_config(dialect.name, indent_size, indent_style, max_line_length);

    // `formatting_with` returns a single whole-document edit, or an empty Vec
    // when the source is already canonical.
    let edits = formatting_with(&source, &config, &registry);
    let formatted = edits
        .into_iter()
        .next()
        .map_or(source, |edit| edit.new_text);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);
    write_highlighted_output(
        &target,
        &formatted,
        use_colour,
        DEFAULT_TAB_WIDTH,
        dialect.name,
    )?;
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
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(dialect.name);

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
        &registry,
        Some(dialect.name),
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
    write_highlighted_output(
        &target,
        &rendered,
        use_colour,
        DEFAULT_TAB_WIDTH,
        dialect.name,
    )?;

    if !target.is_stdout() {
        eprintln!(
            "optimised {} input(s); rewrites={}",
            documents.len(),
            optimisations.len()
        );
    }
    Ok(0)
}

/// Which minification tier `tcl minify` runs.
///
/// `--aggressive` wins over `--compact` when both are given, matching the
/// long-standing flag precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinifyTier {
    /// Whitespace and comments only.
    Default,
    /// `--compact`: also compact proc-local names.
    Compact,
    /// `--aggressive`: optimiser rewrites, compaction, aliasing, and
    /// keyword abbreviations.
    Aggressive,
}

impl MinifyTier {
    /// The tier the `--compact` / `--aggressive` flags select.
    #[must_use]
    pub const fn from_flags(compact: bool, aggressive: bool) -> Self {
        if aggressive {
            Self::Aggressive
        } else if compact {
            Self::Compact
        } else {
            Self::Default
        }
    }
}

/// Per-run minification switches, grouped so the entry point does not take a
/// row of positional booleans.
#[derive(Debug, Clone, Copy)]
pub struct MinifyOptions {
    /// Which tier to run.
    pub tier: MinifyTier,
    /// Emit unique-prefix keyword abbreviations (the inverse of
    /// `--no-abbreviations`). Only consulted for the aggressive tier.
    pub abbreviations: bool,
    /// The script is self-contained (`--isolated`).
    pub isolated: bool,
}

/// `tcl minify` — strip comments, collapse whitespace, join commands.
pub fn run_minify(
    input: &InputArgs,
    symbol_map: Option<&Path>,
    options: MinifyOptions,
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let MinifyOptions {
        tier,
        abbreviations,
        isolated,
    } = options;
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(dialect.name);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);

    let (rendered, map) = match tier {
        MinifyTier::Aggressive => {
            let result = minify_tcl_aggressive_with(
                &source,
                dialect.name,
                isolated,
                &registry,
                abbreviations,
            );
            (result.source, Some(result.symbol_map))
        }
        MinifyTier::Compact => {
            let (minified, sm) = minify_tcl_compact(&source, dialect.name, isolated, &registry);
            (minified, Some(sm))
        }
        MinifyTier::Default => (minify_tcl(&source, dialect.name, &registry), None),
    };

    write_highlighted_output(
        &target,
        &rendered,
        use_colour,
        DEFAULT_TAB_WIDTH,
        dialect.name,
    )?;

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

#[cfg(test)]
mod tests {
    use super::format_config;

    /// `tcl format` must resolve the document's dialect onto the formatter,
    /// not format everything as modern Tcl (issue #1465). `}{` is the
    /// discriminator: TMM parses it as two words, stock Tcl does not.
    #[test]
    fn the_resolved_dialect_reaches_the_formatter() {
        let irules = format_config("f5-irules", None, None, None);
        assert!(irules.profile.is_irules());
        assert!(irules.lexer_config().irules_brace_separator);

        let registry = tcl_registry::registry_for_dialect("f5-irules");
        let source = "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";
        let out = tcl_lsp_core::formatting::format_tcl(source, &irules, registry);
        assert!(out.contains("} {"), "{out}");
        assert!(!out.contains("}{"), "{out}");

        // A Tcl release resolves its own profile, and leaves those bytes
        // alone.
        let tcl9 = format_config("tcl9.0", None, None, None);
        assert!(!tcl9.lexer_config().irules_brace_separator);
        let out = tcl_lsp_core::formatting::format_tcl(source, &tcl9, registry);
        assert!(out.contains("}{"), "{out}");
    }

    /// The style overrides still apply on top of the profile.
    #[test]
    fn the_style_overrides_apply_on_top_of_the_profile() {
        let cfg = format_config("f5-irules", Some(2), Some("tabs"), Some(70));
        assert!(cfg.profile.is_irules());
        assert_eq!(cfg.indent_size, 2);
        assert_eq!(
            cfg.indent_style,
            tcl_lsp_core::formatting::IndentStyle::Tabs
        );
        assert_eq!(cfg.max_line_length, 70);
    }
}
