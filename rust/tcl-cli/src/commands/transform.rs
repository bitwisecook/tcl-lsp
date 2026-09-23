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
    OutputTarget, combine_sources, combine_texts, combined_effective_dialect, read_input_documents,
    registry_for_dialect, write_highlighted_output, write_text_output,
};
use tcl_lsp_core::formatting::{FormatterConfig, IndentStyle, formatting_with};
use tcl_lsp_core::minify::{
    SymbolMap, minify_tcl, minify_tcl_aggressive_with, minify_tcl_compact, remap_line_references,
    unminify_error,
};

use tcl_compiler::optimiser::profiles::OptimisationProfile;
use tcl_lsp_core::diagnostic_report::optimise_under_policy;

use crate::cli::{ColourArgs, InputArgs};
use crate::commands::policy::{ConfigLayers, invocation_layer};

/// Default tab-expansion width used on stdout (the CLI default).
const DEFAULT_TAB_WIDTH: usize = 4;

/// The `tcl format` formatter configuration for `dialect`, with the CLI's
/// style overrides applied.
///
/// The resolved dialect is the formatter's whole dialect story:
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
    let mut config =
        FormatterConfig::for_profile(tcl_cli_support::environment::profile_for_dialect(dialect));
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
///
/// Kept on `combine_sources` after #2120's investigation of `run_opt`'s
/// concatenation bug: the formatter is a pure syntactic rewriter
/// (`format_tcl(source) -> source`, see `formatter-engine.md`) that never
/// folds a value from one statement into another the way the optimiser's
/// constant propagation does, so concatenating documents before formatting
/// cannot produce the "ran a variable's *value* across a file boundary that
/// never shares a scope at run time" defect #2120 is about.
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
    write_highlighted_output(&target, &formatted, use_colour, DEFAULT_TAB_WIDTH, dialect)?;
    Ok(0)
}

/// `tcl opt` — run the optimiser and emit rewritten Tcl.
///
/// Profile semantics: `full` (the default when nothing names a profile) is a
/// single pass; only `aggressive` runs multi-pass to a fixpoint (max 5
/// iterations).
///
/// Unlike `format` and `minify`, each input is *analysed* as its own program —
/// its own dialect, its own directives, its own policy, its own optimiser
/// pass — rather than concatenated with the others first. The rendered output
/// is still one script, so `tcl opt src/ -o build/optimised.tcl` keeps doing
/// what the README documents. Two files named on one command line
/// never share a scope at run time (they are two separate `tclsh` loads), so
/// folding a store in one across into a load in the other is simply wrong: it
/// forwards a value the second file could never actually see, and can delete
/// the first file's store as "dead" when the only "use" the optimiser found
/// was the second file (#2120). `tcl diag`'s cross-file call-site evidence is
/// a different, opt-in thing — analysing how programs *call* each other, not
/// asserting they execute in one shared frame — so it does not apply here.
/// A user who wants several files optimised as one unit can concatenate them
/// themselves; this verb must not do it silently.
///
/// A rewrite is a finding like any other (`docs/design/compiler/diagnostic-policy.md`
/// § Adapters): only the rewrites the document's policy shows are applied,
/// so a `# noqa` on the command, a file-wide `# tcl-lsp: disable=*`, a code
/// the profile or a configuration layer turned off, mean to a rewrite what
/// they mean to a squiggle. `--disable` and `--enable` are the verb's
/// invocation layer; the global `config.ini` and each input file's own
/// project `.tcl-lsp.ini` are the others. `--profile` is not a layer: named,
/// it is the profile in force over both files, and omitted, the project
/// file's `[optimiser] profile`, then the global file's, then `full` — the
/// profile is a request parameter with a project default
/// (`docs/design/compiler/diagnostic-policy.md` § Configuration). The profile
/// in force decides the passes as well as the categories.
pub fn run_opt(
    input: &InputArgs,
    profile: Option<&str>,
    disable: &[String],
    enable: &[String],
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let explicit_dialect = input.dialect_profile()?;

    let requested = profile.map(OptimisationProfile::parse);
    let layers = ConfigLayers::new(invocation_layer(disable, enable, "optimiser"));

    let target = OutputTarget::from_arg(input.output.as_deref());
    // Per-file is the unit of *analysis*, not of output. The rendered text
    // stays one program, joined exactly as `combine_sources` joined the
    // inputs, because `tcl opt src/ -o build/optimised.tcl` is documented
    // (README, `kcs-feature-tcl-verb-cli`) as optimising a tree "into one
    // output script": a `# file:` banner in the program text would push a
    // leading `#!` off byte zero and stop that script being executable.
    // Which file each rewrite came from is reported in the trailing comment
    // summary instead, where it cannot corrupt the program.
    //
    // Optimising each input separately and then concatenating is safe for
    // both readings: if the files really are loaded together the result is
    // merely conservative (cross-file folds are missed), whereas the old
    // optimise-the-concatenation order was *unsound* when they are not.
    let multi_file = documents.len() > 1;
    let mut sections: Vec<String> = Vec::with_capacity(documents.len());
    let mut per_file: Vec<(String, Vec<String>)> = Vec::with_capacity(documents.len());
    let mut total_rewrites = 0usize;

    for document in &documents {
        let dialect = document.effective_dialect(explicit_dialect);
        let registry = registry_for_dialect(dialect.name);
        let source = document.analysis_source();
        let policy = layers
            .builder_for(document.path.as_deref())
            .requested_profile(requested)
            .default_profile(OptimisationProfile::Full)
            .dialect(dialect)
            .build();

        // Profile spec (`profile_spec`): only `aggressive` is multi-pass (max 5
        // iters); every other profile (including `full`) is a single pass.
        // Every pass applies only the rewrites the policy shows.
        let out = optimise_under_policy(
            &source,
            &registry,
            Some(dialect),
            policy.optimiser.profile.max_iterations(),
            &policy,
        );
        total_rewrites += out.applied.len();
        per_file.push((
            document.label.clone(),
            out.applied
                .iter()
                .map(|o| format!("# {}  {}", o.code, o.message))
                .collect(),
        ));
        sections.push(out.text);
    }

    // A single input keeps the pre-#2120 bytes exactly — no trim, no join.
    let mut rendered = if multi_file {
        combine_texts(sections.iter().map(String::as_str))
    } else {
        sections.into_iter().next().unwrap_or_default()
    };

    // On stdout a comment block summarising the rewrites *applied* is
    // appended. With several inputs each file's rewrites are listed under its
    // own `# file:` line, inside the comment block.
    if target.is_stdout() && total_rewrites > 0 {
        let mut lines = vec![
            "\n\n# -------------".to_owned(),
            format!("# optimised: {total_rewrites} rewrite(s)"),
        ];
        for (label, entries) in &per_file {
            if entries.is_empty() {
                continue;
            }
            if multi_file {
                lines.push(format!("# file: {label}"));
            }
            lines.extend(entries.iter().cloned());
        }
        rendered = format!(
            "{}\n{}\n",
            rendered.trim_end_matches('\n'),
            lines.join("\n")
        );
    }

    // Highlighting is cosmetic styling of the rendered text, not an analysis
    // decision, so one dialect for the whole rendered block (the same
    // detection-order fallback `combined_effective_dialect` always used) is
    // fine even when the inputs' own dialects differ.
    let dialect = combined_effective_dialect(&documents, explicit_dialect);
    let use_colour = tcl_cli_support::resolve_use_colour(colour.colour, colour.no_colour, &target);
    write_highlighted_output(&target, &rendered, use_colour, DEFAULT_TAB_WIDTH, dialect)?;

    if !target.is_stdout() {
        eprintln!(
            "optimised {} input(s); rewrites={}",
            documents.len(),
            total_rewrites
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
///
/// Kept on `combine_sources` after #2120's investigation of `run_opt`'s
/// concatenation bug: unlike `opt`, whose output stays N separate files that
/// are never run together, `minify` has no per-file output (one stream, no
/// `--in-place`) — several inputs name a bundle that is meant to be sourced
/// as a single deployment artifact. Once bundled, the files *do* share one
/// runtime scope, so `--aggressive`'s optimiser pass, name compaction and
/// cross-command aliasing over the whole bundle are correct for what this
/// verb produces, not the same defect as `opt` folding across files that stay
/// separate.
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
            let result =
                minify_tcl_aggressive_with(&source, dialect, isolated, &registry, abbreviations);
            (result.source, Some(result.symbol_map))
        }
        MinifyTier::Compact => {
            let (minified, sm) = minify_tcl_compact(&source, dialect, isolated, &registry);
            (minified, Some(sm))
        }
        MinifyTier::Default => (minify_tcl(&source, dialect, &registry), None),
    };

    write_highlighted_output(&target, &rendered, use_colour, DEFAULT_TAB_WIDTH, dialect)?;

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
    /// not format everything as modern Tcl. `}{` is the discriminator: TMM
    /// parses it as two words, stock Tcl does not.
    #[test]
    fn the_resolved_dialect_reaches_the_formatter() {
        let irules = format_config("f5-irules", None, None, None);
        assert!(irules.profile.is_irules());
        assert!(irules.lexer_config().irules_brace_separator);

        let registry = tcl_registry::model::static_context_for("f5-irules").commands();
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
