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
//!   command head.
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
//!   definition's name.
//!
//! * Spec-pack did-you-mean actions ([`spec_pack_quick_fixes`]) — the
//!   `SpecTcl` loader drops a word it does not know and says so; the
//!   vocabulary it was measured against is closed, so the notice is a typo
//!   with one computable correction.
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

mod diagnostic_currency;
pub use diagnostic_currency::DiagnosticEditSource;

mod spec_notice;
pub use spec_notice::{SpecPackNoticeKind, SpecPackNoticeSubject};

mod diagnostic_context;
pub use diagnostic_context::ContextDiagnosticData;

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lexer::{LineIndex, Utf16Col};
use tcl_registry::events::{DataCollectionAction, EventRegistry};

use crate::definition::{LspRange, utf16_col_to_char_col};
use crate::diagnostic_policy::{Finding, FindingData, Fix, Report};

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

/// Lift every [`Fix`] a shown finding carries into a quick-fix
/// [`CodeAction`] — one action per fix, with the fix's own `(span, new_text)`
/// as a single-edit workspace edit.  The title is the fix's `description`,
/// falling back to the (truncated) finding message when the emitter supplied
/// none.
///
/// The analyser and the compiler-checks pass share one fix shape, so both
/// fixes-bearing families lift through this helper.
fn lift_fixes(
    actions: &mut Vec<CodeAction>,
    fixes: &[Fix],
    diag_message: &str,
    current: &DiagnosticEditSource<'_>,
    line_index: &LineIndex,
) {
    let source = current.source();
    for fix in fixes {
        if !current.contains_span(fix.span) {
            continue;
        }
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
/// whenever the request range touches a line carrying a *shown* unbraced-expr
/// (W100) finding, which corresponds to the `expr` command at the cursor.
/// Keyed on *line* overlap rather than the
/// finding's argument span so it is available with the cursor on the `expr`
/// keyword itself (VS Code invokes refactors at the caret, e.g. column 0), not
/// only over the arguments.  Reuses the finding's own brace-wrapping fix, so
/// `expr $a + $b` rewrites to `expr {$a + $b}`.
fn push_brace_expr_refactors(
    actions: &mut Vec<CodeAction>,
    source: &str,
    range: LspRange,
    report: &Report,
    current: &DiagnosticEditSource<'_>,
    line_index: &LineIndex,
) {
    for shown in report.shown() {
        if !current.matches_finding(shown.finding) || shown.finding.code != DiagCode::W100 {
            continue;
        }
        let Some(fix) = shown.finding.fixes.first() else {
            continue;
        };
        if !current.contains_span(fix.span) {
            continue;
        }
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

/// The optimiser rewrites the report may offer, as quick-fixes: one per
/// applicable rewrite ([`Report::applicable_rewrites`]) any member of which
/// overlaps `range`, titled with the first member's message and carrying
/// every member's edit. A grouped rewrite is one action with the whole
/// group's edits, and a group that lost a member to the policy is not
/// offered at all — O127's inline without its delete runs the assignment
/// twice (#2149). A `hint_only` rewrite, whose span covers the consuming
/// statement rather than a precise sub-span, is informational and never
/// offered, exactly as it never rides a published diagnostic's payload.
fn rewrite_actions(
    report: &Report,
    source: &str,
    range: LspRange,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    let lsp_range = |finding: &Finding| {
        let start = line_index.position_at_utf16(finding.span.start(), source);
        let end = line_index.position_at_utf16(finding.span.end(), source);
        LspRange {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
        }
    };
    report
        .applicable_rewrites()
        .into_iter()
        .filter(|rewrite| {
            rewrite
                .members
                .iter()
                .any(|member| ranges_overlap(lsp_range(member), range))
        })
        .filter_map(|rewrite| {
            let edits: Vec<crate::rename::TextEdit> = rewrite
                .members
                .iter()
                .filter_map(|member| match &member.data {
                    Some(FindingData::Rewrite { replacement, .. }) => {
                        Some(crate::rename::TextEdit {
                            range: lsp_range(member),
                            new_text: replacement.clone(),
                        })
                    }
                    _ => None,
                })
                .collect();
            Some(CodeAction {
                title: rewrite.members.first()?.message.clone(),
                edits,
                kind: ActionKind::QuickFix,
                command: None,
                data_group_definition: None,
                disabled: None,
            })
        })
        .collect()
}

/// Compute code actions for `range` in `source`.
///
/// `analysis`, when `Some`, is the analyser result the caller
/// already computed.  When `None`, returns an empty vector
/// (preserves the stub call shape for callers that haven't
/// yet plumbed analysis through).
///
/// `report` is the document's findings under its policy
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters): a fix is lifted
/// from a finding the report **shows** and from no other, whichever producer
/// emitted it — the analyser, the compiler checks, or the optimiser, whose
/// shown rewrites are offered as quick-fixes here.  Reading a producer's raw
/// set instead is what let a host offer a "did you mean 'ni'?" rewrite over
/// a cross-file `Pi()` call whose diagnostic it had already suppressed, and
/// what let the MCP tool offer a fix an inline `# noqa` silences.
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
    report: &Report,
) -> Vec<CodeAction> {
    code_actions_in_program(
        source,
        range,
        analysis,
        report,
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
/// local same-named proc is a refactor or a behaviour change.
///
/// `report` carries the same meaning as in [`code_actions`]: the two
/// arguments answer different questions — `program` decides what a call
/// *reaches*, `report` decides what the document is *showing* — so a host
/// with a workspace index needs to supply both.
///
/// `docstring_style` is the resolved `tclLsp.formatting.docstringStyle`
/// setting: it decides where the "Generate docstring" source action
/// inserts a new stub (`Preceding` / `Body`), or suppresses the action
/// entirely (`None`).
#[must_use]
pub fn code_actions_in_program(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    report: &Report,
    program: Option<crate::definition::ProgramExports<'_>>,
    docstring_style: crate::formatting::DocstringStyle,
) -> Vec<CodeAction> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    // Implementation contract: naming.editor.original-diagnostic-edit-currency
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
    let Some(current) = DiagnosticEditSource::for_analysis(source, analysis) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();

    push_brace_expr_refactors(&mut actions, source, range, report, &current, &line_index);

    for shown in report.shown() {
        let finding = shown.finding;
        if !current.matches_finding(finding) {
            continue;
        }
        let diag_start = line_index.position_at_utf16(finding.span.start(), source);
        let diag_end = line_index.position_at_utf16(finding.span.end(), source);
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
        // finding, like W213's and W120's.  Synthesising them here
        // from the finding's *end* position would use the end of the
        // `catch` **word** — the finding anchors at the command head, not
        // at the body — so the inserted word would land before
        // the body and turn `catch {error oops}` into
        // `catch result {error oops}`, i.e. a catch of the script `result`
        // storing its message in a variable named `error`.
        // The analyser computes the anchor from the argument tokens instead,
        // and `lift_fixes` below surfaces it unchanged.
        //
        // W213's `Add '-nocomplain' to unset` quick-fix is carried on the
        // finding itself (the analyser knows the exact `unset` keyword span
        // and narrows the finding to the offending variable word), so it is
        // surfaced by the generic `lift_fixes` path rather than re-derived
        // here from the span.
        //
        // The compiler checks' fixes (the iRules control-flow insertions,
        // taint-family rewrites, the W201 `file join` rewrite among them)
        // lift through the same call: the checks and the analyser are
        // disjoint families, so no fix is offered twice.
        lift_fixes(
            &mut actions,
            &finding.fixes,
            &finding.message,
            &current,
            &line_index,
        );
        if is_shimmer_family(finding.code)
            && let Some(action) = build_shimmer_noqa_suppress_action(source, finding, &line_index)
        {
            actions.push(action);
        }
    }
    actions.extend(rewrite_actions(report, source, range, &line_index));

    // Range-based refactors / source actions that don't depend on a diagnostic.
    actions.extend(continuation_comment_actions(
        source,
        range,
        analysis,
        report,
        &line_index,
    ));
    actions.extend(ip_conversion_actions(source, range, &line_index));
    actions.extend(expr_rewrite_actions(source, range, analysis, &line_index));
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
/// The caller captures [`DiagnosticEditSource`] from its actual current analysis
/// and passes the `run_all_checks` output (e.g. `CompilerDiagnostics::checks`).
/// Each check must independently retain matching issuer source/config/Registry;
/// absent provenance supplies no quick-fix or suppression edit.
///
/// `disabled` is the resolved per-check toggle set
/// (`tclLsp.diagnostics.<CODE> = false`) and `suppressed` the analyser's
/// `# noqa` / `# tcl-lsp: disable=…` map.  A check silenced by either has no
/// diagnostic in the published set, so its quick-fix must not be offered
/// either — otherwise the lightbulb re-surfaces a hidden warning, and a
/// shimmer code would offer to add a second `# noqa` above the one already
/// silencing it.  The analyser path bakes the disabled set into its build and
/// has its suppression applied by the caller; this path is fed the raw
/// `run_all_checks` output, so it applies both filters here.
#[must_use]
pub fn check_diagnostic_actions<S: std::hash::BuildHasher, H: BuildHasher, I: BuildHasher>(
    current: &DiagnosticEditSource<'_>,
    range: LspRange,
    checks: &[tcl_compiler::compiler_checks::Diagnostic],
    disabled: &std::collections::HashSet<String, S>,
    suppressed: &HashMap<i32, HashSet<String, I>, H>,
) -> Vec<CodeAction> {
    // Implementation contract: naming.editor.original-diagnostic-edit-currency
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
    let source = current.source();
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();
    for diag in checks {
        if !current.matches_compiler_diagnostic(diag) {
            continue;
        }
        if disabled.contains(diag.code.as_str()) {
            continue;
        }
        let diag_start = line_index.position_at_utf16(diag.span.start(), source);
        if line_suppressed(
            diag.code.as_str(),
            i32::try_from(diag_start.line).unwrap_or(i32::MAX),
            suppressed,
        ) {
            continue;
        }
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
        let finding = Finding::from(diag.clone());
        lift_fixes(
            &mut actions,
            &finding.fixes,
            &finding.message,
            current,
            &line_index,
        );
        if is_shimmer_family(diag.code)
            && let Some(action) = build_shimmer_noqa_suppress_action(source, &finding, &line_index)
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
    finding: &Finding,
    line_index: &LineIndex,
) -> Option<CodeAction> {
    let line = line_index.line_at(finding.span.start());
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
        title: format!("Suppress {} with a noqa comment", finding.code.as_str()),
        edits: vec![crate::rename::TextEdit {
            range: insertion,
            new_text: format!("{indent}# noqa: {}\n", finding.code.as_str()),
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

/// Reviewed package-loading suggestions for an original unresolved,
/// namespace-qualified command head touched by the request. The actual full
/// context owns the catalogue, and matching a namespace to package metadata
/// supplies a suggestion only: it establishes neither installation nor which
/// commands that package will provide.
///
/// Native advice requires the genuine emitting W123 subject and its original
/// static lookup. Explicit Logical advice requires its positively sealed whole
/// source vector, shared positioned absence and whole-program declarations.
/// Dynamic providers, unsupported original names and existing selected source
/// requirements/provisions withdraw advice. External diagnostic prose supplies
/// no substitute for the original owner. Insertion geometry comes from the same
/// root-source owner as W120, preserving split commands and input grammar.
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

/// Package suggestions under the caller's whole-program declaration view.
/// Workspace declarations may withdraw source advice; they cannot establish a
/// Native slot, original source producer or package installation.
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
    if analysis.has_dynamic_providers
        || analysis.body_lexer_config.is_none_or(|config| {
            !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        })
    {
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
    let Some(insert_offset) =
        tcl_compiler::registry_invocation::source_structure::original_package_require_insert_offset(
            source, analysis,
        )
    else {
        return Vec::new();
    };
    let insertion = line_index.position_at_utf16(insert_offset, source);
    let separator = if usize::try_from(insert_offset).ok() == Some(source.len())
        && !source.is_empty()
        && !source.ends_with('\n')
    {
        "\n"
    } else {
        ""
    };
    vec![CodeAction {
        title: format!("Add 'package require {package}'"),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: insertion.line,
                start_character: insertion.character.get(),
                end_line: insertion.line,
                end_character: insertion.character.get(),
            },
            new_text: format!("{separator}package require {package}\n"),
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
    _context_diagnostics: &[ContextDiagnostic],
    line_index: &LineIndex,
) -> Option<String> {
    resolution.registry?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let catalogue = package_catalogue(&context);
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
        if let Some(input) = invocation.original_name_input.as_ref() {
            let Some(key) = input.original_word_key() else {
                continue;
            };
            let Some(lookup) = invocation.original_lookup.as_ref().filter(|lookup| {
                lookup.name_input() == input
                    && lookup.site().source.source_image() == key.source_image()
                    && lookup.site().offset == invocation.range.start()
            }) else {
                continue;
            };
            if !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                input,
                invocation.range,
            ) || invocation.resolved_command_reference.is_some()
                || !analysis.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == DiagCode::W123
                        && diagnostic.span == invocation.range
                        && diagnostic.unresolved_command().is_some_and(|subject| {
                            subject.name_input() == key
                                && subject
                                    .invocation()
                                    .original_static_command_lookup(key)
                                    .as_ref()
                                    == Some(lookup)
                        })
                })
            {
                continue;
            }
            let Some(package) = package_named_by_original_input(input, &catalogue) else {
                continue;
            };
            let selected = tcl_registry::native_package::NativePackageNameKey::from_native_units(
                package.as_bytes(),
                input.policy(),
            );
            if original_package_already_required(source, analysis, &selected) != Some(false)
                || selected.manifest_atom(analysis.body_lexer_config?) != Some(package.clone())
            {
                continue;
            }
            return Some(package);
        }
        // A missing Native producer cannot enter explicit Logical advice.
        let Some(offset) = invocation.lookup.offset(invocation.range) else {
            continue;
        };
        let Some(words) =
            tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
                source, analysis, offset,
            )
        else {
            continue;
        };
        let Some(head) = words.first().and_then(diagnostic_context::source_literal) else {
            continue;
        };
        if !is_static_command_name(head) {
            continue;
        }
        let Some(package) = package_named_by_namespace(head, &catalogue) else {
            continue;
        };
        if !head_is_unresolved(source, analysis, resolution, invocation, head)
            || original_source_package_already_named(source, analysis, &package) != Some(false)
            || !package_source_atom(&package)
        {
            continue;
        }
        return Some(package);
    }
    None
}

// Catalogue names are authored metadata. Comparing their bytes supplies a
// package hint only; it does not establish a package loader or callable head.
fn package_named_by_original_input(
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    catalogue: &[String],
) -> Option<String> {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};
    match input.policy().recipe() {
        protocol @ NativeNameProtocol::C(_) => {
            let slot = protocol
                .command_lookup_slot(NativeNameContext::root(), input.bytes())
                .ok()?;
            let namespace = slot.namespace.as_segments().first()?.as_bytes();
            catalogue
                .iter()
                .find(|package| package.is_ascii() && package.as_bytes() == namespace)
                .cloned()
        }
        protocol @ NativeNameProtocol::Jim084 => {
            // Jim's flat command key does not become a C namespace path.
            let keys = protocol
                .jim_command_lookup_keys(NativeNameContext::root(), input.bytes())
                .ok()?;
            let key = keys.first()?.as_bytes();
            catalogue
                .iter()
                .find(|package| {
                    package.is_ascii()
                        && key
                            .strip_prefix(package.as_bytes())
                            .is_some_and(|tail| tail.starts_with(b"::"))
                })
                .cloned()
        }
    }
}

fn original_package_already_required(
    source: &str,
    analysis: &AnalysisResult,
    selected: &tcl_registry::native_package::NativePackageNameKey,
) -> Option<bool> {
    let metadata = std::str::from_utf8(selected.bytes()).ok()?;
    metadata.is_ascii().then_some(())?;
    original_source_package_already_named(source, analysis, metadata)
}

/// Authentic selected package syntax, not line prefixes or signature labels.
fn original_source_package_already_named(
    source: &str,
    analysis: &AnalysisResult,
    package: &str,
) -> Option<bool> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let config = analysis.body_lexer_config?;
    analysis
        .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        .then_some(())?;
    for invocation in &analysis.command_invocations {
        if invocation.is_mathfunc_call
            || invocation.existence_probe
            || invocation.lookup
                != tcl_compiler::signature_scan::types::SignatureCommandLookup::InvocationHead
        {
            continue;
        }
        let Some(offset) = invocation.lookup.offset(invocation.range) else {
            continue;
        };
        let Some(words) =
            tcl_compiler::registry_invocation::source_structure::source_registry_words_at(
                source, analysis, offset,
            )
        else {
            continue;
        };
        if words
            .package_reference(&context)
            .is_some_and(|reference| reference.matches_ascii(package))
        {
            return Some(true);
        }
    }
    Some(false)
}

fn package_source_atom(package: &str) -> bool {
    package.is_ascii()
        && !package.is_empty()
        && !package.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte.is_ascii_whitespace()
                || matches!(
                    byte,
                    b';' | b'$' | b'[' | b']' | b'{' | b'}' | b'"' | b'\\' | b'#'
                )
        })
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

/// Positioned absence from the shared source owner, followed by the same
/// whole-program declaration resolver used by definition navigation. Registry
/// descriptor names and external diagnostic prose cannot establish absence.
fn head_is_unresolved(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    head: &str,
) -> bool {
    let Some(offset) = invocation.lookup.offset(invocation.range) else {
        return false;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    if realm.diagnostic_slot_presence_at(offset)
        != tcl_compiler::command_binding::SourceCommandSlotPresence::Absent
    {
        return false;
    }
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        offset,
        &analysis.namespace_overrides,
    );
    crate::definition::resolve_called_proc(analysis, source, &namespace, head, offset, resolution)
        .is_none()
}

/// Authored package suggestions from the actual full current context/store.
fn package_catalogue(context: &tcl_registry::model::ContextRegistry) -> Vec<String> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let registry = context.commands();
    let mut names = std::collections::BTreeSet::new();
    for name in registry.command_names_in_any_dialect() {
        if let Some(spec) = context.context().resolve_spec(registry, name) {
            names.extend(
                spec.required_package
                    .into_iter()
                    .chain(spec.tcllib_package)
                    .map(str::to_owned),
            );
        }
    }
    names.into_iter().collect()
}

// W115 — convert a backslash-continued comment to per-line comments.

/// Offered only where a *shown* W115 overlaps `range` (§ Adapters, code
/// actions: "A fix is offered for a shown finding and for no other") — a
/// W115 turned off at any scope, or silenced by a directive, offers no
/// conversion, even though the comment shape below is still detectable.
fn continuation_comment_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    report: &Report,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    let finding_range = |finding: &Finding| {
        let start = line_index.position_at_utf16(finding.span.start(), source);
        let end = line_index.position_at_utf16(finding.span.end(), source);
        LspRange {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
        }
    };
    let shown_w115_overlaps = report.shown().any(|shown| {
        shown.finding.code == DiagCode::W115 && ranges_overlap(finding_range(shown.finding), range)
    });
    if !shown_w115_overlaps {
        return Vec::new();
    }
    let lines: Vec<&str> = source.split('\n').collect();
    let start_line = range.start_line as usize;
    if start_line >= lines.len() {
        return Vec::new();
    }
    let Some(comments) = crate::source_style::comment_facts_from_analysis(source, analysis) else {
        return Vec::new();
    };
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

fn expr_rewrite_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    // Single-line, non-empty selection only.
    if range.start_line != range.end_line || range.start_character >= range.end_character {
        return Vec::new();
    }
    let start = crate::definition::byte_offset_at(
        line_index,
        source,
        range.start_line,
        range.start_character,
    );
    let finish =
        crate::definition::byte_offset_at(line_index, source, range.end_line, range.end_character);
    let Some(operand) = crate::expr_context::source_expression_operand_at(source, analysis, start)
    else {
        return Vec::new();
    };
    let Some(extent) = operand.content_span() else {
        return Vec::new();
    };
    if start < extent.start() || finish > extent.end() {
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
    if !analysis.allows_retained_logical_declaration_advice() {
        for action in &mut out {
            action.edits.clear();
            action.disabled = Some("missing-expression-operator-rewrite-permission: Original expression roles provide source geometry; Native or hosted operator rewrites require an independent selected evaluation and replacement contract".to_owned());
        }
    }
    out
}

/// De Morgan: `!(X && Y)` ↔ `!X || !Y`, `!(X || Y)` ↔ `!X && !Y` — plus the
/// iRules word-operator equivalents (`not`/`and`/`or`, i.e.
/// `UnaryOp::WordNot`/`BinOp::WordAnd`/`BinOp::WordOr`). Recognising only
/// the symbolic forms would never offer the rewrite for a selection written
/// in iRules' word style (`!($a and $b)`) — an inconsistent gap given the
/// sibling `invert_comparison` rewrite in this same file handles TIP 461's
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
/// from `BinOp::inverse()` (`tcl_syntax::expr::operators`) rather than a
/// hand-typed list, which would miss the TIP 461 string-ordering four
/// (`lt`/`le`/`gt`/`ge`) and never offer the "Invert comparison" quick fix
/// for a selection containing one. Order doesn't matter for correctness — each
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
    let Some(declarations) = crate::procedure_symbol::declarations(source, analysis) else {
        return out;
    };
    for proc_def in declarations {
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
                if !original_literal_procedure_body(source, analysis, proc_def) {
                    continue;
                }
                body_docstring_edit(source, line_index, proc_def)
            }
            // `Preceding` (and unreachable `None`, filtered above).
            _ => preceding_docstring_edit(decl.line),
        };
        // The DOXYGEN stub (a `# @brief` placeholder plus one `# @param`
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

// Editing inside a body needs its actual grouped word. The declaration
// inventory supplies the selected ProcDef; body text/ranges alone cannot turn
// a computed script into writable original source.
fn original_literal_procedure_body(
    source: &str,
    analysis: &AnalysisResult,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> bool {
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let image = tcl_lexer::SourceImage::document(source);
    let Some(input) = analysis.retained_command_realm().and_then(|realm| {
        realm.original_written_name_input_at_span_in_source(&image, proc_def.body_span, config)
    }) else {
        return false;
    };
    let Some(key) = input.original_word_key() else {
        return false;
    };
    let word = key.original_word();
    word.image() == &image
        && word.config() == config
        && word.group().kind == tcl_lexer::WordKind::Braced
        && !word.group().expand
        && word
            .tokens()
            .first()
            .is_some_and(|token| token.span == proc_def.body_span)
        && word.content_span().is_ok()
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
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    out.extend(refactor_engine_actions(
        source,
        range,
        analysis,
        line_index,
        crate::definition::CallResolution {
            registry: Some(registry),
            program,
        },
    ));
    out
}

/// Surface the [`crate::refactor`] transforms (extract / inline variable,
/// if↔switch, switch→dict, extract-to-datagroup) as `CodeAction`s.
///
/// The cursor is `range`'s start; extract-variable additionally needs a
/// non-empty selection. Data-group extraction remains explicit lexical
/// authoring advice and requires the selected Registry's output forms.
/// Native source geometry supplies no insertion or movement permission.
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

    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return Vec::new();
    }

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
        if let Some(r) =
            refactor::extract_variable(source, (cursor, end), "result", analysis, line_index)
        {
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
    if let Some(r) = refactor::if_to_switch(source, cursor, analysis, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if let Some(r) = refactor::switch_to_dict(source, cursor, analysis, line_index) {
        out.push(refactoring_to_action(&r, source, line_index));
    }
    if analysis.allows_retained_logical_declaration_advice()
        && let Some(r) =
            refactor::extract_to_datagroup(source, cursor, "", registry, line_index, config)
    {
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
    let Some(commands) = diagnostic_context::current_commands(source, analysis, registry) else {
        return Vec::new();
    };
    let events = tcl_registry::events::EventRegistry::build();
    for (_, event) in selected_event_handlers(source, analysis, registry, &commands) {
        if let Some(props) = events.get_props(event) {
            for profile in props.implied_profiles {
                profiles.insert((*profile).to_owned());
            }
        }
    }
    let Some(profile) = analysis.resolved_profile() else {
        return Vec::new();
    };
    if !profile.is_irules() {
        return Vec::new();
    }
    for command in commands {
        if let Some(spec) = source_action_spec(analysis, registry, &command.canonical)
            && let Some(requirement) = spec.event_requires.as_ref()
        {
            for profile in requirement.profiles {
                profiles.insert((*profile).to_owned());
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
    /// Human-readable message, with no identity or edit authority.
    pub message: String,
    /// Actual compiler-issued source subject, when preserved by the publisher.
    pub data: Option<ContextDiagnosticData>,
    /// The diagnostic's range.
    pub range: LspRange,
}

const HTML_ENCODE_PROC: &str =
    "proc html_encode {str} { string map {& &amp; < &lt; > &gt; \\\" &quot; ' &#39;} $str }";
/// The `regex::quote` helper the T103 fix inserts when the file does not
/// already define it.
///
/// The `namespace eval` line is load-bearing: Tcl does not create a namespace
/// implicitly for a qualified `proc` name, so without it the definition fails
/// with `can't create procedure "regex::quote": unknown namespace` on every
/// supported release, and the fix would leave the file worse than the
/// diagnostic it silences. Creating a namespace that already exists is a
/// no-op, so the line is safe wherever the fix lands.
const REGEX_QUOTE_PROC: &str = "namespace eval regex {}\n\
    proc regex::quote {str} { regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&} }";
/// `string map` mapping that strips CR/LF — the fix the T101 / IRULE3003 KCS
/// docs recommend for an output/log sink (`puts $x` / `log ... $x`): a
/// `string map` element beginning with `"` is itself list-parsed with
/// backslash escapes honoured, so `"\n"` / `"\r"` really do map the newline /
/// carriage-return characters to empty, not the two-character sequences.
const CRLF_STRIP_MAP: &str = "string map {\"\\n\" \"\" \"\\r\" \"\"}";

/// The diagnostic code every `SpecTcl` pack-load notice carries.
///
/// One code for the family rather than one per notice kind: the notices are
/// *degradations* the loader chose to report, not a diagnostic catalogue with
/// per-code documentation and quick fixes behind it. A user who does not want
/// them silences the family.
///
/// It lives here rather than beside the publisher so the code the notices go
/// out under and the code the quick-fix below reads back are one string.
pub const SPEC_PACK_DIAGNOSTIC_CODE: &str = "SPECTCL";

/// How many candidates a did-you-mean rewrite offers.
///
/// One, not a menu: a pack-load notice names one word, and three near-equal
/// candidates out of a closed vocabulary is a worse answer than none — at that
/// point the author wants the vocabulary, which completion already offers on
/// the same line, not a ranking of guesses.
const SPEC_PACK_SUGGESTIONS: usize = 1;

/// Did-you-mean source edits require an actual typed loader notice and
/// independently current complete source/configuration/Registry. Prose is
/// presentation and cannot select a word, row, vocabulary or edit extent.
#[must_use]
pub fn spec_pack_quick_fixes(
    source: &str,
    analysis: &AnalysisResult,
    diags: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    if registry.document_grammar().is_none() {
        return Vec::new();
    }
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    for d in diags.iter().filter(|d| d.code == SPEC_PACK_DIAGNOSTIC_CODE) {
        let Some(data) = d.data.as_ref().and_then(ContextDiagnosticData::notice) else {
            continue;
        };
        if !data.matches(source, analysis, registry, d) {
            continue;
        }
        let subject = &data.subject;
        let word = subject.word();
        let offset = subject.span().start();
        let Some(head) = statement_head_at(source, offset, config) else {
            continue;
        };
        let grammar = crate::oo_body::definition_grammar_at(
            source,
            offset,
            registry,
            config,
            analysis
                .resolved_profile()
                .map(tcl_dialect::DialectProfile::surface_query),
        );
        let candidates: Vec<&str> = match subject.kind() {
            SpecPackNoticeKind::Property if head == word => {
                grammar.map(|g| g.members.iter().map(|m| m.keyword).collect())
            }
            SpecPackNoticeKind::Flag { row } if head == *row => registry
                .get(&head)
                .map(|spec| spec.options.iter().map(|opt| opt.name).collect()),
            _ => None,
        }
        .unwrap_or_default();
        let Some(&suggestion) = tcl_compiler::text::suggest_similar(
            word,
            candidates,
            SPEC_PACK_SUGGESTIONS,
            tcl_compiler::text::scaled_max_distance(word),
        )
        .first() else {
            continue;
        };
        out.push(CodeAction::new(
            format!("Change `{word}` to `{suggestion}`"),
            vec![crate::rename::TextEdit {
                range: crate::definition::span_to_range(source, &line_index, subject.span()),
                new_text: suggestion.into(),
            }],
            ActionKind::QuickFix,
            None,
        ));
    }
    out
}

/// The keyword of the statement `offset` falls inside.
///
/// A statement is not a line: Tcl ends one at `;` as readily as at a newline,
/// so on `arity 1; arty 2` the line's first word names the *neighbouring* row
/// and every candidate set derived from it is the wrong one. The segmenter is
/// what knows where a row begins, and the descent into braced words is what
/// brings a nested row — every row of a pack is nested at least twice — within
/// its reach.
///
/// `None` when no statement covers the offset (a word inside a comment, a
/// nest deeper than the walk follows), which costs an action rather than
/// offering one computed against the wrong row.
fn statement_head_at(source: &str, offset: u32, config: tcl_lexer::LexerConfig) -> Option<String> {
    fn walk(
        source: &str,
        slice: &str,
        base: u32,
        offset: u32,
        depth: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<String> {
        // The same bound every other walk over a nested body carries.
        if depth > 32 {
            return None;
        }
        for cmd in tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            slice,
            base,
            config.at_depth(depth),
        ) {
            if offset < cmd.span.start() || offset >= cmd.span.end() {
                continue;
            }
            // A braced word holding the offset is a script of its own, and the
            // row the notice points at is one of *its* statements.
            for tok in cmd.argv.iter().skip(1) {
                if tok.kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                let inner_start = tok
                    .span
                    .start()
                    .saturating_add(u32::from(tok.content_offset));
                let inner_end = tok
                    .span
                    .end()
                    .min(u32::try_from(source.len()).unwrap_or(u32::MAX));
                if inner_start > inner_end || offset < inner_start || offset >= inner_end {
                    continue;
                }
                let Some(inner) = source.get(inner_start as usize..inner_end as usize) else {
                    continue;
                };
                return walk(source, inner, inner_start, offset, depth + 1, config);
            }
            return Some(cmd.name().to_owned());
        }
        None
    }

    walk(source, source, 0, offset, 0, config)
}

/// Compatibility entry point without an independent current analysis. It
/// cannot authenticate a diagnostic-driven edit. Use the current-analysis
/// entry point for compiler-issued context diagnostics.
#[must_use]
pub fn context_diagnostic_actions(_source: &str, _diags: &[ContextDiagnostic]) -> Vec<CodeAction> {
    Vec::new()
}

/// Source actions from an actual retained diagnostic, independently checked
/// against the request's complete image, configuration and command store.
#[must_use]
pub fn context_diagnostic_actions_in_analysis(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    diags: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    let Some(commands) = diagnostic_context::current_commands(source, analysis, registry) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for diagnostic in diags {
        let Some(data) = diagnostic.data.as_ref() else {
            continue;
        };
        if !data.matches(source, analysis, registry, diagnostic) {
            continue;
        }
        out.extend(taint_subject_actions(
            source, analysis, registry, &commands, data,
        ));
        out.extend(collect_bootstrap_actions_current(
            source, analysis, registry, &commands, data,
        ));
    }
    let mut seen = FxHashSet::default();
    out.retain(|action| {
        seen.insert((
            action.title.clone(),
            action
                .edits
                .iter()
                .map(|edit| edit.new_text.clone())
                .collect::<Vec<_>>(),
        ))
    });
    out
}

fn source_edit(source: &str, span: tcl_lexer::Span, new_text: String) -> crate::rename::TextEdit {
    crate::rename::TextEdit {
        range: crate::definition::span_to_range(source, &LineIndex::new(source), span),
        new_text,
    }
}

fn context_action(title: String, edits: Vec<crate::rename::TextEdit>) -> CodeAction {
    CodeAction {
        title,
        edits,
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
        disabled: None,
    }
}

/// Availability of a current source descriptor or an inserted source helper.
/// The exact retained generation supplies authoring metadata only; it establishes
/// no live command slot, entered handler, evaluation permission or normal effect.
fn source_action_spec(
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    name: &str,
) -> Option<&'static tcl_registry::CommandSpec> {
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key() {
        return None;
    }
    input.availability_context().resolve_spec(registry, name)
}

fn selected_event_handlers<'a>(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    commands: &'a [diagnostic_context::SourceCommand],
) -> Vec<(&'a diagnostic_context::SourceCommand, &'a str)> {
    let Some(profile) = analysis.resolved_profile() else {
        return Vec::new();
    };
    if !profile.is_irules() {
        return Vec::new();
    }
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap_or(u32::MAX)),
        config,
    ) else {
        return Vec::new();
    };
    if plan.fatal_tail.is_some() {
        return Vec::new();
    }
    let events = EventRegistry::build();
    commands
        .iter()
        .filter_map(|command| {
            // Handler insertion anchors come only from authentic top-level
            // command geometry. A nested command cannot declare that boundary.
            if !plan.commands.iter().any(|root| root.span == command.span) {
                return None;
            }
            let spec = source_action_spec(analysis, registry, &command.canonical)?;
            if !spec.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER)
                || command.words.last()?.group().kind != tcl_lexer::WordKind::Braced
            {
                return None;
            }
            let event = diagnostic_context::source_literal(command.words.get(1)?)?;
            events.is_known(event).then_some((command, event))
        })
        .collect()
}

fn collect_bootstrap_actions_current(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    commands: &[diagnostic_context::SourceCommand],
    data: &ContextDiagnosticData,
) -> Vec<CodeAction> {
    let Some(diagnostic) = data.diagnostic() else {
        return Vec::new();
    };
    if !matches!(diagnostic.code.as_str(), "IRULE1005" | "IRULE1006") {
        return Vec::new();
    }
    let Some(profile) = analysis.resolved_profile() else {
        return Vec::new();
    };
    if !profile.is_irules() {
        return Vec::new();
    }
    let handlers = selected_event_handlers(source, analysis, registry, commands);
    let mut enclosing = handlers
        .iter()
        .filter(|(command, _)| {
            command.span.start() <= diagnostic.span.start()
                && diagnostic.span.end() <= command.span.end()
        })
        .collect::<Vec<_>>();
    enclosing.sort_by_key(|(command, _)| command.span.end() - command.span.start());
    let Some((enclosing, event)) = enclosing.first().copied() else {
        return Vec::new();
    };
    let events = EventRegistry::build();
    let Some(handler) = source_action_spec(analysis, registry, &enclosing.canonical) else {
        return Vec::new();
    };
    let Some(priority) = handler.event_handler_priority else {
        return Vec::new();
    };
    if source_action_spec(analysis, registry, handler.name).is_none() {
        return Vec::new();
    }
    let choices: Vec<(&str, &str)> = if diagnostic.code.as_str() == "IRULE1005" {
        // The issuer's exact event operand must belong to the selected header.
        if enclosing
            .words
            .get(1)
            .is_none_or(|word| word.span() != diagnostic.span)
        {
            return Vec::new();
        }
        let Some(setup) = events
            .get_props(event)
            .and_then(|properties| properties.setup_event)
        else {
            return Vec::new();
        };
        let Some((protocols, _)) = events.data_collect_requirement(event) else {
            return Vec::new();
        };
        protocols
            .iter()
            .filter_map(|protocol| {
                registry
                    .data_collection_collect_command(protocol)
                    .filter(|spec| source_action_spec(analysis, registry, spec.name).is_some())
                    .map(|spec| (spec.name, setup))
            })
            .collect()
    } else {
        let mut operations = commands
            .iter()
            .filter(|command| {
                command.span.start() <= diagnostic.span.start()
                    && diagnostic.span.end() <= command.span.end()
            })
            .filter_map(|command| registry.data_collection_operation(&command.canonical))
            .filter(|operation| operation.action == DataCollectionAction::Payload);
        let Some(operation) = operations.next() else {
            return Vec::new();
        };
        if operations.next().is_some() {
            return Vec::new();
        }
        let Some(setup) = operation.protocol.bootstrap_event_for(event) else {
            return Vec::new();
        };
        registry
            .data_collection_collect_command(operation.protocol.name)
            .filter(|spec| source_action_spec(analysis, registry, spec.name).is_some())
            .map(|spec| vec![(spec.name, setup)])
            .unwrap_or_default()
    };
    let index = LineIndex::new(source);
    let anchor = index.line_at(enclosing.span.start());
    choices
        .into_iter()
        .map(|(collect, setup)| {
            context_action(
                format!("Add '{collect}' bootstrap in '{setup}'"),
                vec![crate::rename::TextEdit {
                    range: LspRange {
                        start_line: anchor,
                        start_character: 0,
                        end_line: anchor,
                        end_character: 0,
                    },
                    new_text: format!(
                        "{} {setup} {} {} {{\n    {collect}\n}}\n\n",
                        handler.name, priority.keyword, priority.default_priority
                    ),
                }],
            )
        })
        .collect()
}

fn taint_subject_actions(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    commands: &[diagnostic_context::SourceCommand],
    data: &ContextDiagnosticData,
) -> Vec<CodeAction> {
    let Some(diagnostic) = data.diagnostic() else {
        return Vec::new();
    };
    let Some(subject) = diagnostic.taint_subject.as_ref() else {
        return Vec::new();
    };
    let Some(command) = diagnostic_context::selected_sink(commands, diagnostic) else {
        return Vec::new();
    };
    // This hardening changes subst's accepted input, independently of a
    // semantics-preserving refactoring. The selected source head, option
    // grammar and written insertion extent must all be current.
    if diagnostic.code.as_str() == "T100" && subject.sink_command() == "subst" {
        let Some(spec) = source_action_spec(analysis, registry, &command.canonical) else {
            return Vec::new();
        };
        if !spec
            .options
            .iter()
            .any(|option| option.name == "-nocommands")
            || command
                .words
                .iter()
                .skip(1)
                .any(|word| diagnostic_context::source_literal(word) == Some("-nocommands"))
        {
            return Vec::new();
        }
        let Some(head) = command.words.first() else {
            return Vec::new();
        };
        return vec![context_action(
            "Add -nocommands to disable command substitution".to_owned(),
            vec![source_edit(
                source,
                tcl_lexer::Span::new(head.span().end(), head.span().end()),
                " -nocommands".to_owned(),
            )],
        )];
    }
    // Other rewrites introduce or remove evaluations. Authored vendor and
    // explicit lexical source advice may offer reviewed hardening; selected
    // Native inputs require independent insertion/evaluation permissions.
    if !analysis.has_original_vendor_source_names()
        && !analysis.allows_retained_logical_declaration_advice()
    {
        return Vec::new();
    }
    let Some(read) = diagnostic_context::selected_read(source, analysis, command, diagnostic)
    else {
        return Vec::new();
    };
    let Some(matched) = source.get(read.as_range()) else {
        return Vec::new();
    };
    let variable = subject.variable();
    match diagnostic.code.as_str() {
        "T101" | "IRULE3003" => {
            if source_action_spec(analysis, registry, "string").is_none() {
                return Vec::new();
            }
            vec![context_action(
                format!("Sanitise ${variable} (strip CR/LF) before output"),
                vec![source_edit(
                    source,
                    read,
                    format!("[{CRLF_STRIP_MAP} {matched}]"),
                )],
            )]
        }
        "T106" => {
            // Only the exact selected encoder substitution can be removed.
            // A nearby bracket or a different variable is never an anchor.
            if command.words.len() != 2
                || command.words[1].span() != read
                || command.span.start() == 0
                || source.as_bytes().get(command.span.start() as usize - 1) != Some(&b'[')
                || source.as_bytes().get(command.span.end() as usize) != Some(&b']')
            {
                return Vec::new();
            }
            vec![context_action(
                "Remove redundant encoder".to_owned(),
                vec![source_edit(
                    source,
                    tcl_lexer::Span::new(command.span.start() - 1, command.span.end() + 1),
                    matched.to_owned(),
                )],
            )]
        }
        "IRULE3001" | "IRULE3002" | "T103" => {
            let (encoder, template) = match diagnostic.code.as_str() {
                "IRULE3001" => ("html_encode", Some(HTML_ENCODE_PROC)),
                "IRULE3002" => ("URI::encode", None),
                _ => ("regex::quote", Some(REGEX_QUOTE_PROC)),
            };
            let mut edits = Vec::new();
            if let Some(template) = template {
                if !diagnostic_context::helper_name_is_free(analysis, encoder)
                    || source_action_spec(analysis, registry, "proc").is_none()
                    || source_action_spec(
                        analysis,
                        registry,
                        if encoder == "html_encode" {
                            "string"
                        } else {
                            "regsub"
                        },
                    )
                    .is_none()
                {
                    return Vec::new();
                }
                edits.push(source_edit(
                    source,
                    tcl_lexer::Span::new(0, 0),
                    format!("{template}\n"),
                ));
            } else if source_action_spec(analysis, registry, encoder).is_none() {
                return Vec::new();
            }
            edits.push(source_edit(
                source,
                read,
                format!("[::{encoder} {matched}]"),
            ));
            vec![context_action(
                format!("Wrap ${variable} with [{encoder}]"),
                edits,
            )]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::diagnostic_policy::{Directives, Policy, PolicyBuilder, PolicyLayer, apply};
    use tcl_compiler::analyser::{Analyser, AnalysisResult, CodeFix, Diagnostic};
    use tcl_lexer::Span;

    #[test]
    fn original_source_action_metadata_uses_retained_availability_over_report_profile() {
        // naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        // Exact metadata admission, without a Native slot/handler/effect claim.
        let catalogue = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        for (environment, available) in [("tcl8.6", true), ("tcl8.4", false)] {
            let context = std::sync::Arc::new(
                tcl_registry::model::ingress::static_context_for(environment)
                    .with_command_store(std::sync::Arc::clone(catalogue.commands())),
            );
            assert!(std::sync::Arc::ptr_eq(
                context.commands(),
                catalogue.commands()
            ));
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse("", profile.name);
            analysis.dialect = "tcl9.1".to_owned();
            assert_eq!(
                source_action_spec(&analysis, catalogue.commands(), "dict").is_some(),
                available,
                "same store and profile, retained {environment} availability",
            );
            analysis.resolved_input = None;
            assert!(source_action_spec(&analysis, catalogue.commands(), "dict").is_none());
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
        assert!(
            code_actions(
                "set x 1\n",
                whole_document_range("set x 1\n"),
                None,
                &Report::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn fix_attached_to_diagnostic_surfaces_as_action() {
        // A current source analysis carries the isolated fix-lifting fixture.
        // This checks transport; semantic fix permission remains its issuer's.
        let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
        r.diagnostics.clear();
        r.diagnostics.push(Diagnostic {
            subject: None,
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
            &report_of(&r.diagnostics),
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
        let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
        r.diagnostics.clear();
        r.diagnostics.push(Diagnostic {
            subject: None,
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
        assert!(
            code_actions("set x 1\n", far_range, Some(&r), &report_of(&r.diagnostics)).is_empty()
        );
    }

    #[test]
    fn empty_description_falls_back_to_diagnostic_message() {
        let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
        r.diagnostics.clear();
        r.diagnostics.push(Diagnostic {
            subject: None,
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
            &report_of(&r.diagnostics),
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
            &report_of(&analysis.diagnostics),
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
        let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
        r.diagnostics.clear();
        r.diagnostics.push(Diagnostic {
            subject: None,
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
            &report_of(&r.diagnostics),
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
            &report_of(&analysis.diagnostics),
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
        // braces, so the transform declines and says why.
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
            &report_of(&analysis.diagnostics),
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
            &report_of(&analysis.diagnostics),
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
    /// the inserted string plus a zero-width range: an insertion of exactly
    /// the right text at exactly the wrong place satisfies both of those
    /// weaker assertions.
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
            &report_of(&analysis.diagnostics),
        )
        .into_iter()
        .filter(|action| action.title.starts_with("Add catch"))
        .collect()
    }

    #[test]
    fn w302_result_action_applies_after_the_body() {
        // The diagnostic anchors at the `catch` word, so a provider that
        // reconstructs the insertion point from the diagnostic's end
        // produces `catch result { puts hi }` — a catch of the script
        // `result`.
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
            &report_of(&analysis.diagnostics),
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

    // Fuzzy `package require` suggestions.
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
        // Splitting on `::` without stripping it first yields an empty
        // namespace component, which matches nothing at all.
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
        // A textual editor diagnostic carries no original issuer. The
        // genuine analysis subject, when present, independently supports
        // the suggestion; removing it must leave no native fallback.
        let src = "http::foo $x\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, "tcl9.0").clone();
        let context = vec![ContextDiagnostic {
            data: None,
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

    /// A document with no `# noqa` and no file-level directive.
    fn no_suppression() -> HashMap<i32, HashSet<String>> {
        HashMap::new()
    }

    fn check_analysis(
        source: &str,
        registry: &tcl_registry::CommandRegistry,
        profile: &'static tcl_dialect::DialectProfile,
        config: tcl_lexer::LexerConfig,
    ) -> AnalysisResult {
        let context = crate::context_for_dialect_profile(profile)
            .with_command_store(registry.snapshot().shared_registry());
        Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(context),
                config,
            ))
            .analyse(source, profile.name)
    }

    fn report_of(diagnostics: &[Diagnostic]) -> crate::diagnostic_policy::Report {
        apply(
            diagnostics
                .iter()
                .cloned()
                .map(crate::diagnostic_policy::Finding::from)
                .collect(),
            &Policy::unrestricted(),
        )
    }
    fn check_actions(
        current: &DiagnosticEditSource<'_>,
        range: LspRange,
        checks: &[tcl_compiler::compiler_checks::Diagnostic],
        disabled: &HashSet<String>,
        suppressed: &HashMap<i32, HashSet<String>>,
    ) -> Vec<CodeAction> {
        let layer: serde_json::Map<String, serde_json::Value> = disabled
            .iter()
            .map(|code| (code.clone(), serde_json::Value::Bool(false)))
            .collect();
        let policy = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &serde_json::json!({"diagnostics": layer}),
            )
            .directives(Directives::new(suppressed.clone(), current.source()))
            .build();
        let report = apply(
            checks
                .iter()
                .cloned()
                .map(crate::diagnostic_policy::Finding::from)
                .collect(),
            &policy,
        );
        code_actions(current.source(), range, Some(current.analysis()), &report)
            .into_iter()
            .filter(|action| action.kind == ActionKind::QuickFix)
            .collect()
    }
    fn style_report(source: &str, analysis: &AnalysisResult) -> crate::diagnostic_policy::Report {
        crate::diagnostic_report::document_report_with_analysis(
            &w115_test_doc(source),
            Vec::new(),
            &Policy::unrestricted(),
            Some(analysis),
        )
    }

    // compiler-check fixes: IRULE5002/5004 flow-warning fixes

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
        let profile = tcl_dialect::DialectProfile::irules();
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some(profile));
        let checks = run_all_checks(&cu, &registry, Some(profile));
        let analysis = check_analysis(
            src,
            &registry,
            profile,
            LexerConfig::for_dialect("f5-irules"),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
        assert!(
            checks
                .iter()
                .any(|d| d.code == DiagCode::Irule5002 && !d.fixes.is_empty()),
            "expected an IRULE5002 check with a fix, got {checks:?}",
        );

        let none_disabled = std::collections::HashSet::new();
        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &none_disabled,
            &no_suppression(),
        );
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
        let analysis = Analyser::new().analyse(src, "tcl8.6");
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
        let none_disabled = std::collections::HashSet::new();
        assert!(
            check_actions(
                &current,
                whole_document_range(src),
                &[],
                &none_disabled,
                &no_suppression(),
            )
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
        let profile = tcl_dialect::DialectProfile::irules();
        let cu = CompilationUnit::build_for_with_config(
            src,
            &registry,
            false,
            LexerConfig::for_dialect("f5-irules"),
        )
        .with_interprocedural(&registry, Some(profile));
        let checks = run_all_checks(&cu, &registry, Some(profile));
        let analysis = check_analysis(
            src,
            &registry,
            profile,
            LexerConfig::for_dialect("f5-irules"),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();

        let mut disabled = std::collections::HashSet::new();
        disabled.insert("IRULE5002".to_string());
        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &disabled,
            &no_suppression(),
        );
        assert!(
            !actions
                .iter()
                .any(|a| a.title == "Add 'event disable all' + 'return'"),
            "disabled IRULE5002 must not offer a quick-fix, got {actions:?}",
        );
    }

    // compiler-check fixes: shimmer-family noqa-suppress action

    /// A check the document already silences offers nothing: neither its
    /// quick-fix nor a suppress action for a line that is already suppressed.
    #[test]
    fn check_actions_skip_a_diagnostic_a_noqa_already_silences() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        use tcl_compiler::compiler_checks::run_all_checks;

        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "set x hello\n# noqa: S100\nincr x\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let checks = run_all_checks(&cu, &registry, None);
        let analysis = check_analysis(
            src,
            &registry,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_lexer::LexerConfig::default(),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
        assert!(
            checks.iter().any(|d| d.code == DiagCode::S100),
            "the check itself still fires; only its actions are withheld: {checks:?}"
        );
        let suppressed = Analyser::new()
            .analyse(src, "tcl8.6")
            .suppressed_lines
            .clone();

        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &std::collections::HashSet::new(),
            &suppressed,
        );
        assert!(
            actions.is_empty(),
            "a silenced S100 must leave no action behind: {actions:?}"
        );
    }

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
        let analysis = check_analysis(
            src,
            &registry,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_lexer::LexerConfig::default(),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
        let s100 = checks
            .iter()
            .find(|d| d.code == DiagCode::S100)
            .unwrap_or_else(|| panic!("expected an S100 check, got {checks:?}"));
        assert!(
            s100.fixes.is_empty(),
            "S100 should carry no diag-level CodeFix — this test exercises the synthetic path"
        );

        let none_disabled = std::collections::HashSet::new();
        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &none_disabled,
            &no_suppression(),
        );
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
        let analysis = check_analysis(
            src,
            &registry,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_lexer::LexerConfig::default(),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
        assert!(checks.iter().any(|d| d.code == DiagCode::S100));

        let none_disabled = std::collections::HashSet::new();
        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &none_disabled,
            &no_suppression(),
        );
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
        let analysis = check_analysis(
            src,
            &registry,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_lexer::LexerConfig::default(),
        );
        let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();

        let mut disabled = std::collections::HashSet::new();
        disabled.insert("S100".to_string());
        let actions = check_actions(
            &current,
            whole_document_range(src),
            &checks,
            &disabled,
            &no_suppression(),
        );
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

    /// An O127 pair is one quick-fix carrying both members' edits, never an
    /// action for either member alone (#2149).
    #[test]
    fn a_grouped_rewrite_is_one_action_with_every_edit() {
        let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let pair: Vec<Finding> = tcl_compiler::optimiser::optimise_with_dialect(
            src,
            &registry,
            Some(crate::profile_for_dialect("tcl8.6")),
        )
        .into_iter()
        .filter(|o| o.code == DiagCode::O127)
        .map(Finding::from)
        .collect();
        assert_eq!(pair.len(), 2, "the fixture yields the O127 pair: {pair:?}");
        let policy = Policy {
            optimiser: crate::diagnostic_policy::OptimiserPolicy::all_on(),
            ..Policy::default()
        };
        let report = apply(pair, &policy);
        let actions: Vec<CodeAction> =
            code_actions(src, whole_document_range(src), Some(&analyse(src)), &report)
                .into_iter()
                .filter(|a| a.kind == ActionKind::QuickFix)
                .collect();
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].edits.len(), 2, "{actions:?}");
        assert!(
            actions[0].edits.iter().any(|e| e.new_text.is_empty()),
            "the delete member rides the same action: {actions:?}"
        );
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
        let actions = code_actions(
            src,
            range,
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
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
                &report_of(&analysis.diagnostics),
            );
            assert!(
                !actions
                    .iter()
                    .any(|action| action.title == "Convert to per-line comments"),
                "pseudo-comment offered W115 action: {actions:?}"
            );
        }
    }

    fn w115_test_doc(text: &str) -> crate::diagnostic_report::DocumentSource<'_> {
        crate::diagnostic_report::DocumentSource {
            text,
            analysis_text: text,
            decode: None,
            dialect: crate::profile_for_dialect("tcl8.6"),
            pass: crate::diagnostic_report::SourcePass::Tcl { line_length: 120 },
        }
    }

    /// The conversion follows a *shown* W115, not the bare comment
    /// shape — a layer that turns W115 off, or a directive that silences it,
    /// must silence the action too.
    #[test]
    fn a_conversion_follows_a_shown_w115() {
        let dialect = crate::profile_for_dialect("tcl8.6");
        let offers_conversion = |actions: &[CodeAction]| {
            actions
                .iter()
                .any(|a| a.title.contains("per-line comments"))
        };

        let src = "# trailing \\\nset x 1\n";
        let analysis = analyse(src);
        let shown = crate::diagnostic_report::document_report(
            &w115_test_doc(src),
            Vec::new(),
            &Policy::unrestricted(),
        );
        assert!(
            offers_conversion(&code_actions(src, line_range(0), Some(&analysis), &shown)),
            "a shown W115 must offer the conversion"
        );

        let w115_off = PolicyBuilder::new()
            .layer(
                PolicyLayer::Editor,
                &serde_json::json!({ "diagnostics": { "W115": false } }),
            )
            .build();
        let disabled =
            crate::diagnostic_report::document_report(&w115_test_doc(src), Vec::new(), &w115_off);
        assert!(
            !offers_conversion(&code_actions(
                src,
                line_range(0),
                Some(&analysis),
                &disabled
            )),
            "W115 disabled at a layer must not offer the conversion"
        );

        let marked = "# noqa: W115\n# trailing \\\nset x 1\n";
        let marked_analysis = analyse(marked);
        let scanned = PolicyBuilder::new()
            .directives(Directives::scan(marked, dialect))
            .build();
        let silenced =
            crate::diagnostic_report::document_report(&w115_test_doc(marked), Vec::new(), &scanned);
        assert!(
            !offers_conversion(&code_actions(
                marked,
                line_range(1),
                Some(&marked_analysis),
                &silenced
            )),
            "a `# noqa: W115` must not offer the conversion"
        );
    }

    #[test]
    fn w115_action_uses_actual_schema_and_complete_original_unicode_lines() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "hold-script {\n    # café 😀 \\\n    puts 😀\n}\n";
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "hold-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let profile = crate::profile_for_dialect("tcl8.6");
        let config = tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let analysis = check_analysis(source, &registry, profile, config);
        let range = LspRange {
            start_line: 1,
            start_character: 4,
            end_line: 1,
            end_character: 4,
        };
        let actions = code_actions(
            source,
            range,
            Some(&analysis),
            &style_report(source, &analysis),
        );
        let action = actions
            .iter()
            .find(|action| action.title == "Convert to per-line comments")
            .unwrap();
        assert_eq!(action.edits.len(), 1);
        assert_eq!(
            action.edits[0].range,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 2,
                end_character: 11
            }
        );
        assert_eq!(action.edits[0].new_text, "    # café 😀\n    # puts 😀");
        assert!(
            !code_actions(
                &format!("{source} "),
                range,
                Some(&analysis),
                &style_report(source, &analysis)
            )
            .iter()
            .any(|action| action.title == "Convert to per-line comments")
        );
    }

    #[test]
    fn w115_action_never_replaces_a_command_prefix_beside_a_body_comment() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "if 1 {# inline \\\nputs hidden\n}\n";
        let analysis = analyse(source);
        assert!(
            !code_actions(
                source,
                whole_document_range(source),
                Some(&analysis),
                &style_report(source, &analysis)
            )
            .iter()
            .any(|action| action.title == "Convert to per-line comments")
        );
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
        let mut actions = code_actions(
            src,
            range,
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
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
        let actions = code_actions(
            src,
            cursor,
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
        assert!(
            actions
                .iter()
                .any(|a| a.title.to_lowercase().contains("switch")),
            "{actions:?}",
        );
    }

    /// The refactor code actions reach control flow inside an
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
        let actions = code_actions(
            src,
            cursor,
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
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
        let analysis = Analyser::new().analyse(src, "f5-irules");
        let cursor = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        };
        let actions = code_actions(
            src,
            cursor,
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
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

    /// A pack-load notice about a dropped property becomes the one edit that
    /// spells the property the way the grammar in force does.
    #[test]
    fn a_dropped_property_offers_the_nearest_real_word() {
        let src = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "arty",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "mylib::x: unknown property `arty` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 14,
            },
        }];
        let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].title, "Change `arty` to `arity`");
        assert_eq!(actions[0].edits.len(), 1);
        assert_eq!(actions[0].edits[0].new_text, "arity");
        assert_eq!(actions[0].edits[0].range.start_line, 2);
        assert_eq!(actions[0].edits[0].range.start_character, 8);
        assert_eq!(actions[0].edits[0].range.end_character, 12);
    }

    /// The candidate set is the grammar *in force*, not the pack's whole
    /// vocabulary: `summary` is a `hover` word, so a typo of it inside a
    /// `command` body must not resolve to it.
    #[test]
    fn the_candidate_vocabulary_is_the_grammar_in_force() {
        let src = "speclib mylib 1 {\n    command mylib::x {\n        hover {\n            sumary {x}\n        }\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                4,
                "sumary",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown property `sumary` dropped".to_owned(),
            range: LspRange {
                start_line: 3,
                start_character: 0,
                end_line: 3,
                end_character: 22,
            },
        }];
        let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].edits[0].new_text, "summary");

        // The same misspelling one level out has no `summary` to reach.
        let outer = "speclib mylib 1 {\n    command mylib::x {\n        sumary {x}\n    }\n}\n";
        let outer_diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                outer,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "sumary",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(outer, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown property `sumary` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 18,
            },
        }];
        assert!(
            !spec_pack_quick_fixes(
                outer,
                &Analyser::new().analyse(outer, "spectcl"),
                &outer_diags
            )
            .iter()
            .any(|a| a.edits[0].new_text == "summary")
        );
    }

    /// A dropped flag is corrected against the option table of the row it was
    /// written on, which the registry owns.
    #[test]
    fn a_dropped_flag_offers_the_nearest_option() {
        let src = "speclib mylib 1 {\n    command mylib::x {\n        arg 0 -rle Body\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "-rle",
                SpecPackNoticeKind::Flag { row: "arg".into() },
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown flag `-rle` on `arg` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 23,
            },
        }];
        let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].edits[0].new_text, "-role");
    }

    /// Two rows on one line: `;` ends a statement, so the row a word sits on
    /// is the segmenter's answer and not the line's first word — which here
    /// names the *neighbour* and would offer nothing at all.
    #[test]
    fn a_row_sharing_its_line_is_corrected_against_its_own_head() {
        let src = "speclib mylib 1 {\n    command mylib::x {\n        arity 1; arty 2\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "arty",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "mylib::x: unknown property `arty` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 23,
            },
        }];
        let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].edits[0].new_text, "arity");
        assert_eq!(actions[0].edits[0].range.start_character, 17);

        // …and the flag case, whose option table comes from the head of the
        // row the flag was written on (`arg`), not the line's (`arity`).
        let src = "speclib mylib 1 {\n    command mylib::x {\n        arity 1; arg 0 -rle Body\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "-rle",
                SpecPackNoticeKind::Flag { row: "arg".into() },
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown flag `-rle` on `arg` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 32,
            },
        }];
        let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].edits[0].new_text, "-role");
    }

    /// A word the buffer no longer carries — the notice is from the last load,
    /// the author has since retyped the line — yields no edit at all.
    #[test]
    fn a_notice_the_buffer_has_moved_past_offers_nothing() {
        let src = "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n    }\n}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                3,
                "arty",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown property `arty` dropped".to_owned(),
            range: LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 15,
            },
        }];
        assert!(
            spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags).is_empty()
        );
    }

    /// Only a pack file: the same-shaped diagnostic on an ordinary Tcl
    /// document has no closed vocabulary behind it.
    #[test]
    fn an_ordinary_tcl_document_gets_no_pack_quick_fix() {
        let src = "proc arty {} {}\n";
        let diags = vec![ContextDiagnostic {
            data: SpecPackNoticeSubject::from_loader(
                src,
                tcl_lexer::LexerConfig::for_file_grammar(
                    crate::profile_for_dialect("spectcl").grammar,
                ),
                1,
                "arty",
                SpecPackNoticeKind::Property,
            )
            .and_then(|subject| {
                ContextDiagnosticData::from_spec_pack_notice(
                    &subject,
                    &Analyser::new().analyse(src, "spectcl"),
                )
            }),
            code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
            message: "unknown property `arty` dropped".to_owned(),
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 15,
            },
        }];
        assert!(
            spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "tcl"), &diags,).is_empty()
        );
    }
}

#[cfg(test)]
mod original_docstring_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn actions(
        source: &str,
        analysis: &AnalysisResult,
        line: u32,
        style: crate::formatting::DocstringStyle,
    ) -> Vec<CodeAction> {
        let index = LineIndex::new(source);
        docstring_actions(
            source,
            LspRange {
                start_line: line,
                start_character: 0,
                end_line: line,
                end_character: 0,
            },
            analysis,
            &index,
            style,
        )
    }

    #[test]
    // Implementation contract: naming.core.original-docstring-declaration-actions
    // docs/design/analysis/name-resolution-proofs/original-docstring-declaration-actions.md
    fn original_docstrings_keep_opaque_redefinitions_without_reporting_maps() {
        let source = "proc p\\uD800 {first} {return $first}\nproc p\\uD801 {second} {return $second}\nproc p\\uD800 {third} {return $third}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for (line, formal) in [(0, "first"), (1, "second"), (2, "third")] {
            let found = actions(
                source,
                &analysis,
                line,
                crate::formatting::DocstringStyle::Preceding,
            );
            assert_eq!(found.len(), 1);
            assert!(found[0].edits[0].new_text.contains(formal));
            assert_eq!(found[0].edits[0].range.start_line, line);
        }
        assert!(
            actions(
                &format!("# displaced\n{source}"),
                &analysis,
                1,
                crate::formatting::DocstringStyle::Preceding
            )
            .is_empty()
        );
        analysis.body_lexer_config = None;
        assert!(
            actions(
                source,
                &analysis,
                0,
                crate::formatting::DocstringStyle::Preceding
            )
            .is_empty()
        );
    }

    #[test]
    // Implementation contract: naming.core.original-docstring-declaration-actions
    // docs/design/analysis/name-resolution-proofs/original-docstring-declaration-actions.md
    fn original_body_docstring_requires_its_braced_word_and_rejects_computed_body() {
        let source = "proc p {value} {return $value}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let found = actions(
            source,
            &analysis,
            0,
            crate::formatting::DocstringStyle::Body,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].edits[0].new_text.contains("value"));
        let computed = "set script {return value}\nproc computed {} $script\n";
        let analysis = Analyser::new().analyse(computed, "tcl8.6");
        assert_eq!(
            actions(
                computed,
                &analysis,
                1,
                crate::formatting::DocstringStyle::Preceding
            )
            .len(),
            1
        );
        assert!(
            actions(
                computed,
                &analysis,
                1,
                crate::formatting::DocstringStyle::Body
            )
            .is_empty()
        );
        assert!(
            actions(
                computed,
                &analysis,
                1,
                crate::formatting::DocstringStyle::None
            )
            .is_empty()
        );
    }
}

#[cfg(test)]
mod original_package_suggestion_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(line: u32) -> LspRange {
        LspRange {
            start_line: line,
            start_character: 2,
            end_line: line,
            end_character: 2,
        }
    }

    #[test]
    // Implementation contract: naming.core.original-package-suggestion-subjects
    // docs/design/analysis/name-resolution-proofs/original-package-suggestion-subjects.md
    fn original_package_hint_uses_written_units_and_issuer_after_reporting_clear() {
        let source = "http::f\\uD800\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let registry = tcl_registry::CommandRegistry::build_default();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "other::display".into();
        }
        for diagnostic in &mut analysis.diagnostics {
            diagnostic.message = "counterfactual display".into();
        }
        let actions = package_require_actions(source, at(0), &registry, Some(&analysis), &[]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].edits[0].new_text, "package require http\n");
        assert!(
            package_require_actions(
                &format!("# displaced\n{source}"),
                at(1),
                &registry,
                Some(&analysis),
                &[]
            )
            .is_empty()
        );
        let context = [ContextDiagnostic {
            data: None,
            code: "W123".into(),
            message: "Unknown http::name".into(),
            range: at(0),
        }];
        for diagnostic in &mut analysis.diagnostics {
            diagnostic.subject = None;
        }
        assert!(
            package_require_actions(source, at(0), &registry, Some(&analysis), &context).is_empty()
        );
    }

    #[test]
    // Implementation contract: naming.core.original-package-suggestion-subjects
    // docs/design/analysis/name-resolution-proofs/original-package-suggestion-subjects.md
    fn original_package_hint_requires_own_lookup_and_canonical_package_key() {
        let source = "http::missing\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            package_require_actions(source, at(0), &registry, Some(&analysis), &[]).len(),
            1
        );
        let invocation = analysis
            .command_invocations
            .iter_mut()
            .find(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"http::missing")
            })
            .unwrap();
        invocation.original_lookup = None;
        assert!(package_require_actions(source, at(0), &registry, Some(&analysis), &[]).is_empty());
        let source = "package require h\\u0074tp\nhttp::missing\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let requirement = analysis.package_requires.first_mut().unwrap();
        assert!(
            requirement
                .original_name
                .as_ref()
                .unwrap()
                .key()
                .matches_ascii("http")
        );
        requirement.name = "unrelated-report".into();
        assert!(package_require_actions(source, at(1), &registry, Some(&analysis), &[]).is_empty());
    }
    #[test]
    fn original_package_action_uses_genuine_provision_and_requirement_operands() {
        // naming.core.original-package-source-action-context
        // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
        for (source, available) in [
            ("package provide http\n", false),
            ("package provide http 1\n", true),
            ("package require -exact h\\u0074tp 1\n", true),
            ("set text {package require http}\n", false),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            for requirement in &mut analysis.package_requires {
                requirement.name = "wrong".to_owned();
            }
            for provision in &mut analysis.package_provides {
                provision.name = "wrong".to_owned();
            }
            assert_eq!(
                super::original_source_package_already_named(source, &analysis, "http"),
                Some(available),
                "{source}"
            );
        }
    }

    #[test]
    fn original_logical_package_action_requires_its_positive_vector_and_current_input() {
        // naming.core.original-package-source-action-context
        // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-package-source-action",
            &[],
            "Logical package advice",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let source = "http::missing\n";
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        assert!(
            tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
                source, &analysis, 0
            )
            .is_some()
        );
        for invocation in &mut analysis.command_invocations {
            invocation.name = "wrong::report".to_owned();
        }
        let actions =
            package_require_actions(source, at(0), context.commands(), Some(&analysis), &[]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].edits[0].new_text, "package require http\n");
        let mut config = input.lexer_config();
        config.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
        };
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            config,
        ));
        assert!(
            package_require_actions(source, at(0), context.commands(), Some(&analysis), &[])
                .is_empty()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
                source, &native, 0
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod original_refactor_context_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_refactor_actions_keep_document_registry_and_full_source() {
        // Implementation contract: naming.refactor.original-document-context
        // docs/design/analysis/name-resolution-proofs/refactor-original-document-context.md
        let source = "if {$host eq \"a.com\"} {pool web_pool} elseif {$host eq \"b.com\"} {pool web_pool} elseif {$host eq \"c.com\"} {pool web_pool}";
        let range = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        };
        let index = LineIndex::new(source);
        let mut plain = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !extract_inline_actions(source, range, &plain, &index, None)
                .iter()
                .any(|action| action.data_group_definition.is_some())
        );
        plain.dialect = "f5-irules".to_owned();
        assert!(
            !extract_inline_actions(source, range, &plain, &index, None)
                .iter()
                .any(|action| action.data_group_definition.is_some())
        );
        let mut vendor = Analyser::new().analyse(source, "f5-irules");
        let actions = extract_inline_actions(source, range, &vendor, &index, None);
        assert!(
            actions
                .iter()
                .any(|action| action.data_group_definition.is_some()),
            "{actions:?}"
        );
        vendor.dialect = "tcl8.6".to_owned();
        assert!(
            extract_inline_actions(source, range, &vendor, &index, None)
                .iter()
                .any(|action| action.data_group_definition.is_some())
        );
        let displaced = format!("#{source}");
        assert!(
            extract_inline_actions(
                &displaced,
                range,
                &vendor,
                &LineIndex::new(&displaced),
                None
            )
            .is_empty()
        );
        vendor.body_lexer_config = None;
        assert!(extract_inline_actions(source, range, &vendor, &index, None).is_empty());
    }
}

#[cfg(test)]
mod original_event_action_boundary_tests {
    use super::*;

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_bootstrap_boundaries_exclude_nested_handler_lookalikes() {
        let source = "when HTTP_REQUEST {if {1} {when CLIENT_DATA {TCP::payload}}}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let registry = analysis.resolved_registry().unwrap();
        let commands = diagnostic_context::current_commands(source, &analysis, registry).unwrap();
        let handlers = selected_event_handlers(source, &analysis, registry, &commands);
        assert_eq!(
            handlers.iter().map(|(_, event)| *event).collect::<Vec<_>>(),
            vec!["HTTP_REQUEST"]
        );
        assert_eq!(handlers[0].0.span.start(), 0);
    }
}

#[cfg(test)]
mod original_expression_action_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn actions(source: &str, analysis: &AnalysisResult) -> Vec<CodeAction> {
        let selected = "!(1 && 0)";
        let start = u32::try_from(source.find(selected).unwrap()).unwrap();
        expr_rewrite_actions(
            source,
            LspRange {
                start_line: 0,
                start_character: start,
                end_line: 0,
                end_character: start + u32::try_from(selected.len()).unwrap(),
            },
            analysis,
            &LineIndex::new(source),
        )
    }

    #[test]
    // Implementation contract: naming.core.original-expression-source-selection
    // docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
    fn original_expression_actions_separate_position_from_rewrite_equivalence() {
        let source = "expr {!(1 && 0)}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let proposed = actions(source, &analysis);
        assert_eq!(proposed.len(), 1);
        assert!(proposed[0].edits.is_empty());
        assert!(
            proposed[0]
                .disabled
                .as_ref()
                .unwrap()
                .starts_with("missing-expression-operator-rewrite-permission:")
        );
        analysis.command_invocations.clear();
        assert_eq!(actions(source, &analysis).len(), 1);
        assert!(actions(&format!("#{source}"), &analysis).is_empty());
        for source in [
            "set data {!(1 && 0)}",
            "# expr {!(1 && 0)}",
            "proc expr args {}; expr {!(1 && 0)}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(actions(source, &analysis).is_empty(), "{source}");
        }
    }
}
