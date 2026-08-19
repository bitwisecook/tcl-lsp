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
//! * Catch-result-variable actions — W302 (`catch` without result
//!   variable) carries insert `CodeFix`es that splice a trailing
//!   ` result` (or ` result options`) after the body's closing
//!   delimiter; the provider lifts them via the generic `diag.fixes`
//!   path.  The **anchor is the analyser's**, computed from the
//!   invocation's argument tokens: this provider must not re-derive an
//!   insertion point from the diagnostic's span, which covers only the
//!   command head (issue #1190).
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
//!   fuzzy-rank known package names (the registry's
//!   `required_package` / `tcllib_package` catalogue) against an
//!   unresolved command head's namespace prefix and offer
//!   `Add 'package require <pkg>'`.  Gated on two pieces of
//!   evidence — the cursor is inside a recorded command-invocation
//!   head, and an unknown-command (W123) diagnostic covers it — so
//!   it never fires on a comment, a string, an argument word, or a
//!   definition's name (issue #1191).
//!
//! Limitations:
//!
//! * [`package_require_actions`] derives its catalogue from
//!   the registry, so locally-installed-but-unregistered
//!   packages aren't suggested.  Applying one loads the package and
//!   runs its initialisation code — it is a
//!   [`FixSafety::BehaviourHardening`](tcl_compiler::analyser::FixSafety)-class
//!   change, never an unattended one.
//! * Cross-document refactors (move to file, split namespace)
//!   are not supported.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lexer::{LineIndex, Utf16Col};
use tcl_registry::events::{DataCollectionAction, EventRegistry};
use tcl_registry::registry_for_dialect;

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
    /// Why this action cannot be applied here, when it cannot.
    ///
    /// Lifted to LSP's `CodeAction.disabled.reason`, which the editor shows
    /// on a greyed-out menu entry.  A refactoring that finds its subject but
    /// cannot preserve behaviour reports *why* rather than disappearing: the
    /// user otherwise cannot tell "does not apply here" from "is broken".
    /// `edits` is empty whenever this is set.
    pub disabled: Option<String>,
}

impl CodeAction {
    /// Construct an applicable `CodeAction` with no `data_group_definition`.
    ///
    /// The common path: every action except the extract-to-datagroup
    /// refactor leaves the structured payload unset, and only a refusing
    /// refactoring sets `disabled`, so this keeps the call sites free of
    /// both fields.
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
            disabled: None,
        }
    }
}

/// Rewrite every newline an action's inserted text carries onto `line_ending`.
///
/// The action builders compose their inserted text with plain `\n` — a
/// docstring block, a `package require` line, a `# noqa` suppression, an
/// extracted `set` assignment.  Applied verbatim to a CRLF (or old-Mac)
/// document that silently mixes terminators into the file, so the server
/// resolves the document's own line ending
/// ([`crate::formatting::FormatterConfig::resolved_line_ending`]) and passes
/// it here before the actions go on the wire.
///
/// Any terminator already present in the text — a `\r\n` or lone `\r` copied
/// out of the source by a block-rewriting action — is folded to `\n` first, so
/// the result is uniform rather than doubled.  A `"\n"` line ending is the
/// no-op every LF document takes.
pub fn retarget_newlines(actions: &mut [CodeAction], line_ending: &str) {
    if line_ending == "\n" {
        return;
    }
    for action in actions {
        for edit in &mut action.edits {
            if !edit.new_text.contains('\n') && !edit.new_text.contains('\r') {
                continue;
            }
            edit.new_text = tcl_lexer::normalise_lone_cr(&edit.new_text)
                .replace("\r\n", "\n")
                .replace('\n', line_ending);
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
            disabled: None,
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
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
    line_index: &LineIndex,
) {
    for diag in diagnostics {
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
            disabled: None,
        });
    }
}

/// Compute code actions for `range` in `source`.
///
/// `analysis`, when `Some`, is the analyser result the caller
/// already computed.  When `None`, returns an empty vector
/// (preserves the stub call shape for callers that haven't
/// yet plumbed analysis through).
///
/// `diagnostics` is the **published** diagnostic set — the one the host has
/// decided this document actually shows, after whatever workspace refinement
/// it applies (the LSP server's package / auto-load / cross-file W120 and W123
/// passes).  It is a separate argument from `analysis` precisely because it is
/// *not* `analysis.diagnostics`: reading the analyser's raw set here is what
/// let the server offer a "did you mean 'ni'?" rewrite over a cross-file
/// `Pi()` call whose diagnostic it had already suppressed (issue #923 idx 80).
/// A host with no workspace knowledge passes `&analysis.diagnostics`, which is
/// then the same set by definition.
///
/// The "Generate docstring" source action is offered at the
/// [`crate::formatting::DocstringStyle::Preceding`] placement — the only
/// placement this entry point can offer, since it has no client config to
/// resolve a `tclLsp.formatting.docstringStyle` setting from. A host that
/// resolves the setting (the LSP server's `code_action` handler) should call
/// [`code_actions_in_program`] directly with the resolved style instead.
#[must_use]
pub fn code_actions(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<CodeAction> {
    code_actions_in_program(
        source,
        range,
        analysis,
        diagnostics,
        None,
        crate::formatting::DocstringStyle::Preceding,
    )
}

/// [`code_actions`] with the caller's whole-program export view attached —
/// the entry point a host with a workspace index should call.
///
/// The refactor engine's inline-proc transform substitutes the body of the
/// proc the call reaches, so a `namespace import -force` whose covering
/// `namespace export` lives in another file decides whether inlining the
/// local same-named proc is a refactor or a behaviour change (issue #1116
/// item 1).
///
/// `diagnostics` carries the same published-set meaning as in [`code_actions`]:
/// the two arguments answer different questions — `program` decides what a call
/// *reaches*, `diagnostics` decides what the document is *showing* — so a host
/// with a workspace index needs to supply both.
///
/// `docstring_style` is the resolved `tclLsp.formatting.docstringStyle`
/// setting (#1314): it decides where the "Generate docstring" source action
/// inserts a new stub (`Preceding` / `Body`), or suppresses the action
/// entirely (`None`).
#[must_use]
pub fn code_actions_in_program(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
    program: Option<crate::definition::ProgramExports<'_>>,
    docstring_style: crate::formatting::DocstringStyle,
) -> Vec<CodeAction> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();

    push_brace_expr_refactors(&mut actions, source, range, diagnostics, &line_index);

    for diag in diagnostics {
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
        // W302's catch-result-variable quick-fixes are carried on the
        // diagnostic, like W213's and W120's below.  This provider used to
        // synthesise them here from the diagnostic's *end* position, which is
        // the end of the `catch` **word** — the diagnostic anchors at the
        // command head, not at the body — so the inserted word landed before
        // the body and turned `catch {error oops}` into
        // `catch result {error oops}`, i.e. a catch of the script `result`
        // storing its message in a variable named `error` (issue #1190).
        // The analyser computes the anchor from the argument tokens instead,
        // and `lift_fixes` below surfaces it unchanged.
        //
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
        tcl_dialect::DialectProfile::by_name(&analysis.dialect),
    ));
    actions.extend(ip_conversion_actions(source, range, &line_index));
    actions.extend(expr_rewrite_actions(source, range, &line_index));
    actions.extend(docstring_actions(
        source,
        range,
        analysis,
        &line_index,
        docstring_style,
    ));
    actions.extend(extract_inline_actions(
        source,
        range,
        analysis,
        &line_index,
        program,
    ));

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
        disabled: None,
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
                disabled: None,
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
        disabled: None,
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

/// `package require` suggestions for an **unresolved, namespace-qualified
/// command head** the request range touches: when the head's leading
/// namespace names a package the registry knows, offer
/// `Add 'package require <pkg>'`.
///
/// # Why this needs evidence
///
/// Adding a `package require` is not a harmless suggestion.  Applying it
/// changes what the interpreter loads and runs the package's initialisation
/// code, so it must be offered only where there is real evidence a package is
/// missing.  The provider used to take whichever identifier-like word sat
/// under the cursor and fuzzy-match its prefix, with no notion of context at
/// all, so a cursor anywhere on `http::geturl` in *any* of these offered
/// `package require http` (issue #1191):
///
/// ```tcl
/// # Documentation: http::geturl
/// set example "http::geturl"
/// dict set docs command http::geturl
/// proc http::geturl {} {}
/// http::geturl
/// ```
///
/// The first four are data or a definition.  Only the last is a call — and
/// even it may be satisfied locally.
///
/// # The gates
///
/// All must hold, and each reads a fact the analyser or the registry already
/// computed rather than scanning text:
///
/// 1. **A proven command head.**  The request range must touch the head-token
///    span of a recorded command invocation
///    (`AnalysisResult::command_invocations`).  A comment, a quoted or braced
///    datum, an argument word, and a `proc` definition's *name* word are none
///    of them command heads, so none of them reach this.
/// 2. **A statically-written name.**  A computed head (`$cmd`, `[pick]`, an
///    `{*}`-expanded word) is recorded as an invocation, but its written text
///    is not the command that will run, so there is nothing to match a
///    package against.
/// 3. **The namespace names a package.**  The head's leading namespace
///    component must *exactly* match a package in the registry catalogue.
///    This replaces the containment ranking, which was the mechanism that
///    turned a passing textual resemblance into a suggestion to load code.
///    `json::write` in a file with no `package require json` is evidence;
///    `jsonify` is not.
/// 4. **Resolution finds nothing.**  The name must resolve to no registry
///    command and to no definition reachable from the call —
///    [`crate::definition::resolve_called_proc`] is the shared resolver
///    go-to-definition and find-references use, so it already accounts for
///    namespace visibility, `namespace import` (including `-force` shadows),
///    static `rename`, and `interp alias`.  A file carrying a dynamic package
///    provider (`AnalysisResult::has_dynamic_providers`) is skipped whole: a
///    computed `package require` / `load` may register the command at run
///    time, which is the same reason W123 stands down there.
/// 5. **The package is not already required.**
///
/// A command the registry *does* know but whose package is missing is W120's
/// business, not this provider's: W120 carries a precise registry-derived
/// insertion fix that the generic `diag.fixes` lift already surfaces.  This is
/// the recovery path for names the registry has never heard of.
///
/// `context_diagnostics` are the diagnostics the editor sent with the request.
/// An unknown-command diagnostic among them corroborates gate 4 when the
/// editor's view is fresher than the analysis in hand; it never substitutes
/// for gate 1.
///
/// # Limits
///
/// The catalogue comes from the registry's `required_package` /
/// `tcllib_package` fields, so a locally-installed but unregistered package is
/// never suggested.  A namespace matching a package name is strong evidence,
/// not proof that the package provides this particular command.
#[must_use]
pub fn package_require_actions(
    source: &str,
    range: LspRange,
    registry: &tcl_registry::CommandRegistry,
    analysis: Option<&AnalysisResult>,
    context_diagnostics: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    package_require_actions_in_program(
        source,
        range,
        crate::definition::CallResolution::document_only().with_registry(registry),
        analysis,
        context_diagnostics,
    )
}

/// [`package_require_actions`] with the caller's whole-program export view
/// attached — the entry point a host with a workspace index should call.
///
/// Gate 4 ("nothing answers to this head") runs the shared call resolver, so
/// it must run it with the same context go-to-definition uses or the two can
/// disagree about whether a call is satisfied (issue #1116 item 1).
///
/// In practice the `-force` shadow cannot change this provider's answer: gate
/// 3 only lets a *package-qualified* head through, and a `-force` import
/// rewrites the meaning of a **bare** name in the importing namespace. The
/// context is threaded anyway so the resolver call is not the odd one out.
#[must_use]
pub fn package_require_actions_in_program(
    source: &str,
    range: LspRange,
    resolution: crate::definition::CallResolution<'_>,
    analysis: Option<&AnalysisResult>,
    context_diagnostics: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    // No analysis means no evidence, and evidence is the whole gate.
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    if analysis.has_dynamic_providers {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let Some(package) = missing_package_for_head_at(
        source,
        range,
        resolution,
        analysis,
        context_diagnostics,
        &line_index,
    ) else {
        return Vec::new();
    };
    let insert_line = package_insert_line(source);
    vec![CodeAction {
        title: format!("Add 'package require {package}'"),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: insert_line,
                start_character: 0,
                end_line: insert_line,
                end_character: 0,
            },
            new_text: format!("package require {package}\n"),
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
        disabled: None,
    }]
}

/// The package a command head the request range touches appears to need, or
/// `None` when any of [`package_require_actions`]'s gates fails.
fn missing_package_for_head_at(
    source: &str,
    range: LspRange,
    resolution: crate::definition::CallResolution<'_>,
    analysis: &AnalysisResult,
    context_diagnostics: &[ContextDiagnostic],
    line_index: &LineIndex,
) -> Option<String> {
    let registry = resolution.registry?;
    let catalogue = package_catalogue(registry);
    for invocation in &analysis.command_invocations {
        let start = line_index.position_at_utf16(invocation.range.start(), source);
        let end = line_index.position_at_utf16(invocation.range.end(), source);
        let head_range = LspRange {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
        };
        // Gate 1: the range must touch this head.
        if !ranges_overlap(head_range, range) {
            continue;
        }
        // Gate 2: a statically-written name.
        if !is_static_command_name(&invocation.name) {
            continue;
        }
        // Gate 3 (cheap, so tried before the resolver): the leading namespace
        // component must name a catalogue package exactly.
        let Some(package) = package_named_by_namespace(&invocation.name, &catalogue) else {
            continue;
        };
        // Gate 4: nothing the registry or the workspace defines answers to
        // this name.  A corroborating unknown-command diagnostic from the
        // editor is accepted in place of the local resolver run, for the case
        // where the editor's view is fresher than the analysis in hand.
        if !head_is_unresolved(source, analysis, resolution, invocation)
            && !unresolved_diagnostic_covers(
                head_range,
                analysis,
                context_diagnostics,
                source,
                line_index,
            )
        {
            continue;
        }
        // Gate 5: the package is not already loaded.
        if already_required(source, &package) {
            continue;
        }
        return Some(package);
    }
    None
}

/// `true` when `name` is a command name written literally in the source —
/// the only shape a package suggestion can be matched against.
///
/// A head built at run time (`$cmd`, `[pick]`, an `{*}`-expanded word) is
/// still recorded as an invocation, but its *name* is whatever the caller
/// wrote, not the command that will run.
fn is_static_command_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('$')
        && !name.contains('[')
        && !name.contains('{')
        && !name.contains(char::is_whitespace)
}

/// The catalogue package whose name is exactly `head`'s leading namespace
/// component, ignoring case and any leading `::`.
///
/// A bare (unqualified) name has no namespace and therefore yields nothing:
/// `frobnicate` carries no evidence about which package might define it, and
/// guessing from a textual resemblance is the behaviour this replaced.  The
/// leading `::` is stripped first — the fully-qualified spelling is what
/// library code writes to be unambiguous, and splitting it on `::` without
/// stripping yields an empty component.
fn package_named_by_namespace(head: &str, catalogue: &[String]) -> Option<String> {
    let qualified = head.trim_start_matches("::");
    let (namespace, _rest) = qualified.split_once("::")?;
    if namespace.is_empty() {
        return None;
    }
    catalogue
        .iter()
        .find(|package| package.eq_ignore_ascii_case(namespace))
        .cloned()
}

/// `true` when nothing the registry or the workspace defines answers to this
/// invocation's head.
///
/// Delegates the definition half to [`crate::definition::resolve_called_proc`]
/// — the resolver go-to-definition, find-references, and the inline-proc
/// refactor share — so this provider cannot disagree with them about whether
/// a call is satisfied.  That resolver already understands namespace
/// visibility, `namespace import` (including `-force` shadowing), static
/// `rename`, and `interp alias`.
fn head_is_unresolved(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> bool {
    let Some(registry) = resolution.registry else {
        return false;
    };
    if registry.get(&invocation.name).is_some() {
        return false;
    }
    let head_off = invocation.range.start();
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        head_off,
        &analysis.namespace_overrides,
    );
    crate::definition::resolve_called_proc(
        analysis,
        source,
        &namespace,
        &invocation.name,
        head_off,
        resolution,
    )
    .is_none()
}

/// `true` when an unknown-command (W123) diagnostic covers `head_range`,
/// from either the analyser's own diagnostics or the ones the editor sent
/// with the request.
///
/// W123's emitter is the single place "does this command resolve?" is decided
/// for a bare name, and it already accounts for same-file and scoped
/// definitions, `namespace import`, static `rename` / `interp alias`, dynamic
/// providers, and a user-supplied `unknown` handler.  Accepting it as
/// corroboration keeps this provider from contradicting that answer.
fn unresolved_diagnostic_covers(
    head_range: LspRange,
    analysis: &AnalysisResult,
    context_diagnostics: &[ContextDiagnostic],
    source: &str,
    line_index: &LineIndex,
) -> bool {
    let from_analysis = analysis.diagnostics.iter().any(|diag| {
        if diag.code != DiagCode::W123 {
            return false;
        }
        let start = line_index.position_at_utf16(diag.span.start(), source);
        let end = line_index.position_at_utf16(diag.span.end(), source);
        ranges_overlap(
            LspRange {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
            },
            head_range,
        )
    });
    from_analysis
        || context_diagnostics.iter().any(|diag| {
            diag.code == DiagCode::W123.as_str() && ranges_overlap(diag.range, head_range)
        })
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

// W115 — convert a backslash-continued comment to per-line comments.

fn continuation_comment_actions(source: &str, range: LspRange, dialect: &'static tcl_dialect::DialectProfile) -> Vec<CodeAction> {
    // The shared W115 detector is also enough for clients that request source
    // actions without forwarding server diagnostics.
    let lines: Vec<&str> = source.split('\n').collect();
    let start_line = range.start_line as usize;
    if start_line >= lines.len() {
        return Vec::new();
    }
    let profile = dialect;
    let comments = tcl_compiler::analyser::utils::script_comment_facts(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        crate::registry_for_dialect_profile(profile),
    );
    let Some(block_end) =
        crate::source_style::comment_continuation_run_with_facts(&lines, &comments, start_line)
    else {
        return Vec::new();
    };
    // Gather the continuation run starting at `start_line`.
    let mut block: Vec<String> = Vec::new();
    for line in &lines[start_line..block_end] {
        let line = *line;
        let without_cr = line.trim_end_matches('\r');
        let continues = without_cr.ends_with('\\');
        // Strip the trailing backslash; preserve leading indentation.
        let body = if continues {
            without_cr[..without_cr.len() - 1].trim_end()
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
    }
    let new_text = block.join("\n");
    let end_line = block_end - 1;
    vec![CodeAction {
        title: "Convert to per-line comments".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: range.start_line,
                start_character: 0,
                end_line: u32::try_from(end_line).unwrap_or(range.start_line),
                // LSP columns are UTF-16 code units — use the line's UTF-16
                // length, not its codepoint count.
                end_character: char_col_to_utf16_local(
                    lines[end_line],
                    lines[end_line].chars().count(),
                ),
            },
            new_text,
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
        disabled: None,
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
        disabled: None,
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
            disabled: None,
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
            disabled: None,
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
    docstring_style: crate::formatting::DocstringStyle,
) -> Vec<CodeAction> {
    // `None` — "do not generate or reformat docstrings" — offers no action
    // at all, matching the setting's documented (and default) meaning.
    if docstring_style == crate::formatting::DocstringStyle::None {
        return Vec::new();
    }
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
        let edit = match docstring_style {
            crate::formatting::DocstringStyle::Body => {
                body_docstring_edit(source, line_index, proc_def)
            }
            // `Preceding` (and unreachable `None`, filtered above).
            _ => preceding_docstring_edit(decl.line),
        };
        // The DOXYGEN stub (`# @brief TODO: describe <proc>` + one `# @param`
        // line per parameter) is rendered by the shared docstring generator.
        let indent = edit.indent;
        let doc = crate::formatting::generate_stub_for_proc(
            proc_def,
            crate::formatting::DocstringTagStyle::Doxygen,
            false,
            '.',
            70,
            &indent,
        );
        out.push(CodeAction {
            title: format!("Generate docstring for '{}'", proc_def.name),
            edits: vec![crate::rename::TextEdit {
                range: edit.range,
                new_text: format!("{}{doc}{}", edit.prefix, edit.suffix),
            }],
            kind: ActionKind::Source,
            command: None,
            data_group_definition: None,
            disabled: None,
        });
    }
    out
}

/// Where + how to insert a generated docstring stub, and the indent its
/// lines should carry.
struct DocstringInsertion {
    range: LspRange,
    /// Text emitted before the rendered stub (e.g. nothing, or a leading
    /// newline when inserting mid-line).
    prefix: String,
    /// Text emitted after the rendered stub — a newline separating it from
    /// what follows, omitted when the insertion point is already followed
    /// by one (so `Body` placement never leaves a spurious blank line).
    suffix: String,
    indent: String,
}

/// [`crate::formatting::DocstringStyle::Preceding`]: insert a zero-indent
/// comment block on its own line directly above the `proc` declaration.
fn preceding_docstring_edit(decl_line: u32) -> DocstringInsertion {
    DocstringInsertion {
        range: LspRange {
            start_line: decl_line,
            start_character: 0,
            end_line: decl_line,
            end_character: 0,
        },
        prefix: String::new(),
        suffix: "\n".to_owned(),
        indent: String::new(),
    }
}

/// [`crate::formatting::DocstringStyle::Body`]: insert the comment block as
/// the first line inside the `proc` body, indented to match the body's
/// existing content (or four spaces — the formatter's default indent size —
/// when the body has no other indented line to match, e.g. an empty or
/// single-line proc).
fn body_docstring_edit(
    source: &str,
    line_index: &LineIndex,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> DocstringInsertion {
    let body_start = proc_def.body_span.start();
    let body_start_idx = body_start as usize;
    let body_end_idx = (proc_def.body_span.end() as usize).min(source.len());
    let body_text = source.get(body_start_idx..body_end_idx).unwrap_or("");
    // The opening `{` sits at `body_start`; the rest of that line is body
    // text too (a K&R `proc … {` puts nothing else there, but a
    // single-line proc does), so skip it and read the indent off the first
    // genuinely-new line instead.
    let indent = body_text
        .lines()
        .skip(1)
        .map(str::trim_start)
        .zip(body_text.lines().skip(1))
        .find(|(trimmed, _)| !trimmed.is_empty())
        .map_or_else(
            || "    ".to_owned(),
            |(trimmed, line)| line[..line.len() - trimmed.len()].to_owned(),
        );
    // The body already starts with its own newline (the common multi-line
    // shape) — reuse it rather than inserting a second one, which would
    // leave a blank line between the stub and the body's first statement.
    let suffix = if body_text.starts_with('\n') {
        String::new()
    } else {
        "\n".to_owned()
    };
    let pos = line_index.position_at_utf16(body_start, source);
    DocstringInsertion {
        range: LspRange {
            start_line: pos.line,
            start_character: pos.character.get(),
            end_line: pos.line,
            end_character: pos.character.get(),
        },
        prefix: "\n".to_owned(),
        suffix,
        indent,
    }
}

fn extract_inline_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    program: Option<crate::definition::ProgramExports<'_>>,
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
    out.extend(refactor_engine_actions(
        source,
        range,
        analysis,
        line_index,
        crate::definition::CallResolution {
            registry: Some(&registry),
            program,
        },
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
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<CodeAction> {
    use crate::refactor;
    let Some(registry) = resolution.registry else {
        return Vec::new();
    };
    let mut out = Vec::new();

    let cursor = line_index.offset_at_utf16(
        range.start_line,
        Utf16Col::new(range.start_character),
        source,
    );
    let has_selection =
        range.start_line != range.end_line || range.start_character != range.end_character;

    // Extract variable / extract proc — both require a selection.
    if has_selection {
        let end =
            line_index.offset_at_utf16(range.end_line, Utf16Col::new(range.end_character), source);
        if let Some(r) = refactor::extract_variable(source, cursor, end, "result", line_index) {
            out.push(refactoring_to_action(&r, source, line_index));
        }
        if let Some(r) = refactor::extract_proc(source, (cursor, end), analysis, registry) {
            // The generated proc name is a placeholder, so the applicable
            // form carries a follow-up rename command; a refused one does
            // not (there is nothing to rename).
            let mut action = refactoring_to_action(&r, source, line_index);
            action.command = refactor::extract_proc_rename_command(&r, source);
            out.push(action);
        }
    }

    if let Some(r) = refactor::inline_variable(source, cursor, analysis, registry, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if let Some(r) = refactor::inline_proc_in_program(source, cursor, analysis, resolution) {
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
/// converting its byte-offset edits to LSP coordinates, rendering the
/// data-group definition (if any) into `data_group_definition`, and carrying
/// a refusal reason through to the action's `disabled` field.
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
        disabled: r.disabled.clone(),
    }
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
    for ev in crate::irules_context::scan_file_events(source, tcl_dialect::DialectProfile::by_name("f5-irules")) {
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
            disabled: None,
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
        disabled: None,
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

/// Source text covered by a single-line diagnostic range.
///
/// Collection diagnostics point at one command or one `when` event token. A
/// range crossing lines is not a useful command selector, so return `None`
/// and deliberately offer no speculative quick fix in that case.
fn diagnostic_text_on_line(source: &str, range: &LspRange) -> Option<String> {
    if range.start_line != range.end_line {
        return None;
    }
    let line = source.split('\n').nth(range.start_line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let start = utf16_col_to_char_col(line, range.start_character).min(chars.len());
    let end = utf16_col_to_char_col(line, range.end_character).min(chars.len());
    (start < end).then(|| chars[start..end].iter().collect())
}

/// Registry-declared payload operation mentioned by a diagnostic range.
///
/// Most diagnostics cover the command head exactly. If a producer supplies a
/// wider statement span, scan its Tcl-shaped words and accept exactly one
/// registered payload command. This stays data-driven and avoids deriving
/// command behaviour from the diagnostic prose.
fn payload_operation_at_diagnostic(
    source: &str,
    range: &LspRange,
) -> Option<tcl_registry::DataCollectionOperation> {
    let text = diagnostic_text_on_line(source, range)?;
    let registry = registry_for_dialect("f5-irules");
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == ':' || ch == '_'))
        .filter(|word| !word.is_empty())
        .filter_map(|word| registry.data_collection_operation(word))
        .find(|operation| operation.action == DataCollectionAction::Payload)
}

/// Line index of the `when` block enclosing `line` (scanning upward), or 0.
fn enclosing_when_line(source: &str, line: u32) -> u32 {
    let index = tcl_lexer::LineIndex::new(source);
    tcl_irules::when_blocks(source)
        .into_iter()
        .filter(|block| {
            index.line_at(block.span.start()) <= line && line <= index.line_at(block.span.end())
        })
        .min_by_key(|block| block.span.end() - block.span.start())
        .map_or(0, |block| index.line_at(block.span.start()))
}

/// IRULE1005 / IRULE1006 "missing collect" quick-fixes: insert a bootstrap
/// handler using the registry-declared handler command and priority grammar.
///
/// The command name, whether collection is actually required, and the setup
/// event are all registry facts. The diagnostic message is presentation only;
/// it is never parsed to construct code.
fn collect_bootstrap_actions(source: &str, d: &ContextDiagnostic) -> Vec<CodeAction> {
    if d.code != "IRULE1005" && d.code != "IRULE1006" {
        return Vec::new();
    }
    let anchor = enclosing_when_line(source, d.range.start_line);
    let event =
        crate::irules_context::find_enclosing_when_event(source, d.range.start_line, tcl_dialect::DialectProfile::by_name("f5-irules"))
            .unwrap_or_default();

    let registry = registry_for_dialect("f5-irules");
    let events = EventRegistry::build();
    let Some(handler) = registry.event_handler_spec() else {
        return Vec::new();
    };
    let Some(priority) = handler.event_handler_priority else {
        return Vec::new();
    };
    let choices: Vec<(&str, &str)> = if d.code == "IRULE1005" {
        let Some(data_event) = diagnostic_text_on_line(source, &d.range) else {
            return Vec::new();
        };
        let data_event = data_event.to_ascii_uppercase();
        let Some(setup_event) = events
            .get_props(&data_event)
            .and_then(|props| props.setup_event)
        else {
            return Vec::new();
        };
        let Some((protocols, _)) = events.data_collect_requirement(&data_event) else {
            return Vec::new();
        };
        protocols
            .iter()
            .filter_map(|protocol| {
                registry
                    .data_collection_collect_command(protocol)
                    .map(|spec| (spec.name, setup_event))
            })
            .collect()
    } else {
        let Some(operation) = payload_operation_at_diagnostic(source, &d.range) else {
            return Vec::new();
        };
        let Some(setup_event) = operation.protocol.bootstrap_event_for(&event) else {
            return Vec::new();
        };
        registry
            .data_collection_collect_command(operation.protocol.name)
            .map(|spec| vec![(spec.name, setup_event)])
            .unwrap_or_default()
    };

    let mut unique: Vec<(&str, &str)> = Vec::new();
    for choice in choices {
        if !unique.contains(&choice) {
            unique.push(choice);
        }
    }
    unique
        .into_iter()
        .map(|(collect_command, setup)| CodeAction {
            title: format!("Add '{collect_command}' bootstrap in '{setup}'"),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: anchor,
                    start_character: 0,
                    end_line: anchor,
                    end_character: 0,
                },
                new_text: format!(
                    "{} {setup} {} {} {{\n    {collect_command}\n}}\n\n",
                    handler.name, priority.keyword, priority.default_priority,
                ),
            }],
            kind: ActionKind::QuickFix,
            command: None,
            data_group_definition: None,
            disabled: None,
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
        disabled: None,
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
        disabled: None,
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
        disabled: None,
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
        disabled: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult, CodeFix, Diagnostic};
    use tcl_lexer::Span;

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
        assert!(code_actions("set x 1\n", whole_document_range("set x 1\n"), None, &[]).is_empty());
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
                safety: tcl_compiler::analyser::FixSafety::RequiresReview,
            }],
        });
        let actions = code_actions(
            "set x 1\n",
            whole_document_range("set x 1\n"),
            Some(&r),
            &r.diagnostics,
        );
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
                safety: tcl_compiler::analyser::FixSafety::RequiresReview,
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
        assert!(code_actions("set x 1\n", far_range, Some(&r), &r.diagnostics).is_empty());
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
                safety: tcl_compiler::analyser::FixSafety::RequiresReview,
            }],
        });
        let actions = code_actions(
            "set x 1\n",
            whole_document_range("set x 1\n"),
            Some(&r),
            &r.diagnostics,
        );
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
            &analysis.diagnostics,
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
                    safety: tcl_compiler::analyser::FixSafety::RequiresReview,
                },
                CodeFix {
                    span: Span::new(0, 5),
                    new_text: "b".into(),
                    description: "B".into(),
                    safety: tcl_compiler::analyser::FixSafety::RequiresReview,
                },
            ],
        });
        let actions = code_actions(
            "set x 1\n",
            whole_document_range("set x 1\n"),
            Some(&r),
            &r.diagnostics,
        );
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

    /// The inline-proc action's replacement text, or its refusal reason.
    fn inline_outcome(src: &str, call_line: u32) -> Result<String, String> {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, "tcl8.6").clone();
        let actions = code_actions(
            src,
            line_range(call_line),
            Some(&analysis),
            &analysis.diagnostics,
        );
        let action = actions
            .iter()
            .find(|a| a.title.starts_with("Inline proc "))
            .unwrap_or_else(|| panic!("expected an inline action in {actions:?}"));
        match &action.disabled {
            Some(reason) => Err(reason.clone()),
            None => Ok(action.edits[0].new_text.clone()),
        }
    }

    #[test]
    fn fp_inline_refuses_a_braced_argument_that_is_not_a_plain_word() {
        // `f {a b}` passes the *value* `a b`, not the four characters
        // `{a b}`.  Splicing the written word makes the body print the
        // braces, so the transform declines and says why (issue #1199).
        let src = "proc f {p} { puts $p }\nf {a b}\n";
        let reason = inline_outcome(src, 1).unwrap_err();
        assert!(reason.contains("plain word"), "{reason}");
    }

    #[test]
    fn tp_inline_does_not_substitute_prefix_sharing_name() {
        // Param `n`; body reads `$nn` (a different variable) and `$n`. Only the
        // complete `$n` reference is replaced — `$nn` must survive intact,
        // where a naive `replace("$n", …)` would corrupt it (issue 179).
        let src = "proc g {n} { puts $nn$n }\ng 5\n";
        assert_eq!(inline_outcome(src, 1).unwrap(), "puts $nn5");
    }

    #[test]
    fn tp_inline_substitutes_braced_var_reference() {
        // `${p}` is a complete reference and is substituted; `${pq}` is not.
        let src = "proc h {p} { puts ${p}${pq} }\nh 9\n";
        assert_eq!(inline_outcome(src, 1).unwrap(), "puts 9${pq}");
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
        let actions = code_actions(
            src,
            whole_document_range(src),
            Some(&analysis),
            &analysis.diagnostics,
        );
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
        let actions = code_actions(
            src,
            whole_document_range(src),
            Some(&analysis),
            &analysis.diagnostics,
        );
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

    /// Apply a single-edit action's `TextEdit` to `src` and return the
    /// rewritten document.
    ///
    /// Every catch-fix test below asserts the *applied document* rather than
    /// the inserted string plus a zero-width range: the bug in issue #1190
    /// inserted exactly the right text at exactly the wrong place, and the
    /// old assertions (inserted text + "the range is zero-width") passed
    /// throughout.
    fn apply_single_edit(src: &str, action: &CodeAction) -> String {
        assert_eq!(action.edits.len(), 1, "expected one edit: {action:?}");
        let edit = &action.edits[0];
        let line_index = LineIndex::new(src);
        let start = line_index.offset_at_utf16(
            edit.range.start_line,
            Utf16Col::new(edit.range.start_character),
            src,
        ) as usize;
        let end = line_index.offset_at_utf16(
            edit.range.end_line,
            Utf16Col::new(edit.range.end_character),
            src,
        ) as usize;
        let mut out = src.to_string();
        out.replace_range(start..end, &edit.new_text);
        out
    }

    /// The quick-fix actions the provider offers for the W302 in `src`.
    fn catch_result_actions(src: &str) -> Vec<CodeAction> {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, "tcl9.0").clone();
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W302),
            "expected W302 from {:?}",
            analysis.diagnostics,
        );
        code_actions(
            src,
            whole_document_range(src),
            Some(&analysis),
            &analysis.diagnostics,
        )
        .into_iter()
        .filter(|action| action.title.starts_with("Add catch"))
        .collect()
    }

    #[test]
    fn w302_result_action_applies_after_the_body() {
        // The issue #1190 reproducer: the diagnostic anchors at the `catch`
        // word, so a provider that reconstructed the insertion point from
        // the diagnostic's end produced `catch result { puts hi }` — a catch
        // of the script `result`.
        let src = "catch { puts hi }\n";
        let actions = catch_result_actions(src);
        assert_eq!(actions.len(), 2, "{actions:?}");
        assert_eq!(
            apply_single_edit(src, &actions[0]),
            "catch { puts hi } result\n"
        );
    }

    #[test]
    fn w302_options_action_applies_after_the_body() {
        let src = "catch { puts hi }\n";
        let actions = catch_result_actions(src);
        assert_eq!(
            apply_single_edit(src, &actions[1]),
            "catch { puts hi } result options\n"
        );
    }

    #[test]
    fn w302_action_applies_after_a_multiline_body() {
        let src = "catch {\n    error oops\n}\n";
        let actions = catch_result_actions(src);
        assert_eq!(
            apply_single_edit(src, &actions[0]),
            "catch {\n    error oops\n} result\n"
        );
    }

    #[test]
    fn w302_action_applies_before_a_trailing_comment() {
        let src = "catch {error oops} ;# best effort\n";
        let actions = catch_result_actions(src);
        assert_eq!(
            apply_single_edit(src, &actions[0]),
            "catch {error oops} result ;# best effort\n"
        );
    }

    #[test]
    fn w302_action_does_not_overshoot_an_empty_body() {
        let src = "catch {}\n";
        let actions = catch_result_actions(src);
        assert_eq!(apply_single_edit(src, &actions[0]), "catch {} result\n");
    }

    #[test]
    fn w302_action_applies_to_a_nested_catch_on_a_later_line() {
        // A catch that is neither on line 0 nor at column 0 — the anchor is
        // an absolute offset from the argument token, not a line-relative
        // guess.
        let src = "proc f {} {\n    catch {error oops}\n}\n";
        let actions = catch_result_actions(src);
        assert_eq!(
            apply_single_edit(src, &actions[0]),
            "proc f {} {\n    catch {error oops} result\n}\n"
        );
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
        let actions = code_actions(
            src,
            whole_document_range(src),
            Some(&analysis),
            &analysis.diagnostics,
        );
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

    // Fuzzy `package require` suggestions — issue #1191.
    //
    // Applying one of these mutates package loading and runs the package's
    // initialisation code, so the provider must have evidence a package is
    // actually missing.  The two gates it applies are "the cursor is on a
    // recorded command *head*" and "resolution says that head is unresolved".
    // The cases below are the four coverage classes the issue asks for.

    /// The package-require actions for `src` at `range`, driven by a real
    /// analysis (the evidence both gates read) and no editor-supplied
    /// diagnostics.
    fn package_actions(src: &str, range: LspRange) -> Vec<CodeAction> {
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, "tcl9.0").clone();
        package_require_actions(src, range, &registry, Some(&analysis), &[])
    }

    /// The titles of the package-require actions for `src` at `range`.
    fn package_titles(src: &str, range: LspRange) -> Vec<String> {
        package_actions(src, range)
            .into_iter()
            .map(|action| action.title)
            .collect()
    }

    #[test]
    fn tp_package_require_offered_on_an_unresolved_command_head() {
        // The one shape that is real evidence: an executable command head
        // the resolver could not satisfy, whose leading namespace exactly
        // names a package the registry knows.
        let src = "http::foo $x\n";
        let actions = package_actions(src, at(0, 2));
        assert_eq!(
            actions.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(),
            vec!["Add 'package require http'"],
            "{actions:?}"
        );
        assert_eq!(actions[0].edits[0].new_text, "package require http\n");
        assert_eq!(actions[0].edits[0].range.start_line, 0);
    }

    #[test]
    fn fn_package_require_offered_on_a_fully_qualified_head() {
        // A leading `::` is how library code writes a call unambiguously.
        // Splitting on `::` without stripping it first yielded an empty
        // namespace component, so this shape used to match nothing at all.
        let titles = package_titles("::http::foo $x\n", at(0, 4));
        assert!(
            titles.iter().any(|t| t == "Add 'package require http'"),
            "{titles:?}"
        );
    }

    #[test]
    fn tn_package_require_not_offered_for_a_bare_unknown_command() {
        // `frobnicate` is unresolved, but its name carries no evidence about
        // which package would define it. Guessing from a textual resemblance
        // is exactly the behaviour the namespace gate replaced.
        assert!(package_titles("frobnicate 1\n", at(0, 2)).is_empty());
    }

    #[test]
    fn tn_package_require_not_offered_for_an_unregistered_namespace() {
        // `myapp` names no package in the registry catalogue, so there is
        // nothing evidence-backed to suggest — a project's own namespace
        // must never be read as a missing dependency.
        assert!(package_titles("myapp::helper x\n", at(0, 2)).is_empty());
    }

    #[test]
    fn tn_package_require_not_offered_under_a_dynamic_provider() {
        // A computed `package require` may register the command at run time,
        // which is the same reason W123 stands down in such a file.
        let src = "set p http\npackage require $p\nhttp::foo\n";
        assert!(package_titles(src, at(2, 2)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_inside_a_comment() {
        let src = "# Documentation: http::geturl\n";
        assert!(package_titles(src, at(0, 20)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_inside_a_quoted_string() {
        let src = "set example \"http::geturl\"\n";
        assert!(package_titles(src, at(0, 16)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_inside_a_braced_word() {
        let src = "set example {http::geturl}\n";
        assert!(package_titles(src, at(0, 16)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_on_an_argument_word() {
        // `http::geturl` here is data passed to `dict set`, not a call.
        let src = "dict set docs command http::geturl\n";
        assert!(package_titles(src, at(0, 25)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_on_a_definition_name() {
        // The name word of a `proc` is a definition, not an invocation — and
        // the file is defining the very command a package would provide.
        let src = "proc http::geturl {} {}\n";
        assert!(package_titles(src, at(0, 8)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_for_a_locally_defined_command() {
        // Defined above, called below: resolution settles the call, so no
        // unknown-command diagnostic covers the head and no package is
        // suggested for it.
        let src = "proc http::geturl {} {}\nhttp::geturl\n";
        assert!(package_titles(src, at(1, 2)).is_empty());
    }

    #[test]
    fn fp_package_require_not_offered_when_the_package_is_already_required() {
        let src = "package require http\nhttp::foo\n";
        assert!(
            !package_titles(src, at(1, 2))
                .iter()
                .any(|t| t.contains("require http'")),
            "http is already required"
        );
    }

    #[test]
    fn tn_package_require_not_offered_for_a_resolved_builtin() {
        // `puts` resolves in the registry, so nothing is unresolved here.
        assert!(package_titles("puts hello\n", at(0, 1)).is_empty());
    }

    #[test]
    fn tn_package_require_not_offered_for_a_dynamic_head() {
        // `$cmd` is recorded as an invocation, but its written name is not
        // the command that will run, so matching it against a package
        // catalogue is meaningless.
        assert!(package_titles("set cmd http::geturl\n$cmd\n", at(1, 1)).is_empty());
    }

    #[test]
    fn tn_package_require_not_offered_for_a_short_prefix() {
        assert!(package_titles("x::y\n", at(0, 0)).is_empty());
    }

    #[test]
    fn tn_package_require_not_offered_without_an_analysis() {
        // No analysis, no evidence — the provider declines rather than
        // falling back to scanning the text.
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(package_require_actions("http::foo\n", at(0, 2), &registry, None, &[]).is_empty());
    }

    #[test]
    fn package_require_offered_from_an_editor_supplied_diagnostic() {
        // The editor sends the diagnostics it is currently showing; a W123
        // among them is evidence even when the analysis in hand did not
        // re-emit it.  The head gate still applies, so this only works over
        // a real invocation.
        let src = "http::foo $x\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut analyser = Analyser::new();
        // Analyse a *different* document so the analysis carries no W123 for
        // this source, leaving the context diagnostic as the only evidence.
        let analysis = analyser.analyse(src, "tcl9.0").clone();
        let context = vec![ContextDiagnostic {
            code: "W123".to_string(),
            message: "Unknown command 'http::foo'".to_string(),
            range: at(0, 0),
        }];
        let actions = package_require_actions(src, at(0, 2), &registry, Some(&analysis), &context);
        assert!(
            actions
                .iter()
                .any(|a| a.title == "Add 'package require http'"),
            "{actions:?}"
        );
    }

    #[test]
    fn package_named_by_namespace_matches_exactly() {
        let catalogue = vec!["http".to_string(), "json".to_string()];
        assert_eq!(
            package_named_by_namespace("http::get", &catalogue),
            Some("http".to_string())
        );
        assert_eq!(
            package_named_by_namespace("::http::get", &catalogue),
            Some("http".to_string()),
            "a leading `::` must not swallow the namespace component"
        );
        // A near-miss is not evidence: `httpd::start` names the `httpd`
        // namespace, which is not the `http` package.
        assert_eq!(package_named_by_namespace("httpd::start", &catalogue), None);
        // Nor is a bare name, which has no namespace at all.
        assert_eq!(package_named_by_namespace("httpget", &catalogue), None);
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
        let profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some(profile));
        let checks = run_all_checks(&cu, &registry, Some(profile));
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
        let profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some(profile));
        let checks = run_all_checks(&cu, &registry, Some(profile));

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
        let actions = code_actions(src, range, Some(&analysis), &analysis.diagnostics);
        assert!(
            actions.iter().any(|a| {
                a.kind == ActionKind::RefactorExtract && a.title.to_lowercase().contains("variable")
            }),
            "{actions:?}",
        );
    }

    #[test]
    fn w115_action_ignores_braced_and_quoted_pseudo_comments() {
        for src in [
            "set payload {# pseudo \\\nputs live}\n",
            "set payload \"# pseudo \\\nputs live\"\n",
            "set marker \"noqa\"; # ordinary comment \\\nputs live\n",
        ] {
            let analysis = analyse(src);
            let actions = code_actions(
                src,
                whole_document_range(src),
                Some(&analysis),
                &analysis.diagnostics,
            );
            assert!(
                !actions
                    .iter()
                    .any(|action| action.title.contains("continued comment")),
                "pseudo-comment offered W115 action: {actions:?}"
            );
        }
    }

    #[test]
    fn retarget_newlines_rewrites_inserted_text_onto_the_documents_eol() {
        // The builders compose with `\n`; on a CRLF document every newline an
        // action inserts must be `\r\n`, or a "Generate docstring" / "Extract
        // into variable" quietly mixes terminators into the file.
        let src = "proc  p {a  b} {\r\nputs $b\r\n}\r\n";
        let analysis = analyse(src);
        let range = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 16,
        };
        let mut actions = code_actions(src, range, Some(&analysis), &analysis.diagnostics);
        assert!(
            actions
                .iter()
                .any(|a| a.edits.iter().any(|e| e.new_text.contains('\n'))),
            "fixture must produce at least one multi-line insertion: {actions:?}",
        );
        retarget_newlines(&mut actions, "\r\n");
        for action in &actions {
            for edit in &action.edits {
                assert!(
                    !edit.new_text.replace("\r\n", "").contains('\n'),
                    "{}: bare LF survived in {:?}",
                    action.title,
                    edit.new_text,
                );
                assert!(
                    !edit.new_text.replace("\r\n", "").contains('\r'),
                    "{}: bare CR survived in {:?}",
                    action.title,
                    edit.new_text,
                );
            }
        }
    }

    #[test]
    fn retarget_newlines_is_a_no_op_for_lf() {
        let mut actions = vec![CodeAction::new(
            "t".to_owned(),
            vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 0,
                    end_character: 0,
                },
                new_text: "a\nb\n".to_owned(),
            }],
            ActionKind::QuickFix,
            None,
        )];
        retarget_newlines(&mut actions, "\n");
        assert_eq!(actions[0].edits[0].new_text, "a\nb\n");
        // …and folds a mixed insertion onto a single terminator.
        retarget_newlines(&mut actions, "\r");
        assert_eq!(actions[0].edits[0].new_text, "a\rb\r");
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
        let actions = code_actions(src, cursor, Some(&analysis), &analysis.diagnostics);
        assert!(
            actions
                .iter()
                .any(|a| a.title.to_lowercase().contains("switch")),
            "{actions:?}",
        );
    }

    /// Issue #1000: the refactor code actions reach control flow inside an
    /// `apply` lambda body too.  `apply`'s literal is
    /// `ArgRole::LambdaLiteral`, so the descent has to split it rather than
    /// re-segment the whole `{argList body}` blob — which read `{m}` as a
    /// command name and left every `body_words`-backed action silently
    /// unavailable in there.
    #[test]
    fn if_to_switch_surfaces_inside_an_apply_lambda_body() {
        let src = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            puts get\n        } elseif {$m eq \"POST\"} {\n            puts post\n        }\n    }} $x\n}\n";
        let analysis = analyse(src);
        // Cursor on the `if` inside the lambda body (line 2, col 8).
        let cursor = LspRange {
            start_line: 2,
            start_character: 8,
            end_line: 2,
            end_character: 8,
        };
        let actions = code_actions(src, cursor, Some(&analysis), &analysis.diagnostics);
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
        let actions = code_actions(src, cursor, Some(&analysis), &analysis.diagnostics);
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
