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

//! `minimize` (`minimise` / `repro`) verb: reduce a diagnostic to a minimal
//! reproducer.
//!
//! Every candidate retains one complete analysis input. Variable proposals use
//! the editor's atomic original-symbol rename planner, including selected
//! command operands, alias coverage and array/root geometry. Missing source
//! ownership withholds rename edits. Diagnostic preservation is the reducer's
//! acceptance condition; it supplies no runtime equivalence claim.
//!
//! Two layers:
//!
//! - The engine: delta-debugging
//!   over source lines, gated by "the target diagnostic still fires", followed
//!   by a verify-gated identifier-rename pass (collect rename edits over the
//!   tokeniser + segmenter, apply them, then dedent). When the code does not
//!   fire on the input the reduction fails with
//!   `Err(MinimizeError::NotPresent)`.
//! - The verb: iterate the input documents,
//!   skip those where the code does not fire, and print the per-document result
//!   as text or JSON.
//!
//! The predicate is membership-only ("does CODE fire?"), so the accepted
//! diagnostic-ordering divergence between the analysers is
//! irrelevant to the reduction — both engines reduce to a snippet that still
//! fires CODE.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::Arc;

use serde::Serialize;
use tcl_cli_support::{OutputTarget, ensure_ascii, read_input_documents, write_text_output};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{LexerConfig, LineIndex, SourceImage, Utf16Col};

use crate::cli::InputArgs;

/// Tcl specials / globals whose names are semantically load-bearing — never
/// renamed.
const RESERVED_NAMES: [&str; 13] = [
    "argv",
    "argv0",
    "argc",
    "env",
    "errorInfo",
    "errorCode",
    "tcl_platform",
    "auto_path",
    "this",
    "self",
    "type",
    "win",
    "selfns",
];

/// Outcome of minimising a diagnostic to a minimal reproducer (the
/// `MinimizeResult` structure).
#[derive(Debug, Clone)]
pub struct MinimizeResult {
    pub source: String,
    pub code: String,
    pub original_lines: usize,
    pub reduced_lines: usize,
    pub renamed: bool,
    pub reproduces: bool,
}

/// Why `minimize_diagnostic` could not run.
#[derive(Debug)]
pub enum MinimizeError {
    /// The requested code does not fire on the given source.
    NotPresent,
}

/// Analyse each candidate with the same complete command generation and grammar.
fn analyse_source(source: &str, input: &ResolvedAnalysisInput) -> AnalysisResult {
    Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name)
}

/// Whether `code` fires anywhere in `source` under the retained input.
fn fires(source: &str, code: &str, input: &ResolvedAnalysisInput) -> bool {
    analyse_source(source, input)
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_str() == code)
}

/// Zeller delta-debugging: minimise `units` keeping `test` true. Returns the
/// smallest sublist of `units` for which `test(&sublist)` holds.
fn ddmin<F>(mut units: Vec<String>, test: &F) -> Vec<String>
where
    F: Fn(&[String]) -> bool,
{
    let mut n = 2usize;
    while units.len() >= 2 {
        let chunk = (units.len() / n).max(1);
        let mut removed_any = false;
        let mut i = 0usize;
        while i < units.len() {
            let mut complement = Vec::with_capacity(units.len());
            complement.extend_from_slice(&units[..i]);
            complement.extend_from_slice(&units[(i + chunk).min(units.len())..]);
            if !complement.is_empty() && test(&complement) {
                units = complement;
                n = n.saturating_sub(1).max(2);
                removed_any = true;
                break;
            }
            i += chunk;
        }
        if !removed_any {
            if n >= units.len() {
                break;
            }
            n = (n * 2).min(units.len());
        }
    }
    units
}

/// Assign the next short name (`a`, `b`, …, `z`, `a1`, `b1`, …) for `name`,
/// or `None` for names that must never be renamed.
fn short_for(name: &str, names_seen: &mut HashMap<String, String>) -> Option<String> {
    if RESERVED_NAMES.contains(&name)
        || name.contains("::")
        || name.is_empty()
        || name.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        return None;
    }
    if let Some(existing) = names_seen.get(name) {
        return Some(existing.clone());
    }
    let idx = names_seen.len();
    let letter = char::from(b'a' + u8::try_from(idx % 26).expect("idx % 26 is 0..26"));
    let suffix = idx / 26;
    let short = if suffix == 0 {
        letter.to_string()
    } else {
        format!("{letter}{suffix}")
    };
    names_seen.insert(name.to_owned(), short.clone());
    Some(short)
}

/// Propose short names for authentic original variable symbols. The shared
/// rename owner selects every operand, lexical root and alias occurrence using
/// the complete current source, actual grammar and retained command world.
/// Unavailable original metadata supplies no edits. This is reproducer editing,
/// not the semantic equivalence contract required by the minifier.
fn collect_rename_edits(
    source: &str,
    analysis: &AnalysisResult,
    names_seen: &mut HashMap<String, String>,
) -> Vec<(usize, usize, String)> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config) {
        return Vec::new();
    }
    let index = LineIndex::new(source);
    let mut occurrences = analysis
        .original_variable_symbols
        .iter()
        .collect::<Vec<_>>();
    occurrences.sort_by_key(|occurrence| (occurrence.span().start(), occurrence.span().end()));
    let mut selected = HashSet::new();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for occurrence in occurrences {
        let symbol = occurrence.symbol();
        if !selected.insert(symbol) {
            continue;
        }
        let Some(name) = symbol
            .rename_tail()
            .and_then(|name| std::str::from_utf8(name).ok())
        else {
            continue;
        };
        let Some(short) = short_for(name, names_seen) else {
            continue;
        };
        let Ok(plan) = tcl_lsp_core::variable_symbol::original_variable_rename_edits(
            source, analysis, symbol, &short,
        ) else {
            continue;
        };
        let plan = plan
            .into_iter()
            .map(|edit| {
                let start = index.offset_at_utf16(
                    edit.range.start_line,
                    Utf16Col::new(edit.range.start_character),
                    source,
                ) as usize;
                let end = index.offset_at_utf16(
                    edit.range.end_line,
                    Utf16Col::new(edit.range.end_character),
                    source,
                ) as usize;
                source.get(start..end).map(|_| (start, end, edit.new_text))
            })
            .collect::<Option<Vec<_>>>();
        let Some(plan) = plan else {
            continue;
        };
        // Two symbols can share one original list container. Accept the whole
        // symbol's plan only when every edit is disjoint from previous plans;
        // retaining just some occurrences would change the reproducer's names.
        if plan.iter().any(|(start, end, _)| {
            edits
                .iter()
                .any(|(prior_start, prior_end, _)| start < prior_end && prior_start < end)
        }) {
            continue;
        }
        edits.extend(plan);
    }
    edits
}

/// Apply non-overlapping `(start, end, text)` edits right-to-left.
fn apply_edits(source: &str, edits: &[(usize, usize, String)]) -> String {
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut uniq: Vec<(usize, usize, String)> = Vec::new();
    for (s, e, t) in edits {
        if seen.insert((*s, *e)) {
            uniq.push((*s, *e, t.clone()));
        }
    }
    uniq.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = source.to_owned();
    for (s, e, t) in uniq {
        out.replace_range(s..e, &t);
    }
    out
}

/// Strip common leading whitespace from non-blank lines.
fn dedent(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let common = lines
        .iter()
        .filter(|ln| !ln.trim().is_empty())
        .map(|ln| ln.len() - ln.trim_start().len())
        .min();
    let Some(common) = common else {
        return source.to_owned();
    };
    if common == 0 {
        return source.to_owned();
    }
    lines
        .iter()
        .map(|ln| {
            if ln.trim().is_empty() {
                (*ln).to_owned()
            } else {
                strip_leading_indent(ln, common).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip up to `common` bytes of leading whitespace from `ln`, advancing only
/// over whole whitespace characters so a multibyte indent char (e.g. U+00A0) is
/// never split. A raw `ln[common..]` panicked ("byte index is not a char
/// boundary") when lines mixed multibyte whitespace with ASCII spaces so that
/// `common` fell inside a char.
fn strip_leading_indent(ln: &str, common: usize) -> &str {
    let mut end = 0;
    for (i, ch) in ln.char_indices() {
        if i >= common || !ch.is_whitespace() {
            break;
        }
        end = i + ch.len_utf8();
    }
    &ln[end..]
}

/// Reduce `source` to the minimum code still firing diagnostic `code` under
/// `dialect`.
pub fn minimize_diagnostic(
    source: &str,
    code: &str,
    rename: bool,
    dialect: &tcl_dialect::DialectProfile,
) -> Result<MinimizeResult, MinimizeError> {
    let dialect = dialect.intern();
    let registry = tcl_cli_support::registry_for_dialect(dialect.name);
    let generation = tcl_lsp_core::context_for_dialect_profile(dialect);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(
        dialect,
        dialect,
        context,
        LexerConfig::for_file_grammar(dialect.grammar),
    );
    minimize_diagnostic_with_input(source, code, rename, &input)
}

/// Reduce a diagnostic while retaining the driver's actual availability, unit
/// profile and full grammar. Every changed source receives a fresh analysis
/// under this same immutable input; a supplied unknown remains unavailable.
pub fn minimize_diagnostic_with_input(
    source: &str,
    code: &str,
    rename: bool,
    input: &ResolvedAnalysisInput,
) -> Result<MinimizeResult, MinimizeError> {
    let original_lines = source.matches('\n').count() + 1;
    if !fires(source, code, input) {
        return Err(MinimizeError::NotPresent);
    }

    // 1. Structural reduction over lines.
    let units: Vec<String> = source.split('\n').map(str::to_owned).collect();
    let test = |u: &[String]| fires(&u.join("\n"), code, input);
    let mut reduced = ddmin(units, &test).join("\n");

    // Dedent must not change the result; revert if it stops the diagnostic.
    let dedented = dedent(&reduced);
    if fires(&dedented, code, input) {
        reduced = dedented;
    }

    // 2. Verify-gated identifier rename.
    let mut did_rename = false;
    if rename {
        let mut names_seen: HashMap<String, String> = HashMap::new();
        let analysis = analyse_source(&reduced, input);
        let edits = collect_rename_edits(&reduced, &analysis, &mut names_seen);
        if !edits.is_empty() {
            let candidate = apply_edits(&reduced, &edits);
            if fires(&candidate, code, input) {
                reduced = candidate;
                did_rename = true;
            }
        }
    }

    let reproduces = fires(&reduced, code, input);
    let reduced_lines = reduced.matches('\n').count() + 1;
    Ok(MinimizeResult {
        source: reduced,
        code: code.to_owned(),
        original_lines,
        reduced_lines,
        renamed: did_rename,
        reproduces,
    })
}

/// One per-document result in the `minimize --json` payload. Field order is
/// fixed (`file, code, original_lines, reduced_lines, renamed,
/// reproduces, source`); `serde_json::to_string_pretty` preserves declaration
/// order, so the JSON is byte-faithful.
#[derive(Serialize)]
struct MinimizeItem {
    file: String,
    code: String,
    original_lines: usize,
    reduced_lines: usize,
    renamed: bool,
    reproduces: bool,
    source: String,
}

/// `tcl minimize [INPUT...] CODE` — reduce a diagnostic to a minimal
/// reproducer.
///
/// The diagnostic CODE is the final positional argument (see the `Minimize`
/// command doc in `cli.rs`); the inputs are everything before it.
pub fn run_minimize(input: &InputArgs, no_rename: bool, json: bool) -> anyhow::Result<u8> {
    let Some((code_arg, file_inputs)) = input.inputs.split_last() else {
        anyhow::bail!("the following arguments are required: code");
    };
    let code = code_arg.to_string_lossy();
    let code = code.as_ref();
    let documents = read_input_documents(file_inputs, &input.source, !input.no_recursive)?;
    let rename = !no_rename;

    let mut results: Vec<MinimizeItem> = Vec::new();
    let explicit_dialect = input.dialect_profile()?;
    for document in &documents {
        // Per document, like `diag`: the reproducer is reduced under the same
        // dialect the diagnostic was reported under.
        let dialect = document.effective_dialect(explicit_dialect);
        // A lone-CR document must reduce under the same reading `diag` gave
        // the finding under: the raw form parses as one command under the
        // lexer's whitespace treatment of `\r`, which would make the
        // reducer minimise a mis-parse rather than the diagnostic it was
        // asked for (#1953).
        match minimize_diagnostic(&document.analysis_source(), code, rename, dialect) {
            Ok(r) => results.push(MinimizeItem {
                file: document.label.clone(),
                code: r.code,
                original_lines: r.original_lines,
                reduced_lines: r.reduced_lines,
                renamed: r.renamed,
                reproduces: r.reproduces,
                source: r.source,
            }),
            // Code doesn't fire on this document — skip it.
            Err(MinimizeError::NotPresent) => {}
        }
    }

    if results.is_empty() {
        eprintln!("{code} does not fire on any input.");
        return Ok(1);
    }

    // Honour the shared `-o/--output FILE` flag (default stdout).
    let target = OutputTarget::from_arg(input.output.as_deref());
    if json {
        write_text_output(
            &target,
            &ensure_ascii(&serde_json::to_string_pretty(&results)?),
        )?;
        return Ok(0);
    }

    let mut rendered = String::new();
    for res in &results {
        let renamed = if res.renamed { "True" } else { "False" };
        let _ = writeln!(
            rendered,
            "# {}: {} ({}\u{2192}{} lines, renamed={})",
            res.file, res.code, res.original_lines, res.reduced_lines, renamed
        );
        rendered.push_str(&res.source);
        rendered.push_str("\n\n");
    }
    write_text_output(&target, &rendered)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddmin_keeps_minimal_satisfying_subset() {
        // Predicate: the kept units must include "x". ddmin should reduce a
        // 4-line block down to just that line.
        let units: Vec<String> = ["a", "b", "x", "c"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let reduced = ddmin(units, &|u: &[String]| u.iter().any(|l| l == "x"));
        assert_eq!(reduced, vec!["x".to_owned()]);
    }

    #[test]
    fn ddmin_noop_when_all_required() {
        // Predicate needs both lines, so nothing can be removed.
        let units: Vec<String> = ["a", "b"].iter().map(|s| (*s).to_owned()).collect();
        let reduced = ddmin(units, &|u: &[String]| u.len() == 2);
        assert_eq!(reduced, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn dedent_strips_common_indent_only() {
        assert_eq!(dedent("    a\n      b\n"), "a\n  b\n");
        // A zero-indent line pins the common indent at 0 — no change.
        assert_eq!(dedent("a\n    b"), "a\n    b");
        // Blank lines are ignored when computing the common indent.
        assert_eq!(dedent("  a\n\n  b"), "a\n\nb");
    }

    #[test]
    fn dedent_mixed_multibyte_whitespace_does_not_panic() {
        // One line indented with U+00A0 (2 bytes, whitespace)
        // and another with a single space → common == 1 byte, which falls
        // inside the U+00A0 char. `ln[common..]` panicked; the char-safe strip
        // must advance over whole whitespace chars instead.
        let src = " a\n\u{a0}b";
        let out = dedent(src); // must not panic
        assert!(out.contains('a') && out.contains('b'), "{out:?}");
        // The strip helper never splits a multibyte char.
        assert_eq!(strip_leading_indent("\u{a0}b", 1), "b");
        assert_eq!(strip_leading_indent("  x", 2), "x");
        assert_eq!(strip_leading_indent("x", 4), "x");
    }

    #[test]
    fn apply_edits_rewrites_right_to_left_and_dedups() {
        // Two edits on the same span are de-duplicated (first wins).
        let edits = vec![
            (0usize, 1usize, "X".to_owned()),
            (4usize, 5usize, "Y".to_owned()),
            (0usize, 1usize, "Z".to_owned()),
        ];
        assert_eq!(apply_edits("abcde", &edits), "XbcdY");
    }

    #[test]
    fn short_for_skips_reserved_and_qualified_names() {
        let mut seen = HashMap::new();
        assert_eq!(short_for("env", &mut seen), None);
        assert_eq!(short_for("::ns::v", &mut seen), None);
        assert_eq!(short_for("1abc", &mut seen), None);
        // First user name → "a"; same name is stable; next new name → "b".
        assert_eq!(short_for("foo", &mut seen).as_deref(), Some("a"));
        assert_eq!(short_for("foo", &mut seen).as_deref(), Some("a"));
        assert_eq!(short_for("bar", &mut seen).as_deref(), Some("b"));
    }

    fn input_for(dialect: &str) -> ResolvedAnalysisInput {
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            environment.default_context_registry(),
            LexerConfig::for_file_grammar(environment.grammar()),
        )
    }

    fn renamed_source(source: &str, input: &ResolvedAnalysisInput) -> String {
        let analysis = analyse_source(source, input);
        let edits = collect_rename_edits(source, &analysis, &mut HashMap::new());
        apply_edits(source, &edits)
    }

    #[test]
    fn minimise_variable_names_share_original_array_literal_and_unicode_geometry() {
        // Source edit contract: naming.editor.original-variable-rename-lexical-geometry
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-lexical-geometry.md
        // These are CLI software controls, not native interpreter observations.
        let input = input_for("tcl8.6");
        for (source, expected) in [
            (
                "set long(index) 1; list $long(index) ${long(index)}",
                "set a(index) 1; list $a(index) ${a(index)}",
            ),
            (
                "set {long(open} 1; list ${long(open}",
                "set {a} 1; list ${a}",
            ),
            ("set {$literal} 1; list ${$literal}", "set {a} 1; list ${a}"),
            ("set λlong 1;\nlist $λlong", "set a 1;\nlist $a"),
        ] {
            assert_eq!(renamed_source(source, &input), expected, "{source:?}");
        }
    }

    #[test]
    fn minimise_variable_roles_follow_selected_renames_and_aliases() {
        // Source edit contract: naming.editor.original-variable-rename-frame-currency
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-frame-currency.md
        let input = input_for("tcl8.6");
        for (source, expected) in [
            (
                "rename set saved; saved long 1; list $long",
                "rename set saved; saved a 1; list $a",
            ),
            (
                "interp alias {} assign {} set; assign long 1; list $long",
                "interp alias {} assign {} set; assign a 1; list $a",
            ),
            (
                "rename set saved; interp alias {} set {} puts; saved long 1; set untouched; list $long",
                "rename set saved; interp alias {} set {} puts; saved a 1; set untouched; list $a",
            ),
        ] {
            assert_eq!(renamed_source(source, &input), expected, "{source:?}");
        }
        // The captured name has its own original container. It cannot be
        // replaced by the written value operand of `assign`.
        let captured = "interp alias {} assign {} set fixed; assign 1; list $fixed";
        assert_eq!(renamed_source(captured, &input), captured);
    }

    #[test]
    fn minimise_variable_edits_refuse_missing_stale_and_colliding_inputs() {
        // Source edit contract: naming.editor.original-variable-rename-frame-currency
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-frame-currency.md
        let input = input_for("tcl8.6");
        let source = "set long 1; list $long";
        let mut analysis = analyse_source(source, &input);
        assert!(!collect_rename_edits(source, &analysis, &mut HashMap::new()).is_empty());
        assert!(
            collect_rename_edits("set other 1; list $long", &analysis, &mut HashMap::new())
                .is_empty()
        );
        analysis.resolved_input = None;
        assert!(collect_rename_edits(source, &analysis, &mut HashMap::new()).is_empty());

        let collision = "set long 1; set a 2; list $long $a";
        let renamed = renamed_source(collision, &input);
        assert!(renamed.starts_with("set long 1;"));
        assert!(renamed.ends_with("list $long $b"));
    }

    #[test]
    fn minimise_variable_edits_use_actual_availability_and_atomic_list_containers() {
        // Actual source roles and edits are tested; no body entry or runtime
        // availability is inferred from an assistance declaration.
        let source = "lassign {1 2} first second; list $first $second";
        assert_eq!(renamed_source(source, &input_for("tcl8.4")), source);
        assert_eq!(
            renamed_source(source, &input_for("tcl8.6")),
            "lassign {1 2} a b; list $a $b"
        );
        let shared = "foreach {first second} {1 2} {list $first $second}";
        assert_eq!(
            renamed_source(shared, &input_for("tcl8.6")),
            "foreach {a second} {1 2} {list $a $second}"
        );
    }

    #[test]
    fn minimise_diagnostic_retains_supplied_input_across_reduction() {
        let input = input_for("tcl8.6");
        let source = "# removable\nset unused 1";
        let result = minimize_diagnostic_with_input(source, "W211", true, &input).unwrap();
        assert!(result.reproduces);
        assert!(result.reduced_lines < result.original_lines);
        assert!(fires(&result.source, "W211", &input));
        let analysis = analyse_source(&result.source, &input);
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
    }
}
