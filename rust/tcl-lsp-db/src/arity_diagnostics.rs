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

//! Cross-file wrong-argument-count diagnostics for calls and callbacks, from the project's source headers.

use super::command_signatures::{
    LogicalSourceCommandSlot, OriginalCommandSlot, logical_source_command_arity,
    original_command_signatures, original_lookup_command_arities_with_mode,
    original_lookup_command_signatures,
};
use super::document_analysis::file_analysis_incremental;
use super::project_source_inputs::{
    original_command_signatures_for_inputs, original_lookup_command_signatures_for_inputs,
};
use super::{
    AnalyserConfig, Project, ProjectSourceInputMode, SourceFile, TclDb, project_token_inputs,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;

/// Build the cross-file wrong-argument-count diagnostic for a call to a workspace
/// proc whose arg count fits none of the proc's arities.  Reuses the analyser's
/// **own** arity codes — `E002` (too few) / `E003` (too many), `Severity::Error`,
/// same message shape — so a cross-file arity problem is classified, linked, and
/// disabled exactly like the local one.  (The code `W124` is the *unrelated*
/// invalid-IP-literal warning and must not be reused here.)
///
/// Returns `None` when `argc` sits inside the candidates' `(min, max)` envelope —
/// i.e. it fits some arity, or falls in a rare gap between disjoint same-tail
/// arities, which is too ambiguous to flag.
fn cross_file_arity_diagnostic(
    name: &str,
    span: tcl_lexer::Span,
    argc: (usize, Option<usize>),
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    use tcl_compiler::analyser::types::{Diagnostic, Severity};
    // `argc` is the caller's supplied arg-count RANGE `(lo, hi)`: an ordinary
    // call is exact (`(k, Some(k))`); a command-prefix callback with an
    // `AtLeast(n)` arity is open-ended (`(baked+n, None)`).  Flag too-few only
    // when even the MOST args the caller can supply (`hi`) is below the proc's
    // `min`, and too-many only when even the FEWEST args (`lo`) exceeds `max` —
    // so an open-ended callback never false-fires "too few".
    let (lo, hi) = argc;
    let min = candidates.iter().map(|&(lo, _)| lo).min()?;
    let max = candidates.iter().map(|&(_, hi)| hi).max()?;
    let (code, message) = if hi.is_some_and(|h| h < min) {
        (
            DiagCode::E002,
            format!(
                "Too few arguments for '{name}': expected at least {min}, got {}",
                hi.unwrap_or(lo)
            ),
        )
    } else if max != usize::MAX && lo > max {
        (
            DiagCode::E003,
            format!("Too many arguments for '{name}': expected at most {max}, got {lo}"),
        )
    } else {
        return None;
    };
    Some(Diagnostic {
        subject: None,
        signed_data_group_comparison: None,
        code,
        span,
        message,
        severity: Severity::Error,
        fixes: Vec::new(),
    })
}

/// Diagnose one exact callback argument count when no visible proc candidate
/// accepts it. Unlike the direct-call count envelope above, this also
/// handles a genuine gap between disjoint selected proc arities: every
/// possible target rejects the count, so E005 is the precise shape error.
fn callback_exact_arity_diagnostic(
    name: &str,
    span: tcl_lexer::Span,
    argc: usize,
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    use tcl_compiler::analyser::types::{Diagnostic, Severity};

    if candidates
        .iter()
        .any(|&(min, max)| min <= argc && argc <= max)
    {
        return None;
    }
    if let Some(diag) = cross_file_arity_diagnostic(name, span, (argc, Some(argc)), candidates) {
        return Some(diag);
    }
    Some(Diagnostic {
        subject: None,
        signed_data_group_comparison: None,
        code: DiagCode::E005,
        span,
        message: format!(
            "Wrong argument count for callback '{name}': no visible definition accepts {argc} arguments"
        ),
        severity: Severity::Error,
        fixes: Vec::new(),
    })
}

/// Local canonical signatures do not read the project table. Authenticated
/// external source names and unknown local targets use exact byte geometry;
/// explicit source refusals never borrow a surviving project declaration.
fn callback_source_target_signatures(
    db: &dyn TclDb,
    project: Project,
    selection: &tcl_compiler::analyser::SourceCallbackSignatureLookup,
    mode: ProjectSourceInputMode,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    use tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind as Kind;
    let original = selection.original();
    if let Some(target) = original.target() {
        match target.kind() {
            Kind::LocalProcedure => Some(Arc::new(Vec::new())),
            Kind::ExternalSourceName => {
                let key = OriginalCommandSlot::new(
                    db,
                    target.target_input().native_input()?.policy(),
                    target.source_slot().clone(),
                );
                match mode {
                    ProjectSourceInputMode::Standalone => {
                        original_command_signatures(db, project, key)
                    }
                    ProjectSourceInputMode::Supplied => {
                        original_command_signatures_for_inputs(db, project, key)
                    }
                }
            }
        }
    } else if original.permits_external_signature_lookup() {
        match mode {
            ProjectSourceInputMode::Standalone => {
                original_lookup_command_signatures(db, project, selection.prefix().lookup()?)
            }
            ProjectSourceInputMode::Supplied => original_lookup_command_signatures_for_inputs(
                db,
                project,
                selection.prefix().lookup()?,
            ),
        }
    } else {
        None
    }
}

#[cfg(test)]
pub(super) fn apply_original_callback_arity<'a>(
    db: &dyn TclDb,
    project: Project,
    out: &mut Vec<tcl_compiler::analyser::types::Diagnostic>,
    invocations: impl IntoIterator<
        Item = &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    >,
    is_disabled: impl Fn(&str) -> bool,
) {
    apply_original_callback_arity_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        out,
        invocations,
        is_disabled,
    );
}

fn apply_original_callback_arity_with_mode<'a>(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    out: &mut Vec<tcl_compiler::analyser::types::Diagnostic>,
    invocations: impl IntoIterator<
        Item = &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    >,
    is_disabled: impl Fn(&str) -> bool,
) {
    use tcl_compiler::analyser::{
        Diagnostic, DiagnosticSubject, Severity, SourceCallbackArityIssue as Issue,
        SourceCallbackAritySubject,
    };
    let (db, project, mode) = source;
    for invocation in invocations {
        let Some(selection) = invocation.original_callback_signature_lookup.as_ref() else {
            // A prefix's scope geometry alone does not own source occupancy.
            continue;
        };
        let Some(headers) = callback_source_target_signatures(db, project, selection, mode) else {
            continue;
        };
        let Some(subject) =
            SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &headers)
        else {
            continue;
        };
        let issue = subject.issue();
        let code = issue.code();
        if is_disabled(code.as_str()) {
            continue;
        }
        // Presentation is independent of exact retained name bytes and policy.
        let name = String::from_utf8_lossy(subject.prefix().name_input().bytes());
        let message = match issue {
            Issue::TooFew {
                supplied,
                expected_minimum,
            } => format!(
                "Too few arguments for '{name}': expected at least {expected_minimum}, got {supplied}"
            ),
            Issue::TooMany {
                supplied,
                expected_maximum,
            } => format!(
                "Too many arguments for '{name}': expected at most {expected_maximum}, got {supplied}"
            ),
            Issue::NoCompatibleSignature { supplied } => format!(
                "Wrong argument count for callback '{name}': no visible definition accepts {supplied} arguments"
            ),
        };
        out.push(
            Diagnostic::new(code, subject.span(), message, Severity::Error)
                .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject))),
        );
    }
}

/// Standalone project source-header advice for retained original occurrences.
/// Foreign headers use each file's explicit compatibility defaults. Use
/// [`project_diagnostics_for_inputs`] for checked per-file project inputs.
/// Native source lookups keep their independent retained policy/slot purpose;
/// Logical calls require their genuine Document/body frame recipe and complete
/// original argv. Missing ownership and earlier opaque mutations preserve the
/// original diagnostics. Project headers supply no installed lookup or entry.
#[salsa::tracked(returns(clone))]
pub fn project_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    // The analyser's production skip: the synthesised arity codes honour the
    // same production skip as the analyser's own; whether they show is the
    // policy step's decision.
    let disabled = config.disabled_diagnostics(db);
    // `file_analysis_incremental` (not the coarse `file_analysis`) so this reuses
    // the per-item firewall result the diagnostics worker already computed for
    // `file` — a cache hit, not a second whole-file analysis.
    let analysis = file_analysis_incremental(db, file, config);

    Arc::new(project_source_diagnostics_for_analysis(
        (db, project, ProjectSourceInputMode::Standalone),
        file.text(db),
        &analysis,
        |code| disabled.iter().any(|disabled| disabled == code),
    ))
}

/// Project source-header advice using every member's checked document input.
/// Missing member configuration or source ownership preserves diagnostics;
/// compatibility defaults never replace an unavailable supplied header.
#[salsa::tracked(returns(clone))]
pub fn project_diagnostics_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    let analysis = file_analysis_incremental(db, file, config);
    if project_token_inputs::configuration_for_file(db, project, file) != Some(config) {
        return Arc::new(analysis.diagnostics.clone());
    }
    Arc::new(project_source_diagnostics_for_analysis(
        (db, project, ProjectSourceInputMode::Supplied),
        file.text(db),
        &analysis,
        |code| {
            config
                .disabled_diagnostics(db)
                .iter()
                .any(|disabled| disabled == code)
        },
    ))
}

type OriginalSourceAritySites = HashMap<(u32, u32), Option<Arc<Vec<(usize, usize)>>>>;

type LogicalSourceAritySites = BTreeMap<
    (u32, u32),
    (
        tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        Arc<Vec<(usize, usize)>>,
    ),
>;

fn project_source_arity_sites(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    unresolved: &HashSet<(u32, u32)>,
    logical_source: bool,
) -> (OriginalSourceAritySites, LogicalSourceAritySites) {
    let (db, project, mode) = source;
    let original_sites = analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            !logical_source
                && invocation.original_name_input.is_some()
                && unresolved.contains(&(invocation.range.start(), invocation.range.end()))
        })
        .map(|invocation| {
            (
                (invocation.range.start(), invocation.range.end()),
                invocation.original_lookup.as_ref().and_then(|lookup| {
                    original_lookup_command_arities_with_mode((db, project, mode), lookup)
                }),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut logical_sites = BTreeMap::new();
    for candidate in
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
            text, analysis,
        )
        .into_iter()
        .flatten()
    {
        let Some(head) = candidate.original_head() else {
            continue;
        };
        let span = head.span();
        let key = (span.start(), span.end());
        if original_sites.contains_key(&key) {
            // A present original Native source owner is terminal even when it
            // cannot supply a project signature.
            continue;
        }
        let Some(coordinates) = candidate.external_header_coordinates() else {
            continue;
        };
        if candidate.has_earlier_unknown_mutations() || candidate.has_lookup_operand_effects() {
            continue;
        }
        let Some(arities) = coordinates.iter().find_map(|slot| {
            logical_source_command_arity(
                db,
                project,
                LogicalSourceCommandSlot::new(db, slot.clone()),
                matches!(mode, ProjectSourceInputMode::Supplied),
            )
        }) else {
            continue;
        };
        logical_sites.insert(key, (candidate, arities));
    }
    (original_sites, logical_sites)
}

fn retained_project_diagnostics(
    analysis: &AnalysisResult,
    original_sites: &OriginalSourceAritySites,
    logical_sites: &LogicalSourceAritySites,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let mut diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code != DiagCode::W123
                || (!original_sites.contains_key(&(diagnostic.span.start(), diagnostic.span.end()))
                    && !logical_sites
                        .contains_key(&(diagnostic.span.start(), diagnostic.span.end())))
        })
        .cloned()
        .collect::<Vec<_>>();
    diagnostics.extend(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == DiagCode::W123
                    && original_sites
                        .get(&(diagnostic.span.start(), diagnostic.span.end()))
                        .is_some_and(Option::is_none)
            })
            .cloned(),
    );
    diagnostics
}

pub(super) fn project_source_diagnostics_for_analysis(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let (db, project, mode) = source;
    let Some(input) = analysis.resolved_input.as_ref() else {
        return analysis.diagnostics.clone();
    };
    if !analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(text),
        input.lexer_config(),
    ) {
        return analysis.diagnostics.clone();
    }
    // Each original coordinate is demanded independently so unrelated header
    // changes keep the per-slot query's early cutoff.
    let unresolved = analysis
        .unresolved_command_sites
        .iter()
        .map(|(span, _)| (span.start(), span.end()))
        .collect::<std::collections::HashSet<_>>();
    let logical_source = analysis.allows_retained_logical_declaration_advice();
    let (original_sites, logical_sites) = project_source_arity_sites(
        (db, project, mode),
        text,
        analysis,
        &unresolved,
        logical_source,
    );
    let mut diagnostics = retained_project_diagnostics(analysis, &original_sites, &logical_sites);
    for invocation in &analysis.command_invocations {
        let Some(Some(candidates)) =
            original_sites.get(&(invocation.range.start(), invocation.range.end()))
        else {
            continue;
        };
        let Some(argc) = invocation.argc else {
            continue;
        };
        if !candidates.is_empty()
            && let Some(diagnostic) = cross_file_arity_diagnostic(
                &invocation.name,
                invocation.range,
                (argc, Some(argc)),
                candidates,
            )
            && !is_disabled(diagnostic.code.as_str())
        {
            diagnostics.push(diagnostic);
        }
    }
    for (candidate, arities) in logical_sites.values() {
        if arities.is_empty() {
            continue;
        }
        let Some(argc) = candidate.argument_count() else {
            continue;
        };
        let Some(head) = candidate.original_head() else {
            continue;
        };
        let Some(name) = tcl_syntax::word_rules::original_static_word_unicode_value(head) else {
            continue;
        };
        let name = String::from_utf8_lossy(&name);
        if let Some(diagnostic) =
            cross_file_arity_diagnostic(&name, head.span(), (argc, Some(argc)), arities)
            && !is_disabled(diagnostic.code.as_str())
        {
            diagnostics.push(diagnostic);
        }
    }
    apply_logical_source_callback_arity(
        (db, project, mode),
        text,
        analysis,
        &mut diagnostics,
        &is_disabled,
    );
    apply_original_callback_arity_with_mode(
        (db, project, mode),
        &mut diagnostics,
        retained_callback_invocations(analysis),
        &is_disabled,
    );
    diagnostics
}

/// Original callback rows that still belong to this analysis's complete input.
/// Reissuing the existing lower-owner join also verifies its local canonical
/// declaration, independently of mutable invocation labels and count fields.
fn retained_callback_invocations(
    analysis: &AnalysisResult,
) -> impl Iterator<Item = &tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis.command_invocations.iter().filter(|invocation| {
        let Some(selection) = invocation.original_callback_signature_lookup.as_ref() else {
            return false;
        };
        tcl_compiler::analyser::SourceCallbackSignatureLookup::from_original_lookup(
            analysis,
            Arc::new(selection.prefix().clone()),
            Arc::new(selection.original().clone()),
        )
        .as_ref()
            == Some(selection.as_ref())
    })
}

/// Add project callback signature advice to an already analysed complete source.
/// The source channel, lexer configuration and retained input must agree;
/// foreign or missing ownership preserves the supplied analyser diagnostics.
/// Known source barriers and missing registration horizons never borrow tails.
/// Local canonical headers and exact held external names share the original
/// callback owner. Logical prefixes require their independently selected source
/// role, frame recipe and exact declaration coordinate; reported names and counts
/// supply no lookup assistance.
/// This supplies no installed callback, future entry or execution verdict.
/// Project headers in this compatibility entry use the explicit standalone
/// provider. Document callers use [`project_callback_diagnostics_for_analysis_with_inputs`].
#[must_use]
pub fn project_callback_diagnostics_for_analysis(
    db: &dyn TclDb,
    project: Project,
    source: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    project_callback_diagnostics_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        source,
        analysis,
        is_disabled,
    )
}

fn apply_logical_source_callback_arity(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    out: &mut Vec<tcl_compiler::analyser::Diagnostic>,
    is_disabled: impl Fn(&str) -> bool,
) {
    let (db, project, mode) = source;
    let Some(callbacks) =
        tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
            text, analysis,
        )
    else {
        return;
    };
    for callback in callbacks {
        // Earlier opaque mutations retain their unresolved alternatives. A
        // conditional source prefix is not permission to discard that risk.
        if callback.registration().obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ) { continue; }
        if callback.has_lookup_operand_effects() {
            continue;
        }
        let appended = callback.appended_arity();
        if !appended.is_checkable() {
            continue;
        }
        let Some(coordinates) = callback.external_header_coordinates() else {
            continue;
        };
        let Some(candidates) = coordinates.iter().find_map(|slot| {
            logical_source_command_arity(
                db,
                project,
                LogicalSourceCommandSlot::new(db, slot.clone()),
                matches!(mode, ProjectSourceInputMode::Supplied),
            )
        }) else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }
        let Some(span) = callback.head().source_span() else {
            continue;
        };
        let baked = callback.baked_argument_count();
        let name = callback.head().value();
        let diagnostic = if let Some(counts) = appended.exact_counts() {
            counts
                .map(|count| baked + usize::from(count))
                .find_map(|count| callback_exact_arity_diagnostic(name, span, count, &candidates))
        } else {
            let minimum = baked + appended.min() as usize;
            let maximum = appended.max().map(|maximum| baked + maximum as usize);
            cross_file_arity_diagnostic(name, span, (minimum, maximum), &candidates)
        };
        if let Some(diagnostic) = diagnostic
            && !is_disabled(diagnostic.code.as_str())
        {
            out.push(diagnostic);
        }
    }
}

pub(super) fn project_callback_diagnostics_with_mode(
    project_input: (&dyn TclDb, Project, ProjectSourceInputMode),
    source: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let (db, project, mode) = project_input;
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let Some(input) = analysis.resolved_input.as_ref() else {
        return analysis.diagnostics.clone();
    };
    if !analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        input.lexer_config(),
    ) {
        return analysis.diagnostics.clone();
    }
    let mut diagnostics = analysis.diagnostics.clone();
    apply_logical_source_callback_arity(
        (db, project, mode),
        source,
        analysis,
        &mut diagnostics,
        &is_disabled,
    );
    apply_original_callback_arity_with_mode(
        (db, project, mode),
        &mut diagnostics,
        retained_callback_invocations(analysis),
        is_disabled,
    );
    diagnostics
}

/// Opt-in callback signature diagnostics from the same supplied-analysis owner.
/// Direct calls retain the server's separate workspace-index settlement.
#[salsa::tracked(returns(clone))]
pub fn project_callback_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    let disabled = config.disabled_diagnostics(db);
    let analysis = file_analysis_incremental(db, file, config);
    Arc::new(project_callback_diagnostics_for_analysis(
        db,
        project,
        file.text(db),
        &analysis,
        |code| disabled.iter().any(|disabled| disabled == code),
    ))
}
