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

use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Serialize;
use tcl_cli_support::{OutputTarget, ensure_ascii, read_input_documents, write_text_output};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::segmenter::segment_with_recovery;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

use crate::cli::InputArgs;

/// Variable-target commands: arguments at these positions name a variable (not
/// a value), so a rename must rewrite them alongside `$` references.
fn var_target_positions(cmd: &str) -> Option<&'static [usize]> {
    match cmd {
        "set" | "variable" | "incr" | "append" | "lappend" => Some(&[0]),
        "global" | "unset" => Some(&[0, 1, 2, 3, 4, 5, 6, 7]),
        "lassign" => Some(&[1, 2, 3, 4, 5, 6, 7]),
        _ => None,
    }
}

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

/// Whether `code` fires anywhere in `source` under `dialect`.
fn fires(source: &str, code: &str, dialect: &str) -> bool {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == code)
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

/// Return `(start, end, new_text)` byte-offset edits renaming user variables.
/// Covers `$`/`${}` references (VAR tokens, base name only) and variable-target
/// command arguments.
fn collect_rename_edits(
    source: &str,
    names_seen: &mut HashMap<String, String>,
) -> Vec<(usize, usize, String)> {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let sm = SourceMap::new(source);

    // 1. `$`/`${}` references — VAR tokens (rename the scalar/array base).
    if let Ok(tokens) = Lexer::new(source).tokenise_all() {
        for tok in &tokens {
            if tok.kind != TokenType::Var {
                continue;
            }
            let inner = sm.token_text(*tok);
            let base = inner.split('(').next().unwrap_or(inner);
            if let Some(short) = short_for(base, names_seen) {
                let name_start = tok.span.start() as usize + tok.content_offset as usize;
                edits.push((name_start, name_start + base.len(), short));
            }
        }
    }

    // 2. Variable-target command arguments (def sites: `set x`, `global a`).
    // No registry is available in this identifier-renaming pass, so the
    // known-command universe is empty (the recovery diagnostics are discarded
    // below anyway; only the segmented command shapes matter here).
    let (cmds, _) = segment_with_recovery(
        source,
        LexerConfig::default(),
        &tcl_compiler::analyser::utils::RecoveryKnownCommands::default(),
    );
    for cmd in &cmds {
        if cmd.texts.is_empty() {
            continue;
        }
        let Some(positions) = var_target_positions(&cmd.texts[0]) else {
            continue;
        };
        let arg_tokens = if cmd.argv.len() > 1 {
            &cmd.argv[1..]
        } else {
            &[][..]
        };
        for &pos in positions {
            let Some(atok) = arg_tokens.get(pos) else {
                continue;
            };
            // Only plain bare-literal names (not $-substituted / quoted / braced).
            if atok.kind != TokenType::Esc {
                continue;
            }
            let text = sm.token_text(*atok);
            let base = text.split('(').next().unwrap_or(text);
            if base != text {
                continue;
            }
            if let Some(short) = short_for(base, names_seen) {
                let start = atok.span.start() as usize;
                edits.push((start, start + base.len(), short));
            }
        }
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
    dialect: &str,
) -> Result<MinimizeResult, MinimizeError> {
    let original_lines = source.matches('\n').count() + 1;
    if !fires(source, code, dialect) {
        return Err(MinimizeError::NotPresent);
    }

    // 1. Structural reduction over lines.
    let units: Vec<String> = source.split('\n').map(str::to_owned).collect();
    let test = |u: &[String]| fires(&u.join("\n"), code, dialect);
    let mut reduced = ddmin(units, &test).join("\n");

    // Dedent must not change the result; revert if it stops the diagnostic.
    let dedented = dedent(&reduced);
    if fires(&dedented, code, dialect) {
        reduced = dedented;
    }

    // 2. Verify-gated identifier rename.
    let mut did_rename = false;
    if rename {
        let mut names_seen: HashMap<String, String> = HashMap::new();
        let edits = collect_rename_edits(&reduced, &mut names_seen);
        if !edits.is_empty() {
            let candidate = apply_edits(&reduced, &edits);
            if fires(&candidate, code, dialect) {
                reduced = candidate;
                did_rename = true;
            }
        }
    }

    let reproduces = fires(&reduced, code, dialect);
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
    for document in &documents {
        // Per document, like `diag`: the reproducer is reduced under the same
        // dialect the diagnostic was reported under.
        let dialect = document.effective_dialect(input.dialect.as_deref());
        match minimize_diagnostic(&document.source, code, rename, &dialect) {
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

    // Honour the shared `-o/--output FILE` flag (default stdout), issue 196.
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
}
