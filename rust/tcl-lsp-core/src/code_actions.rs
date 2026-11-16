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

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lexer::{LineIndex, Utf16Col};

use crate::definition::{LspRange, utf16_col_to_char_col};
use crate::diagnostic_policy::{Finding, Fix, Report};

mod context_diagnostics;
mod docstring_stub;
mod expression_rewrites;
mod irules_profiles;
mod package_suggestions;
mod spec_pack_fixes;

pub use context_diagnostics::{
    ContextDiagnostic, context_diagnostic_actions, context_diagnostic_actions_in_analysis,
};
#[cfg(test)]
use context_diagnostics::{HTML_ENCODE_PROC, selected_event_handlers, source_action_spec};
#[cfg(test)]
use context_diagnostics::{context_action, deduplicate_context_actions};
use docstring_stub::docstring_actions;
use expression_rewrites::expr_rewrite_actions;
pub use irules_profiles::profiles_action;
pub use package_suggestions::{
    PackageRequireSource, package_require_actions, package_require_actions_in_program,
    package_require_source,
};
#[cfg(test)]
use package_suggestions::{original_source_package_already_named, package_named_by_namespace};
#[cfg(test)]
use spec_pack_fixes::statement_head_at;
pub use spec_pack_fixes::{SPEC_PACK_DIAGNOSTIC_CODE, spec_pack_quick_fixes};

/// LSP code-action kind.  Maps to the dotted strings the editor / e2e
/// `only` filter use (`quickfix`, `refactor.extract`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// `source` — generic source action.
    Source,
    /// `source.generateDocstring` — generate a procedure docstring.
    GenerateDocstring,
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
            Self::GenerateDocstring => "source.generateDocstring",
        }
    }
}

/// A command attached to a code action (e.g. the post-extract rename).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionCommand {
    /// Command identifier (e.g. `tclLsp.renameSymbolAtPosition`).
    pub command: String,
    /// Integer arguments (line / start / end for the rename command).
    pub args: Vec<u32>,
    /// String arguments — used by the BIG-IP actions whose command takes
    /// textual arguments rather than integer positions: the document `uri`
    /// (plus a bare partition name for `tclLsp.renamePartition`, or `uri`
    /// alone for `editor.action.rename`). Source-bound generated renames carry
    /// the complete generated source and original Name token before `args`.
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
            // presentation-ok: line-ending normalisation of the edit being applied.
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
    current: &DiagnosticEditSource<'_>,
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
        .filter(|rewrite| current.matches_optimiser_rewrite_group(&rewrite.members))
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
                .map(|member| crate::rename::TextEdit {
                    range: lsp_range(member),
                    new_text: member
                        .original_optimiser_rewrite()
                        .expect("checked whole issued rewrite group")
                        .replacement
                        .clone(),
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
    actions.extend(rewrite_actions(
        report,
        source,
        range,
        &current,
        &line_index,
    ));

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
///   partition-visibility model).  When the caller knows the document
///   `version` the stanza was read from, it travels as the command's one
///   integer argument (`[uri, partition, version]` on the wire), so the
///   deferred rename applies to that exact source or is refused — the
///   identity is the one the action was offered for, not whatever the
///   document holds when the user accepts it.
///
/// Returns an empty vector when the cursor is not inside a parseable
/// stanza or the document is not BIG-IP.  `range`'s start *line* selects
/// the object (keyed on `range.start.line`).
#[must_use]
pub fn bigip_code_actions(
    source: &str,
    range: LspRange,
    uri: &str,
    version: Option<u32>,
) -> Vec<CodeAction> {
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
    // for `/Common` (the query engine refuses that rename).  The partition
    // is the authored identifier itself, or the one component of an
    // absolute `/<partition>` spelling read by the shared object-path owner;
    // anything else (`//x`, `/a/b`) is not a partition name this action can
    // name, and the query engine still admits or refuses what it is sent.
    if stanza.module == "auth" && stanza.object_type == "partition" {
        let partition = if obj_path.starts_with('/') {
            tcl_bigip::object_names::object_path_partition(obj_path)
                .filter(|partition| partition.len() + 1 == obj_path.len())
        } else {
            Some(obj_path)
        };
        if let Some(partition_short) = partition.filter(|p| !p.is_empty() && *p != "Common") {
            actions.push(CodeAction {
                title: format!("Rename partition '{partition_short}'\u{2026}"),
                edits: Vec::new(),
                kind: ActionKind::RefactorRewrite,
                command: Some(ActionCommand {
                    command: "tclLsp.renamePartition".to_string(),
                    args: version.into_iter().collect(),
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
        if tcl_compiler::analyser::line_suppressed(
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
    if resolution.registry.is_none() {
        return Vec::new();
    }
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let registry = current.registry();
    let resolution = resolution.with_registry(registry);
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
            action.command =
                refactor::extract_proc_rename_command_for_analysis(&r, source, analysis);
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
            refactor::extract_to_datagroup_with_analysis(source, cursor, "", analysis, line_index)
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_docstring_tests;

#[cfg(test)]
mod original_package_suggestion_tests;

#[cfg(test)]
mod original_refactor_context_tests;

#[cfg(test)]
mod original_event_action_boundary_tests;

#[cfg(test)]
mod original_expression_action_tests;

#[cfg(test)]
mod package_presence_census_tests;

#[cfg(test)]
mod issued_optimiser_rewrite_currency_tests;

#[cfg(test)]
mod package_authoring_source_tests;
