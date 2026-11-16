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
use crate::diagnostic_policy::{Finding, Fix, Report};

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

/// Explicit new-source declaration, without package installation or execution identity.
pub struct PackageRequireSource {
    /// Complete proposed source, or the unchanged source for an existing requirement.
    pub source: String,
    /// Whether a genuine current source reference already names the intended value.
    pub already_required: bool,
}

/// Propose one requested declaration under the complete current source input.
/// Explicit Unicode source-value comparison remains separate from Native keys.
#[must_use]
pub fn package_require_source(
    source: &str,
    analysis: &AnalysisResult,
    package: &str,
) -> Option<PackageRequireSource> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let word = tcl_syntax::word_rules::lexical_unicode_source_word(package, current.config())?;
    let command = format!("package require {word}\n");
    // This fresh proposed-source owner grants authoring metadata, not a Native entry.
    let owner =
        tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
            &command,
            analysis.resolved_input.as_ref()?.clone(),
        );
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        &command,
        0,
        current.config(),
    );
    let [segment] = commands.as_slice() else {
        return None;
    };
    let words = owner.words(segment)?;
    let reference = words.package_reference(&current.context())?;
    if reference.kind() != tcl_registry::source_navigation::SourcePackageReferenceKind::Require
        || reference.matches_unicode_source_value(
            &tcl_lexer::SourceImage::document(&command),
            current.config(),
            package,
        ) != Some(true)
    {
        return None;
    }
    let census = crate::source_structure::SourcePackagePresenceCensus::capture(source, analysis)?;
    let already_required = census.already_names_unicode_source_value(source, analysis, package)?;
    if already_required {
        return Some(PackageRequireSource {
            source: source.to_owned(),
            already_required,
        });
    }
    let offset = tcl_compiler::registry_invocation::source_structure::original_package_require_insert_offset(source, analysis)?;
    let offset = usize::try_from(offset).ok()?;
    let (before, after) = (source.get(..offset)?, source.get(offset..)?);
    let separator = if !before.is_empty() && !before.ends_with('\n') {
        "\n"
    } else {
        ""
    };
    Some(PackageRequireSource {
        source: format!("{before}{separator}{command}{after}"),
        already_required,
    })
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
    let ownership = context.namespace_package_ownership();
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
            let Some(package) = package_owned_by_original_input(input, &ownership) else {
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
        let Some(package) = package_owned_by_logical_namespace(head, &ownership) else {
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

// Actual selected descriptor ownership supplies a package hint only. The
// independently retained input/absence/source/grammar gates remain above.
fn package_owned_by_original_input(
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    ownership: &tcl_registry::model::NamespacePackageOwnership,
) -> Option<String> {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};
    match input.policy().recipe() {
        protocol @ NativeNameProtocol::C(_) => {
            let slot = protocol
                .command_lookup_slot(NativeNameContext::root(), input.bytes())
                .ok()?;
            // Only an analytical presentation for descriptor ownership. It
            // cannot recreate the original slot or grant Native namespace use.
            let namespace = tcl_syntax::naming::native_namespace_full_name_bytes(&slot.namespace);
            let namespace = std::str::from_utf8(&namespace).ok()?;
            ownership
                .package_for_namespace(namespace)
                .map(str::to_owned)
        }
        protocol @ NativeNameProtocol::Jim084 => {
            // Retain Jim's actual flat key; no C namespace path is fabricated.
            let keys = protocol
                .jim_command_lookup_keys(NativeNameContext::root(), input.bytes())
                .ok()?;
            ownership
                .package_for_flat_key(keys.first()?.as_bytes())
                .map(str::to_owned)
        }
    }
}

fn package_owned_by_logical_namespace(
    head: &str,
    ownership: &tcl_registry::model::NamespacePackageOwnership,
) -> Option<String> {
    let written = tcl_syntax::naming::canonical_written_command(head);
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(&written);
    ownership.package_for_namespace(holder).map(str::to_owned)
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
    let census = crate::source_structure::SourcePackagePresenceCensus::capture(source, analysis)?;
    crate::source_structure::SourcePackagePresenceCensus::already_names(
        &census, source, analysis, package,
    )
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

/// Match the first written qualifier to a catalogue package. This fixture
/// projection supplies no package availability or command identity.
#[cfg(test)]
fn package_named_by_namespace(head: &str, catalogue: &[String]) -> Option<String> {
    let parts = tcl_syntax::naming::written_command_parts(head.as_bytes());
    let namespace = std::str::from_utf8(parts.qualifiers.first()?).ok()?;
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
    use tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind;
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
        out.push((
            CodeAction {
                title: "Apply De Morgan's law".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::DeMorgan,
        ));
    }
    if let Some(rewritten) = invert_comparison(&sel) {
        out.push((
            CodeAction {
                title: "Invert comparison".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::InvertComparison,
        ));
    }
    if !analysis.allows_retained_logical_declaration_advice() {
        for (action, kind) in &mut out {
            if start == extent.start()
                && finish == extent.end()
                && original_operator_rewrite_allowed(
                    source,
                    analysis,
                    extent,
                    &action.edits[0].new_text,
                    *kind,
                )
            {
                continue;
            }
            action.edits.clear();
            action.disabled = Some("missing-expression-operator-rewrite-permission: Original expression roles provide source geometry; Native or hosted operator rewrites require an independent selected evaluation and replacement contract".to_owned());
        }
    }
    out.into_iter().map(|(action, _)| action).collect()
}

fn original_operator_rewrite_allowed(
    source: &str,
    analysis: &AnalysisResult,
    extent: tcl_lexer::Span,
    candidate: &str,
    kind: tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind,
) -> bool {
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let commands =
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let Some(words) = crate::original_invocation::source_registry_words(source, analysis, command)
    else {
        return false;
    };
    let Some(roles) = words.roles.as_ref() else {
        return false;
    };
    roles.iter().any(|&(argument, role)| {
        role == tcl_registry::ArgRole::Expr
            && tcl_compiler::registry_invocation::source_structure::original_expression_operator_rewrite(
                source, analysis, command, argument, candidate, kind,
            ).is_some_and(|proof| proof.original_operand().content_span().ok() == Some(extent))
    })
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
                let Some(content) = original_literal_procedure_body(source, analysis, proc_def)
                else {
                    continue;
                };
                body_docstring_edit(source, line_index, content)
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
            kind: ActionKind::GenerateDocstring,
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
) -> Option<tcl_lexer::Span> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    let input = analysis.retained_command_realm().and_then(|realm| {
        realm.original_written_name_input_at_span_in_source(&image, proc_def.body_span, config)
    })?;
    let key = input.original_word_key()?;
    let word = key.original_word();
    (word.image() == &image
        && word.config() == config
        && word.group().kind == tcl_lexer::WordKind::Braced
        && !word.group().expand
        && word
            .tokens()
            .first()
            .is_some_and(|token| token.span == proc_def.body_span))
    .then_some(())?;
    word.content_span().ok()
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
    content: tcl_lexer::Span,
) -> DocstringInsertion {
    let body_start = content.start();
    let body_start_idx = body_start as usize;
    let body_end_idx = content.end() as usize;
    let body_text = source.get(body_start_idx..body_end_idx).unwrap_or("");
    // The authenticated content starts after its genuine opening delimiter.
    // Read indentation from the first new content line; a single-line body
    // uses the formatter's default below.
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
        let vocabulary = crate::oo_body::definition_grammar_from_analysis(source, analysis, offset);
        let grammar = crate::source_structure::SourceDefinitionGrammar::for_selected_source(
            &vocabulary,
            source,
            analysis,
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
    deduplicate_context_actions(&mut out);
    out
}

fn deduplicate_context_actions(actions: &mut Vec<CodeAction>) {
    let mut seen = FxHashSet::default();
    actions.retain(|action| {
        action.disabled.is_some()
            || seen.insert((
                action.kind,
                action.edits.clone(),
                action.command.clone(),
                action.data_group_definition.clone(),
            ))
    });
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
    let Some(choices) = bootstrap_collect_choices(
        analysis, registry, commands, diagnostic, enclosing, event, &events,
    ) else {
        return Vec::new();
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

fn bootstrap_collect_choices(
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    commands: &[diagnostic_context::SourceCommand],
    diagnostic: &tcl_compiler::compiler_checks::Diagnostic,
    enclosing: &diagnostic_context::SourceCommand,
    event: &str,
    events: &EventRegistry,
) -> Option<Vec<(&'static str, &'static str)>> {
    Some(if diagnostic.code.as_str() == "IRULE1005" {
        // The issuer's exact event operand must belong to the selected header.
        if enclosing
            .words
            .get(1)
            .is_none_or(|word| word.span() != diagnostic.span)
        {
            return None;
        }
        let setup = events
            .get_props(event)
            .and_then(|properties| properties.setup_event)?;
        let (protocols, _) = events.data_collect_requirement(event)?;
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
        let operation = operations.next()?;
        if operations.next().is_some() {
            return None;
        }
        let setup = operation.protocol.bootstrap_event_for(event)?;
        registry
            .data_collection_collect_command(operation.protocol.name)
            .filter(|spec| source_action_spec(analysis, registry, spec.name).is_some())
            .map_or_default(|spec| vec![(spec.name, setup)])
    })
}

/// Remove the exact selected encoder substitution around `read`. A nearby
/// bracket or a different variable is never an anchor.
fn redundant_encoder_action(
    source: &str,
    command: &diagnostic_context::SourceCommand,
    read: tcl_lexer::Span,
    matched: &str,
) -> Vec<CodeAction> {
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
    let Some(command) = diagnostic_context::selected_sink(commands, analysis, diagnostic) else {
        return Vec::new();
    };
    // This hardening changes subst's accepted input, independently of a
    // semantics-preserving refactoring. The selected source head, option
    // grammar and written insertion extent must all be current.
    // registry-axis-ok: command — the hardening edit rewrites subst's own option word; until the substitution option table offers the hardening edit
    if diagnostic.code.as_str() == "T100" && command.canonical == "subst" {
        return subst_nocommands_action(source, analysis, registry, command);
    }
    // Other rewrites introduce or remove evaluations. Authored vendor and
    // explicit lexical source advice may offer reviewed hardening; a Native
    // input offers it only where the retained command world proves every
    // inserted command at the point it runs.
    let native = !analysis.has_original_vendor_source_names()
        && !analysis.allows_retained_logical_declaration_advice();
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
            if source_action_spec(analysis, registry, "string").is_none()
                || native
                    && !native_command_proved(analysis, registry, "string", command.span.start())
            {
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
        "T106" if !native => redundant_encoder_action(source, command, read, matched),
        "T103" if native => {
            if !native_regex_quote_publication_proved(analysis, registry, command.span.start()) {
                return Vec::new();
            }
            vec![context_action(
                format!("Wrap ${variable} with [regex::quote]"),
                vec![
                    source_edit(
                        source,
                        tcl_lexer::Span::new(0, 0),
                        format!("{REGEX_QUOTE_PROC}\n"),
                    ),
                    source_edit(source, read, format!("[::regex::quote {matched}]")),
                ],
            )]
        }
        "IRULE3001" | "IRULE3002" | "T103" if !native => {
            let (encoder, template) = match diagnostic.code.as_str() {
                "IRULE3001" => ("html_encode", Some(HTML_ENCODE_PROC)),
                "IRULE3002" => ("URI::encode", None),
                _ => ("regex::quote", Some(REGEX_QUOTE_PROC)),
            };
            let call = format!("[::{encoder} {matched}]");
            let mut edits = Vec::new();
            if let Some(template) = template {
                let Some(proposal) = diagnostic_context::helper_publication_proposal(
                    source, analysis, template, &call,
                ) else {
                    return Vec::new();
                };
                if proposal.template().bytes() != template.as_bytes()
                    || proposal.call().bytes() != call.as_bytes()
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
            edits.push(source_edit(source, read, call));
            vec![context_action(
                format!("Wrap ${variable} with [{encoder}]"),
                edits,
            )]
        }
        _ => Vec::new(),
    }
}

/// The retained command world proves `name` resolves to its own registry
/// command at `at`: no shadowing procedure, alias, rename or unknown lookup.
fn native_command_proved(
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    name: &str,
    at: u32,
) -> bool {
    use tcl_registry::model::{BindingKnowledge, BindingTarget, SpecKey};
    let (Some(spec), Some(realm), Some(input)) = (
        source_action_spec(analysis, registry, name),
        analysis.retained_command_realm(),
        analysis.resolved_input.as_ref(),
    ) else {
        return false;
    };
    realm.knowledge_at(input.availability_context(), registry, name, at)
        == BindingKnowledge::Must(BindingTarget::Spec(SpecKey::new(spec)))
}

/// No binding of `name` is proven at `at`, and the document never mutates
/// its command table opaquely. An open world leaves an undefined name
/// `Unknown` (`auto_load` may find it), but a `proc` the edit runs first is
/// the definition a later call in this document reaches either way.
fn native_command_unclaimed(
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    name: &str,
    at: u32,
) -> bool {
    use tcl_registry::model::BindingKnowledge;
    let (Some(realm), Some(input)) = (
        analysis.retained_command_realm(),
        analysis.resolved_input.as_ref(),
    ) else {
        return false;
    };
    match realm.knowledge_at(input.availability_context(), registry, name, at) {
        BindingKnowledge::Absent => true,
        BindingKnowledge::Unknown => analysis.root_unknown_command_mutation() == Some(false),
        BindingKnowledge::Must(_) | BindingKnowledge::May(_) => false,
    }
}

/// Publishing [`REGEX_QUOTE_PROC`] at the top of a Native document: its
/// `namespace` and `proc` are the builtins there, the document itself defines
/// neither `::regex::quote` nor `::regex::regsub`, and its `regsub` resolves
/// to the builtin at the sink. A same-named helper defined outside the
/// document is replaced, which this document cannot see.
fn native_regex_quote_publication_proved(
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    sink: u32,
) -> bool {
    native_command_proved(analysis, registry, "namespace", 0)
        && native_command_proved(analysis, registry, "proc", 0)
        && native_command_unclaimed(analysis, registry, "::regex::quote", sink)
        && native_command_unclaimed(analysis, registry, "::regex::regsub", sink)
        && native_command_proved(analysis, registry, "regsub", sink)
}

fn subst_nocommands_action(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
    command: &diagnostic_context::SourceCommand,
) -> Vec<CodeAction> {
    let Some(spec) = source_action_spec(analysis, registry, &command.canonical) else {
        return Vec::new();
    };
    if !spec
        .options
        .iter()
        // registry-axis-ok: options — the hardening edit rewrites subst's own option word; until the substitution option table offers the hardening edit
        .any(|option| option.name == "-nocommands")
        || command
            .words
            .iter()
            .skip(1)
            // registry-axis-ok: options — the hardening edit rewrites subst's own option word; until the substitution option table offers the hardening edit
            .any(|word| diagnostic_context::source_literal(word) == Some("-nocommands"))
    {
        return Vec::new();
    }
    let Some(head) = command.words.first() else {
        return Vec::new();
    };
    vec![context_action(
        "Add -nocommands to disable command substitution".to_owned(),
        vec![source_edit(
            source,
            tcl_lexer::Span::new(head.span().end(), head.span().end()),
            " -nocommands".to_owned(),
        )],
    )]
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
