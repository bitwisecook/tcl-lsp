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

//! Comment directives: `# noqa` and `# tcl-lsp: disable` suppression, `supports` version targets and `package ... provides`.

use super::script_comments::{
    ScriptCommentFact, script_comment_facts, script_comment_facts_from_analysis,
};
use std::collections::{HashMap, HashSet};

/// Cap on how many leading lines the file-suppression scanner
/// inspects. Pathological all-comment files stop scanning past
/// this.
const FILE_DIRECTIVE_SCAN_LINES: usize = 100;

/// Sentinel line key for a file-wide suppression directive: a
/// `# tcl-lsp: disable=…` at the top of the file is recorded against
/// line `-1` in `suppressed_lines`, alongside the real 0-based lines
/// an inline `# noqa` covers.
pub const FILE_SUPPRESS_KEY: i32 = -1;

/// The one suppression contract every diagnostic surface obeys: is
/// `code` silenced at 0-based `line` by an inline `# noqa` or a
/// top-of-file `# tcl-lsp: disable=…` directive?
///
/// `suppressed` is the analyser's `suppressed_lines` map (see
/// [`parse_noqa_line_suppressions_for_dialect`] and
/// [`parse_file_suppression`], whose results
/// `Analyser::analyse` merges into it). A `"*"` entry silences every
/// code, and the file-level [`FILE_SUPPRESS_KEY`] bucket applies
/// document-wide.
///
/// The analyser records the map but never filters with it — the
/// consumer that renders diagnostics does, because only it knows
/// which line a finding lands on. Both consumers (the language
/// server's publish path and the `diag` / `lint` / `validate` CLI
/// verbs) call this, so a `# noqa` means the same thing in the
/// editor and on the command line; see
/// `docs/kcs/kcs-howto-suppress-diagnostics.md`.
///
/// Generic over both hashers so a caller holding an `FxHashMap` /
/// `FxHashSet` need not rebuild it as a `std` map to ask.
#[must_use]
pub fn line_suppressed<S, T>(
    code: &str,
    line: i32,
    suppressed: &HashMap<i32, HashSet<String, T>, S>,
) -> bool
where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher,
{
    let hit = |key: i32| {
        suppressed
            .get(&key)
            .is_some_and(|codes| codes.contains("*") || codes.contains(code))
    };
    hit(FILE_SUPPRESS_KEY) || hit(line)
}

/// The codes a comment's `# noqa` directive silences, or `None` when the
/// comment carries no directive.
///
/// Two spellings, the ones `docs/kcs/kcs-howto-suppress-diagnostics.md`
/// documents:
///
/// * `noqa:` at a word boundary, anywhere in the comment, silences the codes
///   listed after the colon — so an explanatory prefix (`# see #123, noqa:
///   W210`) still carries the directive. An empty list means every code.
/// * A comment line whose whole body is `noqa` silences every code (the `"*"`
///   sentinel).
///
/// Prose that merely mentions the word is not a directive: `# do not use noqa
/// here` would otherwise silence every finding on the command below it. A
/// suppression nobody asked for is invisible by construction — the finding it
/// hides is the only evidence it happened — so only the shapes above count.
///
/// `comment` is the comment text with or without its leading `#`, one physical
/// line or several.
#[must_use]
pub fn parse_noqa_marker(comment: &str) -> Option<HashSet<String>> {
    if let Some(codes) = coded_noqa_codes(comment) {
        return Some(codes);
    }
    comment
        .lines()
        .any(|line| {
            let body = line.trim().trim_start_matches('#').trim();
            body.eq_ignore_ascii_case("noqa")
        })
        .then(|| std::iter::once("*".to_owned()).collect())
}

/// The code list of the first `noqa:` marker in `comment`, or `None` when it
/// carries none.
///
/// A marker must start a word — `renoqa: W210` is a word, not a directive —
/// and the colon must follow the token immediately.
fn coded_noqa_codes(comment: &str) -> Option<HashSet<String>> {
    // `to_ascii_lowercase` maps only A-Z, so every byte offset it reports is
    // equally valid in `comment`, non-ASCII text included.
    let lower = comment.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(offset) = lower[from..].find("noqa") {
        let start = from + offset;
        let end = start + "noqa".len();
        let starts_a_word = !lower[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if starts_a_word && let Some(list) = comment[end..].strip_prefix(':') {
            let codes: HashSet<String> = list
                .split([',', ' ', '\t', '\r', '\n'])
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_owned)
                .collect();
            // `# noqa:` naming nothing is still a directive, and the only
            // reading left is the one the bare form has.
            return Some(if codes.is_empty() {
                std::iter::once("*".to_owned()).collect()
            } else {
                codes
            });
        }
        from = end;
    }
    None
}

/// Extract file-wide diagnostic suppression from top-of-file
/// directives.
///
/// Scans leading comment / blank lines for
/// `# tcl-lsp: disable=CODE1,CODE2` (or `=*` for "all codes");
/// stops at the first line that is neither blank nor a `#`
/// comment. Multiple directives accumulate into the returned set.
///
/// Codes are split on commas and any whitespace. Empty tokens
/// after splitting are discarded. The keyword `tcl-lsp:disable=`
/// matches case-insensitively, but the codes themselves are not
/// lowercased.
#[must_use]
pub fn parse_file_suppression(source: &str) -> HashSet<String> {
    let mut codes: HashSet<String> = HashSet::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some(rest) = parse_disable_directive(line) else {
            continue;
        };
        for token in rest.split([',', ' ', '\t', '\r', '\n']) {
            let token = token.trim();
            if !token.is_empty() {
                codes.insert(token.to_string());
            }
        }
    }
    codes
}

/// Extract §5.4 version-target declarations from top-of-file
/// ``# tcl-lsp: supports NAME RANGE…`` directives (centralisation ruling
/// R6 — the resolver-invisible targets declaration).
///
/// Same scan window as [`parse_file_suppression`]: leading comment /
/// blank lines only, capped at [`FILE_DIRECTIVE_SCAN_LINES`]. `NAME` is
/// the provider — `tcl` for the core, otherwise a package name (`Tk`,
/// `struct`) — and `RANGE` is one or more whitespace-separated clauses in
/// the targets grammar (`8.5-9.0`, `8.5-`, a bare `8.5` naming that
/// release line only). Returns `(provider, clauses)` pairs in file
/// order; a later directive for the same provider replaces the earlier
/// one, and clause validation happens at resolution, not here.
#[must_use]
pub fn parse_supports_directives(source: &str) -> Vec<(String, String)> {
    let mut declarations: Vec<(String, String)> = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some((name, clauses)) = parse_supports_directive(line) else {
            continue;
        };
        declarations.retain(|(existing, _)| existing != name);
        declarations.push((name.to_owned(), clauses.to_owned()));
    }
    declarations
}

/// Match ``# tcl-lsp: supports NAME RANGE…`` (case-insensitive on the
/// keywords), returning the provider name and the raw clause text.
/// `None` when the line is not that directive — including a `supports`
/// with no range, which declares nothing.
fn parse_supports_directive(line: &str) -> Option<(&str, &str)> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if !s.get(..kw_end)?.eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let supports_kw = "supports";
    let sup_end = supports_kw.len();
    if !s.get(..sup_end)?.eq_ignore_ascii_case(supports_kw) {
        return None;
    }
    let rest = s.get(sup_end..)?;
    // The keyword must end at a word boundary (`supportsx` is not it).
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim();
    let (name, clauses) = rest.split_once([' ', '\t'])?;
    let clauses = clauses.trim();
    (!name.is_empty() && !clauses.is_empty()).then_some((name, clauses))
}

/// Scan *source* for ``# noqa`` comment lines and record next-line
/// suppressions.
///
/// For each ``# noqa`` / ``# noqa: CODE,…`` comment on line *N*
/// (0-based), records the suppression codes against line *N+1*.
/// This covers two cases the command-attached
/// `apply_preceding_noqa` mechanism cannot reach:
///
/// * A ``# noqa`` comment at the tail of a brace body (orphaned —
///   no following command in that scope), where the diagnostic
///   fires on the immediately following ``} elseif …`` or similar
///   line.
/// * A ``# noqa`` comment placed immediately before another comment
///   line that itself generates a diagnostic (e.g. W115 on a
///   ``\\``-continued comment).
///
/// The scan is additive — callers are expected to merge the
/// returned map into ``result.suppressed_lines`` alongside the
/// existing command-level (`apply_preceding_noqa`) and file-level
/// (`parse_file_suppression`) passes.
///
/// [`parse_noqa_marker`] decides what each comment carries: bare
/// ``# noqa`` is recorded with the `"*"` sentinel the suppression
/// filter treats as "every code", a ``noqa:`` marker with the codes
/// it names, and prose that merely mentions the word with nothing.
#[cfg(test)]
#[must_use]
pub fn parse_noqa_line_suppressions(
    source: &str,
) -> std::collections::HashMap<i32, HashSet<String>> {
    parse_noqa_line_suppressions_for_dialect(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
}

/// Dialect-aware [`parse_noqa_line_suppressions`].
#[must_use]
pub fn parse_noqa_line_suppressions_for_dialect(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> std::collections::HashMap<i32, HashSet<String>> {
    let generation = crate::environment_ingress::context_for_profile(dialect);
    parse_noqa_line_suppressions_with_registry(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        generation.commands(),
    )
}

fn parse_noqa_line_suppressions_with_registry(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> std::collections::HashMap<i32, HashSet<String>> {
    noqa_lines_from_comments(script_comment_facts(source, config, registry))
}

/// Next-line suppression facts from the same complete retained analysis.
/// Missing source/config/context correspondence remains unavailable; source
/// directives cannot reconstruct a nominal Registry or entered body.
#[must_use]
pub fn parse_noqa_line_suppressions_from_analysis(
    source: &str,
    analysis: &super::super::AnalysisResult,
) -> Option<std::collections::HashMap<i32, HashSet<String>>> {
    let comments = script_comment_facts_from_analysis(source, analysis)?;
    Some(noqa_lines_from_comments(comments))
}

fn noqa_lines_from_comments(
    comments: impl IntoIterator<Item = ScriptCommentFact>,
) -> std::collections::HashMap<i32, HashSet<String>> {
    let mut result: std::collections::HashMap<i32, HashSet<String>> =
        std::collections::HashMap::new();
    for fact in comments {
        let Some(codes) = parse_noqa_marker(&fact.text) else {
            continue;
        };
        let next_line = i32::try_from(fact.line)
            .unwrap_or(i32::MAX)
            .saturating_add(1);
        result.entry(next_line).or_default().extend(codes);
    }
    result
}

/// Match a ``# tcl-lsp: <marker>`` line, where `marker` is
/// e.g. ``"stubs-begin"`` / ``"stubs-end"``.  Case-insensitive
/// on the keyword run; whitespace flexible.
pub(super) fn matches_marker(body: &str, marker: &str) -> bool {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `str::get` returns `None` when `kw_end` is past the end *or* falls inside
    // a multi-byte char, so it subsumes the length check and never panics on
    // non-ASCII source (the marker run itself is pure ASCII).
    let Some(prefix) = s.get(..kw_end) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return false;
    }
    let rest = s[kw_end..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let m_end = marker.len();
    let Some(head) = rest.get(..m_end) else {
        return false;
    };
    head.eq_ignore_ascii_case(marker)
}

/// Scan `source` for ``# tcl-lsp: package NAME provides PKG …`` directives,
/// returning each declared edge as `(loading package, packages it also
/// loads)` in source order.
///
/// The directive states what a package's loader pulls in behind the
/// analyser's back — a binary extension whose C `Init` calls
/// `Tcl_PkgRequire` or `Tk_InitStubs`. It declares the same
/// edge `[packages.provides]` configures, for one file, and it names the
/// loading package rather than relying on where it sits: a document gains
/// the packages only once it actually requires the one named, and the
/// directive may live anywhere, including the top-of-file comment block.
///
/// `package` is the keyword, as in every other member of the family
/// (`supports`, `stub`, `disable`), and it deliberately echoes the Tcl
/// commands the line is about.
///
/// The scan covers the whole file on the same terms as
/// [`scan_source_for_stubs`] — any line whose first non-whitespace character
/// is `#`. One directive may name several loaded packages, and several
/// directives may name the same loader; the edges merge, keeping
/// first-declaration order.
#[must_use]
pub fn parse_provides_directives(source: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in source.lines() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        if !matches_marker(body, "package") {
            continue;
        }
        let Some((from, loaded)) = parse_provides_directive(body) else {
            continue;
        };
        let position = out.iter().position(|(existing, _)| existing == from);
        let index = position.unwrap_or_else(|| {
            out.push((from.to_owned(), Vec::new()));
            out.len() - 1
        });
        for name in loaded.split_whitespace() {
            if !out[index].1.iter().any(|existing| existing == name) {
                out[index].1.push(name.to_owned());
            }
        }
    }
    out.retain(|(_, loaded)| !loaded.is_empty());
    out
}

/// Match ``# tcl-lsp: package NAME provides PKG …`` on a comment body whose
/// `package` marker [`matches_marker`] already confirmed, returning the
/// loading package and the raw list of packages it loads.
///
/// `None` when the line is not that shape — a `package` with no name, no
/// `provides`, or nothing after it declares no edge.
fn parse_provides_directive(body: &str) -> Option<(&str, &str)> {
    let s = body.trim_start();
    let rest = s
        .get("tcl-lsp".len()..)?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let after_kw = rest.get("package".len()..)?;
    // The keyword must end at a word boundary (`packages` is not it).
    if !after_kw.starts_with([' ', '\t']) {
        return None;
    }
    let (from, rest) = after_kw.trim_start().split_once([' ', '\t'])?;
    let rest = rest.trim_start();
    let verb = "provides";
    if !rest.get(..verb.len())?.eq_ignore_ascii_case(verb) {
        return None;
    }
    let loaded = rest.get(verb.len()..)?;
    if !loaded.starts_with([' ', '\t']) {
        return None;
    }
    let loaded = loaded.trim();
    (!from.is_empty() && !loaded.is_empty()).then_some((from, loaded))
}

/// Per-line ``# noqa: CODE`` suppression scanner.
///
/// For each ``# noqa`` (with or without a ``: CODE`` list),
/// records the codes against the 0-based line range read from the
/// ``preceding_comment`` field of the segmented command. The
/// segmented-command dispatch loop calls this on each command to
/// attach the noqa codes to the *following* command's line range.
///
/// The directive shapes come from [`parse_noqa_marker`]: bare
/// ``# noqa`` (no ``: CODE`` list) suppresses every code (`"*"`
/// sentinel); ``# noqa: A, B`` suppresses the named codes. The
/// text searched is the segmenter's ``preceding_comment`` field,
/// which carries comment text only, so a ``#`` inside a Tcl
/// string is never read as a directive.
pub fn apply_preceding_noqa<S, T>(
    cmd: &crate::segmenter::SegmentedCommand,
    line_offsets: &[usize],
    suppressed_lines: &mut std::collections::HashMap<i32, std::collections::HashSet<String, T>, S>,
) where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher + Default,
{
    let Some(comment) = cmd.preceding_comment.as_deref() else {
        return;
    };
    let Some(codes) = parse_noqa_marker(comment) else {
        return;
    };
    // Attribute to every line spanned by the command.
    // ``SegmentedCommand.span`` is byte offsets; convert each via
    // the precomputed ``line_offsets`` index in ``O(log N)``
    // instead of a linear scan per call (the helper runs once per
    // segmented command).
    let span_start = cmd.span.start() as usize;
    let span_end = cmd.span.end() as usize;
    let start_line = super::super::state::line_at_offset(line_offsets, span_start);
    let end_line = super::super::state::line_at_offset(line_offsets, span_end);
    for line in start_line..=end_line {
        suppressed_lines
            .entry(line)
            .or_default()
            .extend(codes.iter().cloned());
    }
}

/// Match `# tcl-lsp: disable=…` (case-insensitive on the keyword),
/// returning the trailing CODE list as a borrowed slice. Returns
/// `None` if the line doesn't match the directive shape.
///
/// Uses [`str::get`] rather than byte-index slicing for the keyword
/// checks: a fixed ASCII-length prefix like `s[..kw_end]` panics when a
/// multi-byte UTF-8 character (an em dash, accented letter, …) straddles
/// that byte offset — `s.get(..kw_end)` instead returns `None` for a
/// non-boundary offset exactly as it does for a too-short `s`, so a
/// leading comment with early non-ASCII text is simply "not a directive"
/// rather than a panic.
fn parse_disable_directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if !s.get(..kw_end)?.eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let disable_kw = "disable";
    let dis_end = disable_kw.len();
    if !s.get(..dis_end)?.eq_ignore_ascii_case(disable_kw) {
        return None;
    }
    s = s[dis_end..].trim_start();
    let s = s.strip_prefix('=')?.trim();
    Some(s)
}
