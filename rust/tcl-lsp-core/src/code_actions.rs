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

//! Code-actions provider.
//!
//! Surfaces every `CodeFix` the analyser attached to a
//! `Diagnostic` whose span overlaps the requested range.  Each
//! fix lifts to one `CodeAction` with the fix's `description`
//! as the title and a single-edit `WorkspaceEdit` carrying
//! the fix's `(span, new_text)`.
//!
//! Provided actions:
//!
//! * Catch-result-variable actions — when the analyser emits
//!   W302 (`catch` without result variable), the provider
//!   offers two quick-fixes that splice a trailing ` result`
//!   or ` result opts` after the body's closing brace.
//! * `unset -nocomplain` action — W213 (unset on possibly-undefined
//!   variable) carries an `Add '-nocomplain' to unset` insert `CodeFix`
//!   (the analyser knows the exact keyword span); the provider lifts it via
//!   the generic `diag.fixes` path, like W120 below.
//! * `Add 'package require <pkg>'` action — the analyser emits
//!   W120 (package-gated command without `package require`)
//!   carrying an insert `CodeFix`; the provider lifts it via
//!   the generic `diag.fixes` path below.
//!
//! * Package-*suggestion* actions ([`package_require_actions`]) —
//!   for the word at the cursor, fuzzy-rank known package names
//!   (the registry's `required_package` / `tcllib_package`
//!   catalogue) against the `::`-prefix and offer
//!   `Add 'package require <pkg>'` (skipping already-required
//!   packages).
//!
//! Limitations:
//!
//! * [`package_require_actions`] derives its catalogue from
//!   the registry, so locally-installed-but-unregistered
//!   packages aren't suggested.
//! * Cross-document refactors (move to file, split namespace)
//!   are not supported.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lexer::{LineIndex, Utf16Col};

use crate::definition::{LspRange, utf16_col_to_char_col};

/// LSP code-action kind.  Maps to the dotted strings the editor / e2e
/// `only` filter use (`quickfix`, `refactor.extract`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// `quickfix` — a diagnostic fix.
    QuickFix,
    /// `refactor.extract` — extract proc.
    RefactorExtract,
    /// `refactor.inline` — inline proc.
    RefactorInline,
    /// `refactor.rewrite` — expression rewrites (De Morgan, invert).
    RefactorRewrite,
    /// `refactor` — generic refactor (IP conversion).
    Refactor,
    /// `source` — source action (generate docstring).
    Source,
}

impl ActionKind {
    /// The dotted LSP kind string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QuickFix => "quickfix",
            Self::RefactorExtract => "refactor.extract",
            Self::RefactorInline => "refactor.inline",
            Self::RefactorRewrite => "refactor.rewrite",
            Self::Refactor => "refactor",
            Self::Source => "source",
        }
    }
}

/// A command attached to a code action (e.g. the post-extract rename).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCommand {
    /// Command identifier (e.g. `tclLsp.renameSymbolAtPosition`).
    pub command: String,
    /// Integer arguments (line / start / end for the rename command).
    pub args: Vec<u32>,
    /// String arguments — used by the BIG-IP actions whose command takes
    /// textual arguments rather than integer positions: the document `uri`
    /// (plus a bare partition name for `tclLsp.renamePartition`, or `uri`
    /// alone for `editor.action.rename`).  Empty for the integer-position
    /// commands.
    pub string_args: Vec<String>,
}

/// One code-action entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    /// Title shown in the editor.
    pub title: String,
    /// Edits the action would apply.
    pub edits: Vec<crate::rename::TextEdit>,
    /// LSP kind (drives the editor's `only` filter).
    pub kind: ActionKind,
    /// Optional command run after the edit (e.g. trigger a rename).
    pub command: Option<ActionCommand>,
    /// Optional structured payload surfaced as the LSP code action's
    /// `data` field.  Currently carries the rendered tmsh `ltm
    /// data-group internal …` definition for the extract-to-datagroup
    /// refactor; the iRule text rewrite is the action's `edits`, and this field
    /// lets tooling (MCP, AI, clipboard) consume the data-group
    /// definition without injecting comment blocks into the source.
    pub data_group_definition: Option<String>,
}

impl CodeAction {
    /// Construct a `CodeAction` with no `data_group_definition`.
    ///
    /// The common path: every action except the extract-to-datagroup
    /// refactor leaves the structured payload unset, so this keeps the
    /// call sites free of a `data_group_definition: None` field.
    #[must_use]
    pub fn new(
        title: String,
        edits: Vec<crate::rename::TextEdit>,
        kind: ActionKind,
        command: Option<ActionCommand>,
    ) -> Self {
        Self {
            title,
            edits,
            kind,
            command,
            data_group_definition: None,
        }
    }
}

/// Lift every [`tcl_compiler::analyser::CodeFix`] carried by a
/// diagnostic into a quick-fix [`CodeAction`] — one action per fix,
/// with the fix's own `(span, new_text)` as a single-edit workspace
/// edit.  The title is the fix's `description`, falling back to the
/// (truncated) diagnostic message when the emitter supplied none.
///
/// The analyser (`AnalysisResult.diagnostics`) and the compiler-checks
/// pass (`run_all_checks`) share one `CodeFix` type, so both
/// fixes-bearing diagnostic families lift through this helper.
fn lift_fixes(
    actions: &mut Vec<CodeAction>,
    fixes: &[tcl_compiler::analyser::CodeFix],
    diag_message: &str,
    source: &str,
    line_index: &LineIndex,
) {
    for fix in fixes {
        let fix_start = line_index.position_at_utf16(fix.span.start(), source);
        let fix_end = line_index.position_at_utf16(fix.span.end(), source);
        let title = if fix.description.is_empty() {
            // Fall back to the diagnostic's message (truncated) when
            // the fix didn't carry a description.
            let trimmed: String = diag_message.chars().take(60).collect();
            format!("Fix: {trimmed}")
        } else {
            fix.description.clone()
        };
        actions.push(CodeAction {
            title,
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: fix_start.line,
                    start_character: fix_start.character.get(),
                    end_line: fix_end.line,
                    end_character: fix_end.character.get(),
                },
                new_text: fix.new_text.clone(),
            }],
            kind: ActionKind::QuickFix,
            command: None,
            data_group_definition: None,
        });
    }
}

/// `refactor.rewrite` — "Brace expr for safety and performance".  Offered
/// whenever the request range touches a line carrying an unbraced-expr (W100)
/// diagnostic, which corresponds to the `expr` command at the cursor.
/// Keyed on *line* overlap rather than the
/// diagnostic's argument span so it is available with the cursor on the `expr`
/// keyword itself (VS Code invokes refactors at the caret, e.g. column 0), not
/// only over the arguments.  Reuses the diagnostic's own brace-wrapping fix, so
/// `expr $a + $b` rewrites to `expr {$a + $b}`.
fn push_brace_expr_refactors(
    actions: &mut Vec<CodeAction>,
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) {
    for diag in &analysis.diagnostics {
        if diag.code != DiagCode::W100 {
            continue;
        }
        let Some(fix) = diag.fixes.first() else {
            continue;
        };
        let fix_start = line_index.position_at_utf16(fix.span.start(), source);
        let fix_end = line_index.position_at_utf16(fix.span.end(), source);
        if range.start_line > fix_end.line || range.end_line < fix_start.line {
            continue;
        }
        actions.push(CodeAction {
            title: "Brace expr for safety and performance".to_string(),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: fix_start.line,
                    start_character: fix_start.character.get(),
                    end_line: fix_end.line,
                    end_character: fix_end.character.get(),
                },
                new_text: fix.new_text.clone(),
            }],
            kind: ActionKind::RefactorRewrite,
            command: None,
            data_group_definition: None,
        });
    }
}

/// Compute code actions for `range` in `source`.
///
/// `analysis`, when `Some`, is the analyser result the caller
/// already computed.  When `None`, returns an empty vector
/// (preserves the stub call shape for callers that haven't
/// yet plumbed analysis through).
#[must_use]
pub fn code_actions(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
) -> Vec<CodeAction> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();

    push_brace_expr_refactors(&mut actions, source, range, analysis, &line_index);

    for diag in &analysis.diagnostics {
        let diag_start = line_index.position_at_utf16(diag.span.start(), source);
        let diag_end = line_index.position_at_utf16(diag.span.end(), source);
        let diag_range = LspRange {
            start_line: diag_start.line,
            start_character: diag_start.character.get(),
            end_line: diag_end.line,
            end_character: diag_end.character.get(),
        };
        if !ranges_overlap(diag_range, range) {
            continue;
        }
        // Surface synthetic
        // catch-result-variable actions for W302 diagnostics
        // even when the analyser didn't attach a `CodeFix`.
        // Two actions: append ` result` (capture the result)
        // or ` result opts` (capture result + options).  The
        // diagnostic's span end sits past the body's closing
        // `}`, so the insertion point is exactly the diag-end
        // position.
        if diag.code == DiagCode::W302 {
            let insertion = LspRange {
                start_line: diag_end.line,
                start_character: diag_end.character.get(),
                end_line: diag_end.line,
                end_character: diag_end.character.get(),
            };
            for (title, suffix) in [
                ("Add catch result variable", " result"),
                ("Add catch result + options variables", " result opts"),
            ] {
                actions.push(CodeAction {
                    title: title.to_string(),
                    edits: vec![crate::rename::TextEdit {
                        range: insertion,
                        new_text: suffix.to_string(),
                    }],
                    kind: ActionKind::QuickFix,
                    command: None,
                    data_group_definition: None,
                });
            }
        }
        // W213's `Add '-nocomplain' to unset` quick-fix is carried on the
        // diagnostic itself (the analyser knows the exact `unset` keyword span
        // and narrows the diagnostic to the offending variable word), so it is
        // surfaced by the generic `lift_fixes` path rather than re-derived
        // here from the span.
        lift_fixes(
            &mut actions,
            &diag.fixes,
            &diag.message,
            source,
            &line_index,
        );
    }

    // Range-based refactors / source actions that don't depend on a diagnostic.
    actions.extend(continuation_comment_actions(
        source,
        range,
        analysis,
        &line_index,
    ));
    actions.extend(ip_conversion_actions(source, range, &line_index));
    actions.extend(expr_rewrite_actions(source, range, &line_index));
    actions.extend(docstring_actions(source, range, analysis, &line_index));
    actions.extend(extract_inline_actions(source, range, analysis, &line_index));

    actions
}

/// BIG-IP-specific code actions for the cursor at `range`'s start.
///
/// A BIG-IP `.conf` is a tree of `module object-type identifier
/// { … }` stanzas (NOT Tcl), so this drives the [`crate::bigip`] stanza
/// parser rather than the Tcl analyser, walks for the stanza whose range
/// covers the cursor line, and emits:
///
/// * **`Rename <full-path>…`** ([`ActionKind::RefactorRewrite`]) for the
///   covering object — a [`ActionCommand`] pointing at the editor's
///   standard `editor.action.rename` flow (its `string_args` carry the
///   document `uri`), so the existing rename UI collects the new name; no
///   pre-baked edit.
/// * **`Rename partition '<name>'…`** when the covering stanza is an
///   `auth partition` — a `tclLsp.renamePartition` command whose
///   `string_args` are `[uri, <bare-partition-name>]`, so the cascade
///   flows through the query engine on accept.  Renames of `/Common` are
///   suppressed (the query engine refuses them — the F5
///   partition-visibility model).
///
/// Returns an empty vector when the cursor is not inside a parseable
/// stanza or the document is not BIG-IP.  `range`'s start *line* selects
/// the object (keyed on `range.start.line`).
#[must_use]
pub fn bigip_code_actions(source: &str, range: LspRange, uri: &str) -> Vec<CodeAction> {
    let cursor_line = range.start_line;
    let stanzas = crate::bigip::parse_stanzas(source);

    // The object whose stanza covers the cursor line — first match in
    // source order.  A
    // nameless singleton (empty identifier) has no path to rename, so it
    // is not a rename target.
    let Some(stanza) = stanzas.iter().find(|s| {
        !s.identifier.is_empty()
            && s.range.start_line <= cursor_line
            && cursor_line <= s.range.end_line
    }) else {
        return Vec::new();
    };
    let obj_path = stanza.identifier.as_str();

    let mut actions = Vec::new();

    // Rename-this-object — routes through the editor's standard rename
    // UI (`editor.action.rename`); no pre-baked workspace edit, so the
    // user supplies the real name.
    actions.push(CodeAction {
        title: format!("Rename {obj_path}\u{2026}"),
        edits: Vec::new(),
        kind: ActionKind::RefactorRewrite,
        command: Some(ActionCommand {
            command: "editor.action.rename".to_string(),
            args: Vec::new(),
            string_args: vec![uri.to_string()],
        }),
        data_group_definition: None,
    });

    // Partition rename — only on an `auth partition` stanza, and never
    // for `/Common` (the query engine refuses that rename).  The bare
    // partition name is the identifier with any leading slash stripped.
    if stanza.module == "auth" && stanza.object_type == "partition" {
        let partition_short = obj_path.trim_start_matches('/');
        if partition_short != "Common" {
            actions.push(CodeAction {
                title: format!("Rename partition '{partition_short}'\u{2026}"),
                edits: Vec::new(),
                kind: ActionKind::RefactorRewrite,
                command: Some(ActionCommand {
                    command: "tclLsp.renamePartition".to_string(),
                    args: Vec::new(),
                    string_args: vec![uri.to_string(), partition_short.to_string()],
                }),
                data_group_definition: None,
            });
        }
    }

    actions
}

/// Lift the quick-fixes carried by **compiler-check** diagnostics whose span
/// overlaps `range` into `CodeAction`s, plus the synthetic shimmer-family
/// "Suppress" action (see [`build_shimmer_noqa_suppress_action`]).
///
/// The analyser-driven [`code_actions`] above only sees
/// `AnalysisResult.diagnostics`; the compiler checks surfaced through
/// `run_all_checks` are a disjoint set, so lifting their fixes here carries no
/// risk of double-offering an analyser fix.  Several check constructors
/// populate `fixes` (the iRules control-flow insertions, taint-family
/// rewrites, and the W201 `file join` rewrite among them) and new ones may
/// join — this lift is generic over whatever the checks carry, never a
/// per-constructor special case.
///
/// The caller passes the `run_all_checks` output
/// (e.g. `CompilerDiagnostics::checks`).
///
/// `disabled` is the resolved per-check toggle set
/// (`tclLsp.diagnostics.<CODE> = false`).  A check whose code is disabled has
/// its diagnostic suppressed from the published set, so its quick-fix must not
/// be offered either — otherwise the lightbulb would re-surface a hidden
/// warning.  The analyser path bakes this set into its build; this path is fed
/// the raw `run_all_checks` output, so it applies the same filter here.
#[must_use]
pub fn check_diagnostic_actions<S: std::hash::BuildHasher>(
    source: &str,
    range: LspRange,
    checks: &[tcl_compiler::compiler_checks::Diagnostic],
    disabled: &std::collections::HashSet<String, S>,
) -> Vec<CodeAction> {
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();
    for diag in checks {
        if disabled.contains(diag.code.as_str()) {
            continue;
        }
        let diag_start = line_index.position_at_utf16(diag.span.start(), source);
        let diag_end = line_index.position_at_utf16(diag.span.end(), source);
        let diag_range = LspRange {
            start_line: diag_start.line,
            start_character: diag_start.character.get(),
            end_line: diag_end.line,
            end_character: diag_end.character.get(),
        };
        if !ranges_overlap(diag_range, range) {
            continue;
        }
        lift_fixes(
            &mut actions,
            &diag.fixes,
            &diag.message,
            source,
            &line_index,
        );
        if is_shimmer_family(diag.code)
            && let Some(action) = build_shimmer_noqa_suppress_action(source, diag, &line_index)
        {
            actions.push(action);
        }
    }
    actions
}

/// True for the shimmer diagnostic family (S100/S101/S102 — performance
/// intrep-conversion; S103 — shared-value copy-on-write; S110 —
/// byte-array-corruption correctness), the set
/// [`build_shimmer_noqa_suppress_action`] offers a suppression fix for.
fn is_shimmer_family(code: DiagCode) -> bool {
    matches!(
        code,
        DiagCode::S100 | DiagCode::S101 | DiagCode::S102 | DiagCode::S103 | DiagCode::S110
    )
}

/// Build a `# noqa: <CODE>` suppression quick-fix for a shimmer-family
/// diagnostic.
///
/// Unlike the semantic fixes above (a mechanical rewrite the analyser is
/// confident preserves behaviour), there is no generally-safe *automatic*
/// rewrite for a shimmer: the KCS-documented fix is to use a separate
/// variable for the numeric/string use, which requires picking a name and
/// judging the surrounding code — not something to apply unattended. The
/// mechanical, always-safe action every diagnostic family in this project
/// supports is the inline suppression directive (see
/// `docs/kcs/kcs-howto-suppress-diagnostics.md`): `# noqa: CODE` on the line
/// **before** the command. This inserts that line, indented to match the
/// command's own line, immediately above it.
///
/// Returns `None` when the diagnostic's start offset doesn't resolve to a
/// source line (defensive; `LineIndex` is built from the same `source`).
fn build_shimmer_noqa_suppress_action(
    source: &str,
    diag: &tcl_compiler::compiler_checks::Diagnostic,
    line_index: &LineIndex,
) -> Option<CodeAction> {
    let line = line_index.line_at(diag.span.start());
    let line_start = line_index.line_start(line);
    let line_text = source.get(line_start as usize..)?.lines().next()?;
    let indent: String = line_text
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let pos = line_index.position_at_utf16(line_start, source);
    let insertion = LspRange {
        start_line: pos.line,
        start_character: pos.character.get(),
        end_line: pos.line,
        end_character: pos.character.get(),
    };
    Some(CodeAction {
        title: format!("Suppress {} with a noqa comment", diag.code.as_str()),
        edits: vec![crate::rename::TextEdit {
            range: insertion,
            new_text: format!("{indent}# noqa: {}\n", diag.code.as_str()),
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    })
}

/// `true` when `a` and `b` overlap (touch, intersect, or are
/// identical).  Mirrors VS Code's range-context filter for
/// code actions.
fn ranges_overlap(a: LspRange, b: LspRange) -> bool {
    // Convert each range to a (start, end) tuple of
    // (line, character) for ordering.
    let a_start = (a.start_line, a.start_character);
    let a_end = (a.end_line, a.end_character);
    let b_start = (b.start_line, b.start_character);
    let b_end = (b.end_line, b.end_character);
    a_start <= b_end && b_start <= a_end
}

/// Fuzzy `package require` suggestions for the word at `range`'s start:
/// when an unknown command's prefix (the part before `::`) fuzzy-matches
/// a known package name, offer `Add 'package require <pkg>'`.  The package
/// catalogue is derived from the registry's `required_package` /
/// `tcllib_package` fields, so locally-installed-but-unregistered packages
/// aren't suggested.
#[must_use]
pub fn package_require_actions(
    source: &str,
    range: LspRange,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<CodeAction> {
    let word = word_at_position(source, range.start_line, range.start_character);
    let prefix = word.split("::").next().unwrap_or("").to_lowercase();
    if prefix.len() < 2 {
        return Vec::new();
    }
    let catalogue = package_catalogue(registry);
    let ranked = rank_package_suggestions(&word, &catalogue, 5);
    let insert_line = package_insert_line(source);
    ranked
        .into_iter()
        .filter(|pkg| !already_required(source, pkg))
        .map(|pkg| CodeAction {
            title: format!("Add 'package require {pkg}'"),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: insert_line,
                    start_character: 0,
                    end_line: insert_line,
                    end_character: 0,
                },
                new_text: format!("package require {pkg}\n"),
            }],
            kind: ActionKind::QuickFix,
            command: None,
            data_group_definition: None,
        })
        .collect()
}

/// Distinct package names known to the registry (`required_package` +
/// `tcllib_package` across all command specs).
fn package_catalogue(registry: &tcl_registry::CommandRegistry) -> Vec<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for name in registry.command_names() {
        if let Some(spec) = registry.get(name) {
            if let Some(pkg) = spec.required_package {
                set.insert(pkg.to_owned());
            }
            if let Some(pkg) = spec.tcllib_package {
                set.insert(pkg.to_owned());
            }
        }
    }
    set.into_iter().collect()
}

/// Rank package names against a symbol's prefix (exact / prefix /
/// substring), best first, capped at `limit`.  The ranking core is
/// [`tcl_compiler::text::rank_containment_suggestions`]; this wrapper
/// only extracts the pre-`::` prefix and applies the two-character
/// minimum.
fn rank_package_suggestions(symbol: &str, packages: &[String], limit: usize) -> Vec<String> {
    let prefix = symbol
        .trim()
        .split("::")
        .next()
        .unwrap_or("")
        .to_lowercase();
    if prefix.len() < 2 {
        return Vec::new();
    }
    tcl_compiler::text::rank_containment_suggestions(
        &prefix,
        packages.iter().map(String::as_str),
        limit,
    )
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Line at which to insert a new `package require` — after a leading
/// shebang and any contiguous top-of-file `package require` lines.
fn package_insert_line(source: &str) -> u32 {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut line = 0usize;
    if lines.first().is_some_and(|l| l.starts_with("#!")) {
        line = 1;
    }
    while line < lines.len() && {
        let t = lines[line].trim_start();
        t.starts_with("package require") || t.starts_with("package\trequire")
    } {
        line += 1;
    }
    u32::try_from(line).unwrap_or(0)
}

/// `true` when `source` already `package require`s `pkg`.
fn already_required(source: &str, pkg: &str) -> bool {
    source.split('\n').any(|line| {
        let t = line.trim_start();
        if let Some(rest) = t
            .strip_prefix("package require")
            .or_else(|| t.strip_prefix("package\trequire"))
        {
            let rest = rest.trim_start();
            rest.strip_prefix(pkg)
                .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
        } else {
            false
        }
    })
}

/// Extract the identifier word (including `::`) at `(line, character)`.
fn word_at_position(source: &str, line: u32, character: u32) -> String {
    let Some(line_text) = source.split('\n').nth(line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == ':';
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut start = col;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

// W115 — convert a backslash-continued comment to per-line comments.

fn continuation_comment_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    // Only offer when a W115 diagnostic overlaps the range OR the start line is
    // a backslash-continued comment (the editor may drive this with a
    // fabricated W115 it computed client-side, so detect the shape directly).
    let lines: Vec<&str> = source.split('\n').collect();
    let start_line = range.start_line as usize;
    if start_line >= lines.len() {
        return Vec::new();
    }
    let first = lines[start_line];
    let trimmed = first.trim_start();
    if !trimmed.starts_with('#') || !first.trim_end().ends_with('\\') {
        // Not a continued comment at the range start — only offer the fix when
        // a W115 diagnostic actually *overlaps* the requested range. A
        // file-wide "any W115" check would rewrite an unrelated
        // backslash-continued command on a different line as commented text.
        let has_overlapping_w115 = analysis.diagnostics.iter().any(|d| {
            if d.code != DiagCode::W115 {
                return false;
            }
            let start = line_index.position_at_utf16(d.span.start(), source);
            let end = line_index.position_at_utf16(d.span.end(), source);
            ranges_overlap(
                LspRange {
                    start_line: start.line,
                    start_character: start.character.get(),
                    end_line: end.line,
                    end_character: end.character.get(),
                },
                range,
            )
        });
        if !has_overlapping_w115 {
            return Vec::new();
        }
    }
    // Gather the continuation run starting at `start_line`.
    let mut idx = start_line;
    let mut block: Vec<String> = Vec::new();
    loop {
        if idx >= lines.len() {
            break;
        }
        let line = lines[idx];
        let stripped = line.trim_end();
        let continues = stripped.ends_with('\\');
        // Strip the trailing backslash; preserve leading indentation.
        let body = if continues {
            stripped[..stripped.len() - 1].trim_end()
        } else {
            line.trim_end()
        };
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let content = body.trim_start();
        if content.starts_with('#') {
            block.push(format!("{indent}{content}"));
        } else if content.is_empty() {
            block.push(indent);
        } else {
            block.push(format!("{indent}# {content}"));
        }
        if !continues {
            break;
        }
        idx += 1;
    }
    if idx <= start_line {
        return Vec::new();
    }
    let new_text = block.join("\n");
    vec![CodeAction {
        title: "Convert to per-line comments".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: range.start_line,
                start_character: 0,
                end_line: u32::try_from(idx).unwrap_or(range.start_line),
                // LSP columns are UTF-16 code units — use the line's UTF-16
                // length, not its codepoint count.
                end_character: char_col_to_utf16_local(lines[idx], lines[idx].chars().count()),
            },
            new_text,
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    }]
}

// IPv4 ↔ IPv6-mapped conversion.

/// `true` when `s` is a dotted-quad IPv4 literal (each octet 0-255).
fn is_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.parse::<u8>().is_ok())
}

fn ip_conversion_actions(
    source: &str,
    range: LspRange,
    _line_index: &LineIndex,
) -> Vec<CodeAction> {
    let Some(line_text) = source.split('\n').nth(range.start_line as usize) else {
        return Vec::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, range.start_character).min(chars.len());
    // IP-literal characters include hex, `.`, `:`, and `/` for the CIDR suffix.
    let is_ip_char = |c: char| c.is_ascii_hexdigit() || matches!(c, '.' | ':' | '/');
    let mut start = col;
    while start > 0 && is_ip_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ip_char(chars[end]) {
        end += 1;
    }
    if start >= end {
        return Vec::new();
    }
    let word: String = chars[start..end].iter().collect();
    let (addr, suffix) = match word.split_once('/') {
        Some((a, s)) => (a.to_string(), format!("/{s}")),
        None => (word.clone(), String::new()),
    };
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: char_col_to_utf16_local(line_text, start),
        end_line: range.start_line,
        end_character: char_col_to_utf16_local(line_text, end),
    };
    let make = |title: String, new_addr: String| CodeAction {
        title,
        edits: vec![crate::rename::TextEdit {
            range: edit_range,
            new_text: format!("{new_addr}{suffix}"),
        }],
        kind: ActionKind::Refactor,
        command: None,
        data_group_definition: None,
    };
    if is_ipv4(&addr) {
        return vec![make(
            "Convert to IPv6-mapped address".to_string(),
            format!("::ffff:{addr}"),
        )];
    }
    if let Some(rest) = addr
        .strip_prefix("::ffff:")
        .or_else(|| addr.strip_prefix("::FFFF:"))
        && is_ipv4(rest)
    {
        return vec![make(
            "Convert to IPv4 address".to_string(),
            rest.to_string(),
        )];
    }
    Vec::new()
}

/// Codepoint column → UTF-16 column on `line_text`.
fn char_col_to_utf16_local(line_text: &str, char_col: usize) -> u32 {
    line_text
        .chars()
        .take(char_col)
        .map(|c| u32::try_from(c.len_utf16()).unwrap_or(1))
        .sum()
}

// Expression rewrites: De Morgan + invert comparison.

fn expr_rewrite_actions(source: &str, range: LspRange, _line_index: &LineIndex) -> Vec<CodeAction> {
    // Single-line, non-empty selection only.
    if range.start_line != range.end_line || range.start_character >= range.end_character {
        return Vec::new();
    }
    let Some(line_text) = source.split('\n').nth(range.start_line as usize) else {
        return Vec::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let s = utf16_col_to_char_col(line_text, range.start_character).min(chars.len());
    let e = utf16_col_to_char_col(line_text, range.end_character).min(chars.len());
    if s >= e {
        return Vec::new();
    }
    let sel: String = chars[s..e].iter().collect();
    let mut out = Vec::new();
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: range.start_character,
        end_line: range.end_line,
        end_character: range.end_character,
    };
    if let Some(rewritten) = demorgan_transform(&sel) {
        out.push(CodeAction {
            title: "Apply De Morgan's law".to_string(),
            edits: vec![crate::rename::TextEdit {
                range: edit_range,
                new_text: rewritten,
            }],
            kind: ActionKind::RefactorRewrite,
            command: None,
            data_group_definition: None,
        });
    }
    if let Some(rewritten) = invert_comparison(&sel) {
        out.push(CodeAction {
            title: "Invert comparison".to_string(),
            edits: vec![crate::rename::TextEdit {
                range: edit_range,
                new_text: rewritten,
            }],
            kind: ActionKind::RefactorRewrite,
            command: None,
            data_group_definition: None,
        });
    }
    out
}

/// De Morgan: `!(X && Y)` ↔ `!X || !Y`, `!(X || Y)` ↔ `!X && !Y` — plus the
/// iRules word-operator equivalents (`not`/`and`/`or`, i.e.
/// `UnaryOp::WordNot`/`BinOp::WordAnd`/`BinOp::WordOr` — issue #983's
/// unification). This used to only recognise the symbolic forms, so it
/// silently never offered the rewrite for a selection written in iRules'
/// word style (`!($a and $b)`) — an inconsistent gap given the sibling
/// `invert_comparison` rewrite in this same file already handles TIP 461's
/// word operators (`lt`/`le`/`gt`/`ge`).
fn demorgan_transform(sel: &str) -> Option<String> {
    let t = sel.trim();
    let word_and = tcl_syntax::expr::ast::BinOp::WordAnd.spec().spelling;
    let word_or = tcl_syntax::expr::ast::BinOp::WordOr.spec().spelling;
    let word_not = tcl_syntax::expr::ast::UnaryOp::WordNot.spec().spelling;

    // Forward: `!( X <op> Y )` or `not ( X <op> Y )`. The outer negation
    // prefix and the inner operator's symbol/word spelling are independent
    // choices in iRules — `!($a and $b)` mixes both — so each outer prefix
    // tries every inner operator spelling, negating operands in the same
    // style as its own prefix.
    if let Some(inner) = t.strip_prefix("!(").and_then(|s| s.strip_suffix(')')) {
        return demorgan_forward_inner(inner, word_and, word_or, negate);
    }
    if let Some(inner) = t
        .strip_prefix(word_not)
        .map(str::trim_start)
        .and_then(|s| s.strip_prefix('('))
        .and_then(|s| s.strip_suffix(')'))
    {
        return demorgan_forward_inner(inner, word_and, word_or, |o| negate_word(o, word_not));
    }
    // Reverse: `!X || !Y` → `!(X && Y)`, `!X && !Y` → `!(X || Y)` — and the
    // word-operator equivalents (`not X or not Y` → `not (X and Y)`, …).
    if let Some((l, r)) = split_top_logical(t, "||")
        && let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!'))
    {
        return Some(format!("!({} && {})", li.trim(), ri.trim()));
    }
    if let Some((l, r)) = split_top_logical(t, "&&")
        && let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!'))
    {
        return Some(format!("!({} || {})", li.trim(), ri.trim()));
    }
    if let Some((l, r)) = split_top_logical_word(t, word_or)
        && let (Some(li), Some(ri)) = (
            strip_word_not(l.trim(), word_not),
            strip_word_not(r.trim(), word_not),
        )
    {
        return Some(format!(
            "{word_not} ({} {word_and} {})",
            li.trim(),
            ri.trim()
        ));
    }
    if let Some((l, r)) = split_top_logical_word(t, word_and)
        && let (Some(li), Some(ri)) = (
            strip_word_not(l.trim(), word_not),
            strip_word_not(r.trim(), word_not),
        )
    {
        return Some(format!(
            "{word_not} ({} {word_or} {})",
            li.trim(),
            ri.trim()
        ));
    }
    None
}

/// The body of forward-direction De Morgan (`negate_op` applies whichever
/// negation spelling matches the outer prefix that was stripped — `!` or
/// `not`), tried against every inner connective spelling (`&&`/`||` and
/// their word-operator equivalents `and`/`or`).
fn demorgan_forward_inner(
    inner: &str,
    word_and: &str,
    word_or: &str,
    negate_op: impl Fn(&str) -> String,
) -> Option<String> {
    if let Some((l, r)) = split_top_logical(inner, "&&") {
        return Some(format!(
            "{} || {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical(inner, "||") {
        return Some(format!(
            "{} && {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical_word(inner, word_and) {
        return Some(format!(
            "{} {word_or} {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical_word(inner, word_or) {
        return Some(format!(
            "{} {word_and} {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    None
}

/// Like [`split_top_logical`], but for a whitespace-delimited word operator
/// (`and`/`or`) rather than a punctuation symbol — requires a single space
/// on each side (via [`find_top_level`]) so the word appearing inside a
/// longer identifier or string (`for`, `orange`, …) is never mistaken for
/// the operator.
fn split_top_logical_word<'a>(expr: &'a str, word: &str) -> Option<(&'a str, &'a str)> {
    let needle = format!(" {word} ");
    let pos = find_top_level(expr, &needle)?;
    Some((&expr[..pos], &expr[pos + needle.len()..]))
}

/// Negate a word-style operand: `$a` → `not $a`, `not $a` → `$a`, a bare
/// `!`-prefixed operand also collapses (mixed-style input) — mirrors
/// [`negate`] but for iRules' `not` spelling.
fn negate_word(operand: &str, word_not: &str) -> String {
    let o = operand.trim();
    if let Some(rest) = o.strip_prefix('!') {
        return rest.trim().to_string();
    }
    if let Some(rest) = strip_word_not(o, word_not) {
        return rest.trim().to_string();
    }
    format!("{word_not} {o}")
}

/// `Some(rest)` when `operand` is `"<word_not> rest"` — a word-boundary
/// check (a required space after `word_not`) so `notify_x` is never
/// mistaken for a negated `x`.
fn strip_word_not<'a>(operand: &'a str, word_not: &str) -> Option<&'a str> {
    operand.strip_prefix(word_not)?.strip_prefix(' ')
}

/// Negate an operand: `$a` → `!$a`, `!$a` → `$a`, `($a && $b)` → `!($a && $b)`.
fn negate(operand: &str) -> String {
    let o = operand.trim();
    if let Some(rest) = o.strip_prefix('!') {
        rest.trim().to_string()
    } else {
        format!("!{o}")
    }
}

/// Split `expr` on the top-level (brace/paren-depth 0) occurrence of `op`.
fn split_top_logical<'a>(expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let bytes = expr.as_bytes();
    let opb = op.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + opb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + opb.len()] == opb {
            return Some((&expr[..i], &expr[i + opb.len()..]));
        }
        i += 1;
    }
    None
}

/// Every comparison operator spelling paired with its inverse — derived
/// from `BinOp::inverse()` (`tcl_syntax::expr::operators`, issue #983's
/// unification) rather than a hand-typed list, which used to be missing
/// the TIP 461 string-ordering four (`lt`/`le`/`gt`/`ge`) entirely: the
/// "Invert comparison" quick fix never even offered itself for a selection
/// containing one of those. Order doesn't matter for correctness — each
/// needle is matched as a *space-delimited* unit (`find_top_level` looks
/// for `" op "`), so e.g. `" < "` and `" <= "` can never collide as
/// substrings of each other regardless of which is tried first.
fn comparison_inversions() -> &'static [(&'static str, &'static str)] {
    static OPS: std::sync::OnceLock<Vec<(&'static str, &'static str)>> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let inv = op.inverse()?;
                Some((op.spec().spelling, inv.spec().spelling))
            })
            .collect()
    })
}

/// Invert the (single) top-level comparison operator in `sel`.
fn invert_comparison(sel: &str) -> Option<String> {
    let t = sel.trim();
    for (from, to) in comparison_inversions() {
        // Require the operator to be surrounded by spaces so `$a == $b` matches
        // but a bare `<` inside a name doesn't; word ops need word boundaries.
        let needle = format!(" {from} ");
        if let Some(pos) = find_top_level(t, &needle) {
            let mut result = String::with_capacity(t.len());
            result.push_str(&t[..pos]);
            result.push(' ');
            result.push_str(to);
            result.push(' ');
            result.push_str(&t[pos + needle.len()..]);
            return Some(result);
        }
    }
    None
}

/// Find ` needle ` at brace/paren depth 0.
fn find_top_level(expr: &str, needle: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let nb = needle.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + nb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + nb.len()] == nb {
            return Some(i);
        }
        i += 1;
    }
    None
}

// Generate docstring (source action).

fn docstring_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    let mut out = Vec::new();
    for proc_def in analysis.all_procs.values() {
        let decl = line_index.position_at_utf16(proc_def.name_span.start(), source);
        if decl.line != range.start_line {
            continue;
        }
        // Skip procs that already carry a doc-comment.
        if !proc_def.doc.is_empty() {
            continue;
        }
        // The DOXYGEN stub (`# @brief TODO: describe <proc>` + one `# @param`
        // line per parameter) is rendered by the shared docstring generator;
        // the code action inserts it as a block, so add the trailing newline
        // that separates it from the `proc` line below.
        let doc = format!(
            "{}\n",
            crate::formatting::generate_stub_for_proc(
                proc_def,
                crate::formatting::DocstringTagStyle::Doxygen,
                false,
                '.',
                70,
                "",
            )
        );
        out.push(CodeAction {
            title: format!("Generate docstring for '{}'", proc_def.name),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: decl.line,
                    start_character: 0,
                    end_line: decl.line,
                    end_character: 0,
                },
                new_text: doc,
            }],
            kind: ActionKind::Source,
            command: None,
            data_group_definition: None,
        });
    }
    out
}

fn extract_inline_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    // Load the iRules dialect so `when`-body descent and the
    // `class match` / `class lookup` data-group form resolve.  Loading is
    // additive — vanilla command resolution is unchanged — so we do it
    // unconditionally rather than threading the document dialect through
    // the code-action signature (the data-group transform self-gates on
    // the registry resolving a `class` form).
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_dialect(tcl_dialect::DialectSet::IRULES);
    let mut out = Vec::new();
    out.extend(extract_proc_action(source, range));
    out.extend(inline_proc_action(source, range, analysis, &registry));
    out.extend(refactor_engine_actions(
        source, range, analysis, line_index, &registry,
    ));
    out
}

/// Surface the [`crate::refactor`] transforms (extract / inline variable,
/// if↔switch, switch→dict, extract-to-datagroup) as `CodeAction`s.
///
/// The cursor is `range`'s start; extract-variable additionally needs a
/// non-empty selection.  The data-group transform is iRules-only — it is
/// gated by [`crate::refactor::extract_to_datagroup`]'s registry
/// resolution (a non-iRules registry resolves no `class match` /
/// `class lookup` form), so offering it unconditionally here is safe.
fn refactor_engine_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<CodeAction> {
    use crate::refactor;
    let mut out = Vec::new();

    let cursor = line_index.offset_at_utf16(
        range.start_line,
        Utf16Col::new(range.start_character),
        source,
    );
    let has_selection =
        range.start_line != range.end_line || range.start_character != range.end_character;

    // Extract variable — requires a selection.
    if has_selection {
        let end =
            line_index.offset_at_utf16(range.end_line, Utf16Col::new(range.end_character), source);
        if let Some(r) = refactor::extract_variable(source, cursor, end, "result", line_index) {
            out.push(refactoring_to_action(&r, source, line_index));
        }
    }

    if let Some(r) = refactor::inline_variable(source, cursor, analysis, registry, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if let Some(r) = refactor::if_to_switch(source, cursor, registry, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if let Some(r) = refactor::switch_to_dict(source, cursor, registry, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if let Some(r) = refactor::extract_to_datagroup(source, cursor, "", registry, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    out
}

/// Lift a [`crate::refactor::Refactoring`] into a [`CodeAction`],
/// converting its byte-offset edits to LSP coordinates and rendering the
/// data-group definition (if any) into `data_group_definition`.
fn refactoring_to_action(
    r: &crate::refactor::Refactoring,
    source: &str,
    line_index: &LineIndex,
) -> CodeAction {
    CodeAction {
        title: r.title.clone(),
        edits: r
            .edits
            .iter()
            .map(|e| e.to_lsp(source, line_index))
            .collect(),
        kind: r.kind,
        command: None,
        data_group_definition: r.data_group.as_ref().map(crate::refactor::data_group_tcl),
    }
}

/// Distinct `$var` / `${var}` names referenced in `text`, in first-seen order.
fn referenced_vars(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            let braced = chars.get(i + 1) == Some(&'{');
            let mut j = i + 1 + usize::from(braced);
            let start = j;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == ':')
            {
                j += 1;
            }
            if j > start {
                let name: String = chars[start..j].iter().collect();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// `refactor.extract` — extract the selected lines into a new proc and replace
/// the selection with a call.
fn extract_proc_action(source: &str, range: LspRange) -> Vec<CodeAction> {
    // Need a non-empty selection.
    if range.start_line == range.end_line && range.start_character >= range.end_character {
        return Vec::new();
    }
    let lines: Vec<&str> = source.split('\n').collect();
    // The selected line span — a selection ending at column 0 doesn't include
    // its end line.
    let last = if range.end_character == 0 {
        range.end_line.saturating_sub(1)
    } else {
        range.end_line
    };
    let (s, e) = (range.start_line as usize, last as usize);
    if s > e || e >= lines.len() {
        return Vec::new();
    }
    let block: Vec<&str> = lines[s..=e].to_vec();
    let body_text = block.join("\n");
    if body_text.trim().is_empty() {
        return Vec::new();
    }
    let params = referenced_vars(&body_text);
    let name = "extracted_proc";
    let body_indented = block
        .iter()
        .map(|l| format!("    {}", l.trim_end()))
        .collect::<Vec<_>>()
        .join("\n");
    let proc_text = format!(
        "proc {name} {{{}}} {{\n{body_indented}\n}}\n\n",
        params.join(" ")
    );
    let call = if params.is_empty() {
        name.to_string()
    } else {
        format!(
            "{name} {}",
            params
                .iter()
                .map(|p| format!("${p}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    // Replace the selected lines with the call.
    let replace = LspRange {
        start_line: range.start_line,
        start_character: 0,
        end_line: u32::try_from(e + 1).unwrap_or(range.end_line),
        end_character: 0,
    };
    let name_start = u32::try_from("proc ".len()).unwrap_or(5);
    vec![CodeAction {
        title: "Extract selection into proc".to_string(),
        edits: vec![
            crate::rename::TextEdit {
                range: LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 0,
                    end_character: 0,
                },
                new_text: proc_text,
            },
            crate::rename::TextEdit {
                range: replace,
                new_text: format!("{call}\n"),
            },
        ],
        kind: ActionKind::RefactorExtract,
        // Trigger a rename of the generated proc name (line 0).
        command: Some(ActionCommand {
            command: "tclLsp.renameSymbolAtPosition".to_string(),
            args: vec![
                0,
                name_start,
                name_start + u32::try_from(name.len()).unwrap_or(0),
            ],
            string_args: Vec::new(),
        }),
        data_group_definition: None,
    }]
}

/// Split one Tcl command line into its words' raw source slices, respecting
/// `{…}` / `"…"` grouping, `[…]` command-substitution nesting, and backslash
/// escapes.  Unlike `str::split_whitespace`, a braced argument `{a b}` stays a
/// single word rather than shredding into `{a` and `b}` (issue 179).
fn split_tcl_words(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let n = bytes.len();
    let mut words = Vec::new();
    let mut i = 0;
    while i < n {
        while i < n && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }
        let start = i;
        let mut brace: i32 = 0;
        let mut bracket: i32 = 0;
        let mut in_quote = false;
        while i < n {
            match bytes[i] {
                b'\\' if i + 1 < n => {
                    // Skip the backslash and the *whole* escaped character. A
                    // fixed `i += 2` would land mid-codepoint when the escaped
                    // char is multi-byte (e.g. `\€`); step its full UTF-8 width
                    // so `i` always stays on a char boundary.
                    i += 1 + utf8_char_width(bytes[i + 1]);
                    continue;
                }
                b'{' if !in_quote => brace += 1,
                b'}' if !in_quote && brace > 0 => brace -= 1,
                b'"' if brace == 0 && bracket == 0 => in_quote = !in_quote,
                b'[' if !in_quote && brace == 0 => bracket += 1,
                b']' if !in_quote && brace == 0 && bracket > 0 => bracket -= 1,
                b' ' | b'\t' if brace == 0 && bracket == 0 && !in_quote => break,
                _ => {}
            }
            i += 1;
        }
        // Every advance lands on a char boundary — whitespace and the grouping
        // delimiters are ASCII, and the `\`-skip steps whole characters — so
        // this slice is always valid; `i.min(n)` guards only the numeric bound.
        words.push(&line[start..i.min(n)]);
    }
    words
}

/// UTF-8 encoded length (1–4 bytes) of the character whose leading byte is `b`.
/// ASCII, continuation bytes, and invalid leading bytes all count as 1 — so a
/// scan that starts mid-sequence still makes forward progress rather than
/// stalling.
fn utf8_char_width(b: u8) -> usize {
    match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// End index (exclusive) of a bare `$name` variable name in `chars` starting at
/// `start`, colon-run aware like C Tcl (`$a::b`, `$a:::b`), stopping at a lone
/// `:`.  Shared shape with `hover::scan_var_name_end`.
fn scan_var_name_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() {
        let c = chars[end];
        if c.is_alphanumeric() || c == '_' {
            end += 1;
        } else if c == ':' && chars.get(end + 1) == Some(&':') {
            end += 2;
            while chars.get(end) == Some(&':') {
                end += 1;
            }
        } else {
            break;
        }
    }
    end
}

/// Substitute `$name` / `${name}` variable *references* in `body` from the
/// `(name, replacement)` map, matching only *complete* names.  A naive
/// `str::replace("$n", …)` would corrupt `$nn` (prefix sharing) or a `$name`
/// embedded in a longer token; this scans references and replaces whole ones
/// (issue 179).
fn substitute_var_refs(body: &str, subs: &[(&str, &str)]) -> String {
    let lookup = |name: &str| subs.iter().find(|(n, _)| *n == name).map(|(_, r)| *r);
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            if chars.get(i + 1) == Some(&'{') {
                // `${name}` — literal name up to the first `}`.
                if let Some(rel) = chars[i + 2..].iter().position(|&c| c == '}') {
                    let name: String = chars[i + 2..i + 2 + rel].iter().collect();
                    if let Some(rep) = lookup(&name) {
                        out.push_str(rep);
                        i = i + 2 + rel + 1;
                        continue;
                    }
                }
            } else {
                let end = scan_var_name_end(&chars, i + 1);
                if end > i + 1 {
                    let name: String = chars[i + 1..end].iter().collect();
                    if let Some(rep) = lookup(&name) {
                        out.push_str(rep);
                        i = end;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `refactor.inline` — inline a single-command proc at the call cursor.
/// Declines branchy / control-flow bodies.
fn inline_proc_action(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<CodeAction> {
    // Frame-sensitive commands (block terminators, control transfers, scope
    // aliases, barriers — the registry's `is_frame_sensitive` union) whose
    // bodies can't be safely inlined: moving them out of the proc frame
    // changes what they return from, break out of, or bind against.
    let unsafe_heads = registry.frame_sensitive_commands();
    let line = source
        .split('\n')
        .nth(range.start_line as usize)
        .unwrap_or("");
    // Split the call into proper Tcl words, respecting `{…}` / `"…"` / `[…]`
    // grouping — `split_whitespace` would shred a braced argument `{a b}` into
    // `{a` and `b}` (issue 179).
    let toks: Vec<&str> = split_tcl_words(line);
    let Some(&head) = toks.first() else {
        return Vec::new();
    };
    // Resolve the call head the way the navigation providers do — the
    // caller's namespace candidates, the registry builtin gate, then the
    // deterministic simple-name fallback — never a namespace-blind
    // `p.name == head` first-`HashMap`-hit scan (the M1 drift class the
    // `cargo xtask resolution-drift` lint flags).
    let line_start: usize = source
        .split_inclusive('\n')
        .take(range.start_line as usize)
        .map(str::len)
        .sum();
    let head_off = u32::try_from(line_start + line.find(head).unwrap_or(0)).unwrap_or(u32::MAX);
    let ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        head_off,
        &analysis.namespace_overrides,
    );
    let Some(proc_def) =
        crate::definition::resolve_called_proc(analysis, source, &ns, head, Some(registry))
    else {
        return Vec::new();
    };
    // Body text (strip the outer braces). `body_span` may exclude the proc's
    // closing `}` (lexer inner-end convention), so strip a trailing `}` only
    // when it is the unbalanced *outer* brace — a greedy `trim_end_matches('}')`
    // would otherwise eat an inner sub-expression brace (`expr {$n * 2}`) and
    // produce an unparseable inline edit (`expr {5 * 2`).
    let bspan = proc_def.body_span;
    let raw = source
        .get(bspan.start() as usize..bspan.end() as usize)
        .unwrap_or("")
        .trim();
    let inner = raw.strip_prefix('{').map_or(raw, str::trim_start);
    let body_raw = if inner.matches('}').count() > inner.matches('{').count() {
        let t = inner.trim_end();
        t.strip_suffix('}').unwrap_or(t).trim()
    } else {
        inner.trim()
    };
    // Decline control-flow / multi-command bodies.
    if body_raw.is_empty()
        || body_raw.contains('\n')
        || body_raw.contains(';')
        || unsafe_heads.iter().any(|kw| {
            body_raw == *kw
                || body_raw.starts_with(&format!("{kw} "))
                || body_raw.contains(&format!("[{kw} "))
        })
    {
        return Vec::new();
    }
    // Map call args onto params, then substitute variable *references* — not
    // by naive string replace, which would rewrite `$nn` when the param is `n`
    // (prefix sharing) and mangle `$name` inside a longer token (issue 179).
    let call_args: Vec<&str> = toks[1..].to_vec();
    let subs: Vec<(&str, &str)> = proc_def
        .params
        .iter()
        .enumerate()
        .filter_map(|(i, p)| call_args.get(i).map(|arg| (p.name.as_str(), *arg)))
        .collect();
    let inlined = substitute_var_refs(body_raw, &subs);
    // Replace the whole call line.
    vec![CodeAction {
        title: format!("Inline proc '{}'", proc_def.name),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: range.start_line,
                start_character: 0,
                end_line: range.start_line,
                end_character: char_col_to_utf16_local(line, line.chars().count()),
            },
            new_text: inlined,
        }],
        kind: ActionKind::RefactorInline,
        command: None,
        data_group_definition: None,
    }]
}

// iRules `# Profiles:` header source action.

/// Compute the sorted required virtual-server profiles from the file's events
/// (`EventProps.implied_profiles`) and commands (`event_requires.profiles`).
fn compute_required_profiles(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut profiles: BTreeSet<String> = BTreeSet::new();
    let events = tcl_registry::events::EventRegistry::build();
    for ev in crate::irules_context::scan_file_events(source, "f5-irules") {
        if let Some(props) = events.get_props(&ev) {
            for p in props.implied_profiles {
                profiles.insert((*p).to_string());
            }
        }
    }
    for inv in &analysis.command_invocations {
        if let Some(spec) = registry.get(&inv.name)
            && let Some(req) = spec.event_requires.as_ref()
        {
            for p in req.profiles {
                profiles.insert((*p).to_string());
            }
        }
    }
    // FASTHTTP is an alternative to HTTP; keep only HTTP when both appear.
    if profiles.contains("HTTP") {
        profiles.remove("FASTHTTP");
    }
    // Drop the stack-implied transport / shared-TLS profiles — classified
    // from each profile's `layer` in the registry, not a hardcoded list.
    let profile_registry = tcl_registry::profiles::ProfileRegistry::build();
    profiles.retain(|p| !profile_registry.is_infrastructure_profile(p));
    // Emit in protocol-stack order (transport → TLS → application → …) using
    // the registry's `layer` metadata, not the `BTreeSet`'s alphabetical
    // order — alphabetical would list e.g. `HTTP` before `SERVERSSL` (an
    // application profile ahead of its TLS layer), or `ASM` before `HTTP`.
    let mut ordered: Vec<String> = profiles.into_iter().collect();
    ordered.sort_by_key(|p| (profile_registry.layer_rank(p), p.clone()));
    ordered
}

/// Scan leading comment lines for a `# Profiles: HTTP, CLIENTSSL` directive,
/// returning `(uppercased profile set, line index)`.
fn scan_profile_directive(source: &str) -> Option<(std::collections::BTreeSet<String>, u32)> {
    for (i, line) in source.split('\n').enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !t.starts_with('#') {
            break; // first non-comment content — stop scanning
        }
        let body = t.trim_start_matches('#').trim();
        let lower = body.to_ascii_lowercase();
        if lower.starts_with("profile")
            && let Some(colon) = body.find(':')
        {
            let set: std::collections::BTreeSet<String> = body[colon + 1..]
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_uppercase)
                .collect();
            return Some((set, u32::try_from(i).unwrap_or(0)));
        }
    }
    None
}

/// Build the `# Profiles:` source action (insert or update) for an iRules
/// document.  Returns `None` when no profiles are required or the existing
/// directive already matches.  The caller gates on the iRules dialect.
#[must_use]
pub fn profiles_action(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Option<CodeAction> {
    let required = compute_required_profiles(source, analysis, registry);
    if required.is_empty() {
        return None;
    }
    let new_text = format!("# Profiles: {}\n", required.join(", "));
    if let Some((existing, line_no)) = scan_profile_directive(source) {
        let required_set: std::collections::BTreeSet<String> = required.iter().cloned().collect();
        if existing == required_set {
            return None;
        }
        return Some(CodeAction {
            title: format!(
                "Update profile requirements \u{2192} {}",
                required.join(", ")
            ),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: line_no,
                    start_character: 0,
                    end_line: line_no + 1,
                    end_character: 0,
                },
                new_text,
            }],
            kind: ActionKind::Source,
            command: None,
            data_group_definition: None,
        });
    }
    Some(CodeAction {
        title: format!(
            "Generate profile requirements header ({})",
            required.join(", ")
        ),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            new_text,
        }],
        kind: ActionKind::Source,
        command: None,
        data_group_definition: None,
    })
}

// iRules taint quick-fixes — driven by the *context* diagnostics the editor
// sends (the analyser may not have re-emitted them), so they take a separate
// entry point.

/// A diagnostic supplied in the code-action request context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextDiagnostic {
    /// Diagnostic code (e.g. `IRULE3001`).
    pub code: String,
    /// Human-readable message (carries the tainted `$var`).
    pub message: String,
    /// The diagnostic's range.
    pub range: LspRange,
}

const HTML_ENCODE_PROC: &str =
    "proc html_encode {str} { string map {& &amp; < &lt; > &gt; \\\" &quot; ' &#39;} $str }";
const REGEX_QUOTE_PROC: &str =
    "proc regex::quote {str} { regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&} }";
/// `string map` mapping that strips CR/LF — the fix the T101 / IRULE3003 KCS
/// docs recommend for an output/log sink (`puts $x` / `log ... $x`): a
/// `string map` element beginning with `"` is itself list-parsed with
/// backslash escapes honoured, so `"\n"` / `"\r"` really do map the newline /
/// carriage-return characters to empty, not the two-character sequences.
const CRLF_STRIP_MAP: &str = "string map {\"\\n\" \"\" \"\\r\" \"\"}";

/// Quick-fixes for context-supplied diagnostics (iRules taint encode-wrap +
/// double-encode removal).
#[must_use]
pub fn context_diagnostic_actions(source: &str, diags: &[ContextDiagnostic]) -> Vec<CodeAction> {
    let mut out = Vec::new();
    for d in diags {
        out.extend(taint_quickfix(source, d));
        out.extend(collect_bootstrap_actions(source, d));
    }
    // De-duplicate (two IRULE1006 diags for the same buffer command yield the
    // same bootstrap action).  Key on the title + edit replacement texts.
    let mut seen = FxHashSet::default();
    out.retain(|a| {
        let key = (
            a.title.clone(),
            a.edits
                .iter()
                .map(|e| e.new_text.clone())
                .collect::<Vec<_>>(),
        );
        seen.insert(key)
    });
    out
}

/// Protocol names `X` for which `X::<suffix>` appears in `message`.
fn protocols_before(message: &str, suffix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = message.as_bytes();
    let mut search = 0;
    while let Some(rel) = message[search..].find(suffix) {
        let end = search + rel;
        let mut start = end;
        while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            start -= 1;
        }
        if start < end {
            out.push(message[start..end].to_ascii_uppercase());
        }
        search = end + suffix.len();
    }
    out
}

/// The `when`-event setup event a `protocol::collect` bootstrap belongs in.
fn setup_event_for(protocol: &str, event: &str) -> String {
    let ev = event.to_ascii_uppercase();
    match protocol {
        "HTTP" if ev.starts_with("HTTP_RESPONSE") => "HTTP_RESPONSE".to_string(),
        "HTTP" => "HTTP_REQUEST".to_string(),
        "SSL" if ev.starts_with("SERVERSSL") => "SERVERSSL_HANDSHAKE".to_string(),
        "SSL" => "CLIENTSSL_HANDSHAKE".to_string(),
        _ if ev.starts_with("SERVER") => "SERVER_CONNECTED".to_string(),
        _ => "CLIENT_ACCEPTED".to_string(),
    }
}

/// Line index of the `when` block enclosing `line` (scanning upward), or 0.
fn enclosing_when_line(source: &str, line: u32) -> u32 {
    let lines: Vec<&str> = source.split('\n').collect();
    let start = (line as usize).min(lines.len().saturating_sub(1));
    for i in (0..=start).rev() {
        if lines[i].trim_start().starts_with("when ") {
            return u32::try_from(i).unwrap_or(0);
        }
    }
    0
}

/// IRULE1005 / IRULE1006 "missing collect" quick-fixes: insert a
/// `when <setup> priority 500 { <proto>::collect }` bootstrap block.
fn collect_bootstrap_actions(source: &str, d: &ContextDiagnostic) -> Vec<CodeAction> {
    if d.code != "IRULE1005" && d.code != "IRULE1006" {
        return Vec::new();
    }
    let anchor = enclosing_when_line(source, d.range.start_line);
    let event =
        crate::irules_context::find_enclosing_when_event(source, d.range.start_line, "f5-irules")
            .unwrap_or_default();

    let (protocols, setup_event) = if d.code == "IRULE1005" {
        // The data event is the word in the diag range; collect protocols from
        // the "X::collect" mentions in the message.
        let data_event = {
            let line = source
                .split('\n')
                .nth(d.range.start_line as usize)
                .unwrap_or("");
            let chars: Vec<char> = line.chars().collect();
            let s = (d.range.start_character as usize).min(chars.len());
            let e = (d.range.end_character as usize).min(chars.len());
            chars[s..e].iter().collect::<String>().to_ascii_uppercase()
        };
        (protocols_before(&d.message, "::collect"), data_event)
    } else {
        // IRULE1006 — the buffer command is `X::payload`.
        (protocols_before(&d.message, "::payload"), event)
    };

    let mut unique: Vec<String> = Vec::new();
    for p in protocols {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    unique
        .into_iter()
        .map(|proto| {
            let setup = setup_event_for(&proto, &setup_event);
            CodeAction {
                title: format!("Add '{proto}::collect' bootstrap in '{setup}'"),
                edits: vec![crate::rename::TextEdit {
                    range: LspRange {
                        start_line: anchor,
                        start_character: 0,
                        end_line: anchor,
                        end_character: 0,
                    },
                    new_text: format!("when {setup} priority 500 {{\n    {proto}::collect\n}}\n\n"),
                }],
                kind: ActionKind::QuickFix,
                command: None,
                data_group_definition: None,
            }
        })
        .collect()
}

/// Extract the variable name (no `$`/braces) named in a taint message.
fn taint_var_name(message: &str) -> Option<String> {
    let bytes = message.as_bytes();
    let dollar = message.find('$')?;
    let mut i = dollar + 1;
    let braced = bytes.get(i) == Some(&b'{');
    if braced {
        i += 1;
    }
    let start = i;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b':' {
            i += 1;
        } else {
            break;
        }
    }
    if i == start {
        return None;
    }
    Some(message[start..i].to_string())
}

/// Find the `$name` / `${name}` reference for `var` on the diagnostic's start
/// line, returning `(char_start, char_end, matched_text)`.
fn find_var_ref(line: &str, var: &str) -> Option<(usize, usize, String)> {
    let braced = format!("${{{var}}}");
    let bare = format!("${var}");
    if let Some(b) = line.find(&braced) {
        let cstart = line[..b].chars().count();
        return Some((cstart, cstart + braced.chars().count(), braced));
    }
    // Bare form — require the next char not to be a var-continuation so `$ab`
    // doesn't match for `$a`.
    let mut search_from = 0;
    while let Some(rel) = line[search_from..].find(&bare) {
        let b = search_from + rel;
        let after = line[b + bare.len()..].chars().next();
        if after.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == ':')) {
            let cstart = line[..b].chars().count();
            return Some((cstart, cstart + bare.chars().count(), bare));
        }
        search_from = b + bare.len();
    }
    None
}

/// T106: remove a redundant `[ENCODER $var]` wrapper → `$var`.
fn t106_remove_redundant_encoder(line: &str, d: &ContextDiagnostic, var: &str) -> Vec<CodeAction> {
    let Some((vstart, vend, matched)) = find_var_ref(line, var) else {
        return Vec::new();
    };
    let chars: Vec<char> = line.chars().collect();
    // Scan left for the enclosing `[`, right for `]`.
    let mut lb = vstart;
    while lb > 0 && chars[lb - 1] != '[' {
        lb -= 1;
    }
    let mut rb = vend;
    while rb < chars.len() && chars[rb] != ']' {
        rb += 1;
    }
    if lb == 0 || rb >= chars.len() {
        return Vec::new();
    }
    let start = char_col_to_utf16_local(line, lb - 1);
    let end = char_col_to_utf16_local(line, rb + 1);
    vec![CodeAction {
        title: "Remove redundant encoder".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: d.range.start_line,
                start_character: start,
                end_line: d.range.start_line,
                end_character: end,
            },
            new_text: matched,
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    }]
}

/// T101 (`puts`) / IRULE3003 (`log`): wrap with a CR/LF-stripping `string
/// map` — the fix the KCS docs for both codes recommend, since neither sink
/// is HTML/URI-context-specific (unlike IRULE3001's response body or
/// IRULE3002's header value) so an HTML/URL encoder would be the wrong
/// mitigation to suggest here.
fn strip_crlf_before_output(line: &str, d: &ContextDiagnostic, var: &str) -> Vec<CodeAction> {
    let Some((vstart, vend, matched)) = find_var_ref(line, var) else {
        return Vec::new();
    };
    let start = char_col_to_utf16_local(line, vstart);
    let end = char_col_to_utf16_local(line, vend);
    vec![CodeAction {
        title: format!("Sanitise ${var} (strip CR/LF) before output"),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: d.range.start_line,
                start_character: start,
                end_line: d.range.start_line,
                end_character: end,
            },
            new_text: format!("[{CRLF_STRIP_MAP} {matched}]"),
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    }]
}

/// Build the "add `-nocommands`" T100 quick-fix for a `subst` sink:
/// inserts `-nocommands` right after the standalone `subst` word on
/// `line`, or no action when the line doesn't contain one (defensive —
/// the caller already matched the diagnostic message, so this should
/// always find it).
fn subst_nocommands_fix(line: &str, line_no: u32) -> Vec<CodeAction> {
    let bytes = line.as_bytes();
    let Some(start) = line.find("subst") else {
        return Vec::new();
    };
    let word_start_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
    let after = start + "subst".len();
    let word_end_ok = bytes.get(after).is_none_or(|c| !c.is_ascii_alphanumeric());
    if !word_start_ok || !word_end_ok {
        return Vec::new();
    }
    let insert_char_col = line[..after].chars().count();
    let insert_col = char_col_to_utf16_local(line, insert_char_col);
    vec![CodeAction {
        title: "Add -nocommands to disable command substitution".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: line_no,
                start_character: insert_col,
                end_line: line_no,
                end_character: insert_col,
            },
            new_text: " -nocommands".to_string(),
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    }]
}

fn taint_quickfix(source: &str, d: &ContextDiagnostic) -> Vec<CodeAction> {
    let line_no = d.range.start_line as usize;
    let Some(line) = source.split('\n').nth(line_no) else {
        return Vec::new();
    };
    let Some(var) = taint_var_name(&d.message) else {
        return Vec::new();
    };

    if d.code == "T106" {
        return t106_remove_redundant_encoder(line, d, &var);
    }
    if d.code == "T101" || d.code == "IRULE3003" {
        return strip_crlf_before_output(line, d, &var);
    }

    // T100 on a `subst` sink: `-nocommands` disables the only hazard the
    // diagnostic names (command substitution) without changing the call's
    // variable/backslash substitution behaviour — exactly the mitigation
    // `subst`'s own hover snippet recommends. Other T100 sinks (`eval`,
    // `uplevel`, `exec`, a braced `expr` operand) have no equivalent
    // single-flag fix, so this only fires for the `subst` sink label.
    if d.code == "T100" && d.message.contains(" into subst;") {
        return subst_nocommands_fix(line, d.range.start_line);
    }

    let (encoder, proc_template): (&str, Option<&str>) = match d.code.as_str() {
        "IRULE3001" => ("html_encode", Some(HTML_ENCODE_PROC)),
        "IRULE3002" => ("URI::encode", None),
        "T103" => ("regex::quote", Some(REGEX_QUOTE_PROC)),
        _ => return Vec::new(),
    };
    let Some((vstart, vend, matched)) = find_var_ref(line, &var) else {
        return Vec::new();
    };
    let start = char_col_to_utf16_local(line, vstart);
    let end = char_col_to_utf16_local(line, vend);
    let mut edits = vec![crate::rename::TextEdit {
        range: LspRange {
            start_line: d.range.start_line,
            start_character: start,
            end_line: d.range.start_line,
            end_character: end,
        },
        new_text: format!("[{encoder} {matched}]"),
    }];
    // Insert the helper proc at the top of the file when it isn't defined and
    // the encoder is a user proc (html_encode / regex::quote; URI::encode is
    // a built-in F5 command).
    if let Some(template) = proc_template {
        let proc_name = encoder;
        if !source.contains(&format!("proc {proc_name}")) {
            edits.push(crate::rename::TextEdit {
                range: LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 0,
                    end_character: 0,
                },
                new_text: format!("{template}\n"),
            });
        }
    }
    vec![CodeAction {
        title: format!("Wrap ${var} with [{encoder}]"),
        edits,
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult, CodeFix, Diagnostic};
    use tcl_lexer::Span;

    #[test]
    fn split_tcl_words_survives_non_ascii_backslash_escape() {
        // A `\`-escape before a multi-byte char must not leave the scanner mid
        // codepoint and panic the `&line[start..i]` slice (Copilot review of
        // #839). `\€` (euro = 3 bytes) is the canonical trigger.
        for line in [
            "set x \\\u{20ac}",     // escape then a 3-byte char at end of line
            "puts \\\u{20ac} tail", // escaped multi-byte followed by another word
            "a \\\u{1f600} b",      // 4-byte astral escaped char mid-line
            "\\\u{e9}nd",           // escaped 2-byte char fused into a word
        ] {
            let words = split_tcl_words(line);
            // Round-trip: the joined words (single-space) recover every word slice
            // without ever slicing mid-char, and no word is empty.
            assert!(!words.is_empty(), "no words for {line:?}");
            for w in &words {
                assert!(line.contains(w), "word {w:?} not a slice of {line:?}");
            }
        }
    }

    fn whole_document_range(source: &str) -> LspRange {
        let line_count = source.lines().count().max(1);
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::try_from(line_count - 1).unwrap_or(0),
            end_character: u32::MAX,
        }
    }

    #[test]
    fn empty_actions_when_analysis_is_none() {
        assert!(code_actions("set x 1\n", whole_document_range("set x 1\n"), None).is_empty());
    }

    #[test]
    fn fix_attached_to_diagnostic_surfaces_as_action() {
        // Build a synthetic AnalysisResult with one diagnostic
        // and one fix.  Verifies the lift logic in isolation
        // from the analyser's diagnostic emitters.
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: DiagCode::W210,
            message: "Variable read before set".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "set var 0".to_string(),
                description: "Initialise `var`".to_string(),
            }],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        let qf: Vec<&CodeAction> = actions
            .iter()
            .filter(|a| a.kind == ActionKind::QuickFix)
            .collect();
        assert_eq!(qf.len(), 1, "{actions:?}");
        assert_eq!(qf[0].title, "Initialise `var`");
        assert_eq!(qf[0].edits.len(), 1);
        assert_eq!(qf[0].edits[0].new_text, "set var 0");
    }

    #[test]
    fn no_action_when_range_outside_diagnostic() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: DiagCode::W210,
            message: "msg".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "fix".to_string(),
                description: "Fix".to_string(),
            }],
        });
        // Request range on line 99 — far away from the
        // diagnostic's line 0.
        let far_range = LspRange {
            start_line: 99,
            start_character: 0,
            end_line: 99,
            end_character: 10,
        };
        assert!(code_actions("set x 1\n", far_range, Some(&r)).is_empty());
    }

    #[test]
    fn empty_description_falls_back_to_diagnostic_message() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: DiagCode::W210,
            message: "Variable read before set".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "x".to_string(),
                description: String::new(), // No description.
            }],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        let qf: Vec<&CodeAction> = actions
            .iter()
            .filter(|a| a.kind == ActionKind::QuickFix)
            .collect();
        assert_eq!(qf.len(), 1);
        assert!(qf[0].title.contains("Variable read before set"));
    }

    #[test]
    fn no_actions_when_analyser_has_no_diagnostics_with_fixes() {
        // Run the actual analyser; with a clean source no
        // fixable diagnostics fire and the result is empty.
        let mut a = Analyser::new();
        let analysis = a.analyse("set x 1\nputs $x\n", "tcl8.6").clone();
        let actions = code_actions(
            "set x 1\nputs $x\n",
            whole_document_range("set x 1\nputs $x\n"),
            Some(&analysis),
        );
        // No diagnostic fixes → no quick-fix actions (range-based refactors
        // like extract-proc may still be offered for the selection).
        assert!(
            !actions.iter().any(|a| a.kind == ActionKind::QuickFix),
            "{actions:?}",
        );
    }

    #[test]
    fn multiple_fixes_on_one_diagnostic_each_become_an_action() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: DiagCode::W210,
            message: "msg".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![
                CodeFix {
                    span: Span::new(0, 5),
                    new_text: "a".into(),
                    description: "A".into(),
                },
                CodeFix {
                    span: Span::new(0, 5),
                    new_text: "b".into(),
                    description: "B".into(),
                },
            ],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        let titles: Vec<&str> = actions
            .iter()
            .filter(|a| a.kind == ActionKind::QuickFix)
            .map(|a| a.title.as_str())
            .collect();
        assert_eq!(titles.len(), 2);
        assert!(titles.contains(&"A") && titles.contains(&"B"));
    }

    // refactor.inline — proc inlining (issue 179)

    fn line_range(line: u32) -> LspRange {
        LspRange {
            start_line: line,
            start_character: 0,
            end_line: line,
            end_character: 0,
        }
    }

    fn inline_edit_text(src: &str, call_line: u32) -> String {
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let actions = code_actions(src, line_range(call_line), Some(&analysis));
        let act = actions
            .iter()
            .find(|a| a.title.starts_with("Inline proc "))
            .unwrap_or_else(|| panic!("expected an inline action in {actions:?}"));
        act.edits[0].new_text.clone()
    }

    #[test]
    fn tp_inline_preserves_braced_argument_as_one_word() {
        // `f {a b}` — the braced argument is a single value; inlining `$p`
        // must yield `puts {a b}`, not `puts {a` from a whitespace split
        // (issue 179).
        let src = "proc f {p} { puts $p }\nf {a b}\n";
        assert_eq!(inline_edit_text(src, 1), "puts {a b}");
    }

    #[test]
    fn tp_inline_does_not_substitute_prefix_sharing_name() {
        // Param `n`; body reads `$nn` (a different variable) and `$n`. Only the
        // complete `$n` reference is replaced — `$nn` must survive intact,
        // where a naive `replace("$n", …)` would corrupt it (issue 179).
        let src = "proc g {n} { set x $nn$n }\ng 5\n";
        assert_eq!(inline_edit_text(src, 1), "set x $nn5");
    }

    #[test]
    fn tp_inline_substitutes_braced_var_reference() {
        // `${p}` is a complete reference and is substituted; `${pq}` is not.
        let src = "proc h {p} { set x ${p}${pq} }\nh 9\n";
        assert_eq!(inline_edit_text(src, 1), "set x 9${pq}");
    }

    // W213 unset -nocomplain action

    #[test]
    fn w213_emits_unset_nocomplain_action() {
        // Confirm the analyser emits W213 on `unset xs` inside a
        // proc where `xs` is possibly undefined, then verify the
        // provider surfaces the `-nocomplain` quick-fix.
        let src = "proc foo {} { unset xs }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W213),
            "expected W213 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let nocomplain = actions
            .iter()
            .find(|a| a.title == "Add '-nocomplain' to unset");
        assert!(nocomplain.is_some(), "expected quick-fix in {actions:?}");
        let act = nocomplain.unwrap();
        assert_eq!(act.edits.len(), 1);
        assert_eq!(act.edits[0].new_text, " -nocomplain");
        // The edit is an insertion (zero-width range).
        assert_eq!(
            act.edits[0].range.start_character,
            act.edits[0].range.end_character,
        );
    }

    #[test]
    fn w213_action_inserts_after_unset_keyword() {
        // Verify the insertion point is exactly after the 5
        // chars of `unset` — splicing produces a syntactically
        // correct `unset -nocomplain xs` command.
        let src = "proc foo {} { unset xs }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let act = actions
            .iter()
            .find(|a| a.title == "Add '-nocomplain' to unset")
            .expect("expected unset action");
        // Apply the edit and check the result.
        let edit = &act.edits[0];
        let line0 = src.lines().nth(edit.range.start_line as usize).unwrap();
        let chars: Vec<char> = line0.chars().collect();
        let col = edit.range.start_character as usize;
        let before: String = chars[..col].iter().collect();
        let after: String = chars[col..].iter().collect();
        let spliced = format!("{before}{}{after}", edit.new_text);
        assert!(
            spliced.contains("unset -nocomplain xs"),
            "spliced line: {spliced}",
        );
    }

    // catch-result-variable actions

    #[test]
    fn w302_emits_catch_result_variable_actions() {
        // The real analyser emits W302 for `catch {body}` with
        // no result variable.  The provider should surface two
        // synthetic actions appending ` result` / ` result opts`.
        let src = "catch { puts hi }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Sanity-check the analyser actually emitted W302.
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W302),
            "expected W302 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"Add catch result variable"), "{titles:?}");
        assert!(
            titles.contains(&"Add catch result + options variables"),
            "{titles:?}",
        );
        // Verify the insertion text shapes.
        let result_act = actions
            .iter()
            .find(|a| a.title == "Add catch result variable")
            .unwrap();
        assert_eq!(result_act.edits[0].new_text, " result");
        let opts_act = actions
            .iter()
            .find(|a| a.title == "Add catch result + options variables")
            .unwrap();
        assert_eq!(opts_act.edits[0].new_text, " result opts");
        // Both insertions land at the same position (a zero-
        // width range immediately after the body's closing `}`).
        for act in [result_act, opts_act] {
            let r = act.edits[0].range;
            assert_eq!(r.start_line, r.end_line);
            assert_eq!(r.start_character, r.end_character);
        }
    }

    // W120 package-require fix

    #[test]
    fn w120_surfaces_add_package_require_action() {
        // The analyser emits W120 (with a CodeFix) for a
        // package-gated command used without `package require`;
        // the code-actions provider lifts that fix into an
        // `Add 'package require ...'` action.
        let src = "tcl::idna decode example.com\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl9.0").clone();
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W120),
            "expected W120 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let add = actions
            .iter()
            .find(|a| a.title.starts_with("Add 'package require"));
        assert!(
            add.is_some(),
            "expected package-require action in {actions:?}"
        );
        let act = add.unwrap();
        assert_eq!(act.edits.len(), 1);
        assert_eq!(act.edits[0].new_text, "package require tcl::idna\n");
        // Insertion at the top of the file (line 0, col 0).
        assert_eq!(act.edits[0].range.start_line, 0);
        assert_eq!(act.edits[0].range.start_character, 0);
    }

    fn at(line: u32, character: u32) -> LspRange {
        LspRange {
            start_line: line,
            start_character: character,
            end_line: line,
            end_character: character,
        }
    }

    #[test]
    fn fuzzy_package_require_suggests_known_package() {
        // `http::foo` — the `http` prefix matches the `http` package
        // (http::geturl's required_package).
        let reg = tcl_registry::CommandRegistry::build_default();
        let actions = package_require_actions("http::foo $x\n", at(0, 2), &reg);
        assert!(
            actions
                .iter()
                .any(|a| a.title == "Add 'package require http'"),
            "{actions:?}"
        );
        // The edit inserts at the top of the file.
        let act = actions
            .iter()
            .find(|a| a.title.contains("http"))
            .expect("http action");
        assert_eq!(act.edits[0].new_text, "package require http\n");
        assert_eq!(act.edits[0].range.start_line, 0);
    }

    #[test]
    fn fuzzy_package_require_dedups_already_required() {
        // `http` is already required → no `http` suggestion, and the
        // insert line lands after the existing require.
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "package require http\nhttp::foo\n";
        let actions = package_require_actions(src, at(1, 2), &reg);
        assert!(
            !actions.iter().any(|a| a.title.contains("require http'")),
            "http already required: {actions:?}"
        );
    }

    #[test]
    fn fuzzy_package_require_ignores_short_prefix() {
        let reg = tcl_registry::CommandRegistry::build_default();
        assert!(package_require_actions("x::y\n", at(0, 0), &reg).is_empty());
    }

    #[test]
    fn rank_package_suggestions_orders_exact_before_prefix() {
        let pkgs = vec!["httpd".to_string(), "http".to_string(), "json".to_string()];
        let ranked = rank_package_suggestions("http::get", &pkgs, 5);
        // Exact (`http`, score 0) before prefix (`httpd`, score 1);
        // `json` doesn't match at all.
        assert_eq!(ranked, vec!["http", "httpd"]);
    }

    #[test]
    fn word_at_position_extracts_namespaced_word() {
        assert_eq!(word_at_position("http::foo $x\n", 0, 2), "http::foo");
        assert_eq!(word_at_position("  set y 1\n", 0, 3), "set");
    }

    // check_diagnostic_actions: IRULE5002/5004 flow-warning fixes

    #[test]
    fn check_actions_surface_irule5002_flow_fix() {
        // An unguarded `drop` fires IRULE5002 through the compiler-checks
        // pass (not the analyser's `AnalysisResult.diagnostics`), carrying an
        // "insert event disable all + return" fix.  The provider must lift it.
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;
        use tcl_lexer::LexerConfig;

        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.load_irules();
        let src = "when CLIENT_ACCEPTED { drop }\n";
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some("f5-irules"));
        let checks = run_all_checks(&cu, &registry, Some("f5-irules"));
        assert!(
            checks
                .iter()
                .any(|d| d.code == DiagCode::Irule5002 && !d.fixes.is_empty()),
            "expected an IRULE5002 check with a fix, got {checks:?}",
        );

        let none_disabled = std::collections::HashSet::new();
        let actions =
            check_diagnostic_actions(src, whole_document_range(src), &checks, &none_disabled);
        let fix = actions
            .iter()
            .find(|a| a.title == "Add 'event disable all' + 'return'");
        assert!(fix.is_some(), "expected IRULE5002 quick-fix in {actions:?}");
        let fix = fix.unwrap();
        assert_eq!(fix.kind, ActionKind::QuickFix);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "\n    event disable all\n    return");
        // Insertion (zero-width range) right after the `drop` command.
        assert_eq!(fix.edits[0].range.start_line, fix.edits[0].range.end_line);
        assert_eq!(
            fix.edits[0].range.start_character,
            fix.edits[0].range.end_character,
        );
    }

    #[test]
    fn check_actions_empty_without_fixes() {
        // Checks with no fixes (or an out-of-range diagnostic) yield nothing.
        let src = "set x 1\n";
        let none_disabled = std::collections::HashSet::new();
        assert!(
            check_diagnostic_actions(src, whole_document_range(src), &[], &none_disabled)
                .is_empty()
        );
    }

    #[test]
    fn check_actions_honour_disabled_codes() {
        // A check whose code is disabled (`tclLsp.diagnostics.IRULE5002 = false`)
        // must not offer its quick-fix — otherwise the lightbulb re-surfaces a
        // diagnostic the user turned off.
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;
        use tcl_lexer::LexerConfig;

        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.load_irules();
        let src = "when CLIENT_ACCEPTED { drop }\n";
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some("f5-irules"));
        let checks = run_all_checks(&cu, &registry, Some("f5-irules"));

        let mut disabled = std::collections::HashSet::new();
        disabled.insert("IRULE5002".to_string());
        let actions = check_diagnostic_actions(src, whole_document_range(src), &checks, &disabled);
        assert!(
            !actions
                .iter()
                .any(|a| a.title == "Add 'event disable all' + 'return'"),
            "disabled IRULE5002 must not offer a quick-fix, got {actions:?}",
        );
    }

    // check_diagnostic_actions: shimmer-family noqa-suppress action

    /// A shimmering `incr` on a String variable fires S100 with no
    /// `CodeFix` attached (there is no generally-safe automatic rewrite —
    /// see `build_shimmer_noqa_suppress_action`'s doc comment), so the
    /// provider must synthesise the suppress action itself rather than
    /// only lifting `diag.fixes`.
    #[test]
    fn check_actions_surface_shimmer_noqa_suppress_action() {
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;

        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "set x hello\nincr x\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let checks = run_all_checks(&cu, &registry, None);
        let s100 = checks
            .iter()
            .find(|d| d.code == DiagCode::S100)
            .unwrap_or_else(|| panic!("expected an S100 check, got {checks:?}"));
        assert!(
            s100.fixes.is_empty(),
            "S100 should carry no diag-level CodeFix — this test exercises the synthetic path"
        );

        let none_disabled = std::collections::HashSet::new();
        let actions =
            check_diagnostic_actions(src, whole_document_range(src), &checks, &none_disabled);
        let suppress = actions
            .iter()
            .find(|a| a.title == "Suppress S100 with a noqa comment");
        assert!(
            suppress.is_some(),
            "expected an S100 noqa-suppress action, got {actions:?}"
        );
        let suppress = suppress.unwrap();
        assert_eq!(suppress.kind, ActionKind::QuickFix);
        assert_eq!(suppress.edits.len(), 1);
        assert_eq!(suppress.edits[0].new_text, "# noqa: S100\n");
        // Insertion (zero-width range) at the start of the `incr x` line —
        // one line above where the current baseline text puts `incr x`.
        assert_eq!(suppress.edits[0].range.start_line, 1);
        assert_eq!(suppress.edits[0].range.start_character, 0);
        assert_eq!(
            suppress.edits[0].range.start_line,
            suppress.edits[0].range.end_line
        );
        assert_eq!(
            suppress.edits[0].range.start_character,
            suppress.edits[0].range.end_character,
        );
    }

    /// The suppress action's inserted line is indented to match the
    /// command's own indentation, not flush to column 0.
    #[test]
    fn check_actions_shimmer_noqa_suppress_matches_indentation() {
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;

        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "proc f {} {\n    set x hello\n    incr x\n}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let checks = run_all_checks(&cu, &registry, None);
        assert!(checks.iter().any(|d| d.code == DiagCode::S100));

        let none_disabled = std::collections::HashSet::new();
        let actions =
            check_diagnostic_actions(src, whole_document_range(src), &checks, &none_disabled);
        let suppress = actions
            .iter()
            .find(|a| a.title == "Suppress S100 with a noqa comment")
            .unwrap_or_else(|| panic!("expected suppress action, got {actions:?}"));
        assert_eq!(suppress.edits[0].new_text, "    # noqa: S100\n");
    }

    /// A disabled shimmer code must not offer the suppress action either —
    /// same "don't re-surface a hidden warning" rule as `IRULE5002` above.
    #[test]
    fn check_actions_shimmer_noqa_suppress_honours_disabled_codes() {
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;

        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "set x hello\nincr x\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let checks = run_all_checks(&cu, &registry, None);

        let mut disabled = std::collections::HashSet::new();
        disabled.insert("S100".to_string());
        let actions = check_diagnostic_actions(src, whole_document_range(src), &checks, &disabled);
        assert!(
            !actions.iter().any(|a| a.title.starts_with("Suppress S100")),
            "disabled S100 must not offer a suppress action, got {actions:?}",
        );
    }

    // refactor-engine dispatch (extract/inline var, if↔switch,
    //    switch→dict, extract-datagroup)

    fn analyse(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn extract_variable_surfaces_with_selection() {
        let src = "set x [string length $name]";
        let analysis = analyse(src);
        // Select the `[string length $name]` value (cols 6..27).
        let range = LspRange {
            start_line: 0,
            start_character: 6,
            end_line: 0,
            end_character: 27,
        };
        let actions = code_actions(src, range, Some(&analysis));
        assert!(
            actions.iter().any(|a| {
                a.kind == ActionKind::RefactorExtract && a.title.to_lowercase().contains("variable")
            }),
            "{actions:?}",
        );
    }

    #[test]
    fn if_to_switch_surfaces_at_cursor() {
        let src = "if {$x eq \"a\"} {\n    puts 1\n} elseif {$x eq \"b\"} {\n    puts 2\n}";
        let analysis = analyse(src);
        let cursor = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        };
        let actions = code_actions(src, cursor, Some(&analysis));
        assert!(
            actions
                .iter()
                .any(|a| a.title.to_lowercase().contains("switch")),
            "{actions:?}",
        );
    }

    #[test]
    fn extract_datagroup_surfaces_and_carries_definition() {
        let src = "if {$host eq \"a.com\"} {\n    pool web_pool\n} elseif {$host eq \"b.com\"} {\n    pool web_pool\n} elseif {$host eq \"c.com\"} {\n    pool web_pool\n}";
        let analysis = analyse(src);
        let cursor = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        };
        let actions = code_actions(src, cursor, Some(&analysis));
        let dg = actions
            .iter()
            .find(|a| a.title.to_lowercase().contains("data-group"))
            .expect("data-group action");
        let def = dg
            .data_group_definition
            .as_ref()
            .expect("data_group_definition payload");
        assert!(def.contains("ltm data-group internal"), "{def:?}");
        assert!(def.contains("type string"), "{def:?}");
    }
}
