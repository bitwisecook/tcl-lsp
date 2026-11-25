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

//! The workspace refinement of W123 and the cross-file argument-count check.

use std::collections::HashSet;
use std::sync::Arc;

#[cfg(test)]
use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_core::package_resolver::PackageResolver;
use tcl_lsp_core::{definition as core_definition, workspace_index as core_workspace_index};
use tcl_registry::CommandRegistry;
use tokio::sync::RwLock;

use crate::provider_source;

use super::consumers::WorkspaceCommandQueries;
use super::{CrossFileCalls, SourceInheritance};

/// Original declaration slots for package availability, independent of the
/// reporting maps used for completion labels.
#[cfg(test)]
pub(in super::super) fn defined_original_commands(
    text: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    let result = Analyser::new().structure_only().analyse(text, dialect.name);
    defined_original_commands_from_analysis(text, &result)
}

pub(in super::super) fn defined_original_commands_from_analysis(
    text: &str,
    result: &AnalysisResult,
) -> Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    let Some(input) = result.resolved_input.as_ref() else {
        return Vec::new();
    };
    if !result.matches_original_source_image(
        &tcl_lexer::SourceImage::document(text),
        input.lexer_config(),
    ) {
        return Vec::new();
    }
    let inventory =
        tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
            text, result,
        );
    let mut procedures = inventory.as_ref().map_or_else(
        || {
            result
                .original_procedure_declarations()
                .map(|record| record.name().clone())
                .collect::<Vec<_>>()
        },
        |inventory| {
            inventory
                .candidates(result.original_procedure_declarations())
                .into_iter()
                .map(|candidate| candidate.source_name())
                .collect()
        },
    );
    if let Some(inventory) =
        tcl_compiler::registry_invocation::source_structure::source_class_publications(text, result)
    {
        procedures.extend(
            inventory
                .candidates(result)
                .into_iter()
                .filter_map(|candidate| candidate.source_name()),
        );
    }
    procedures
}

/// Pure core of the workspace W123 refinement: drop every unknown-command
/// (W123) diagnostic whose command the package database can resolve.
///
/// A command is resolvable when either
/// * the scanned `auto_path` auto-loads it — a `tclIndex` maps its bare name,
///   the "command defined in library path" case (a BLT/Rbc-style
///   library whose procs auto-load with no `package require`), or
/// * one of the packages available to the document (`available`) defines it in
///   an implementation source file (a `pkgIndex`-only package with no
///   `tclIndex`).
///
/// This is the diagnostic dual of go-to-definition: a command the server can
/// resolve to a real library definition must not be reported "unknown". The
/// check is entirely data-driven — the resolver is queried *by the flagged
/// name*, never matched against a hard-coded command list.
///
/// # `pkgIndex.tcl` guards
///
/// A package whose `pkgIndex.tcl` gates its `package ifneeded` behind a
/// version guard supplies commands only on the releases that guard admits, so
/// the package database is queried for the release `dialect` names. The
/// three-way answer maps onto the suppression like this:
///
/// | Availability  | W123 | Why |
/// |---------------|------|-----|
/// | `Available`   | suppressed | the command really is provided |
/// | `Conditional` | suppressed | a guard the scan could not read; treating "unsure" as "absent" produces false unknown-command reports |
/// | `Unavailable` | **fires**  | the guard was read and definitely skips the declaration, so `package require` really fails and the call really is an error |
///
/// `Conditional` is suppressed outright rather than reported more quietly:
/// W123's own default severity is already `Hint`, the lowest LSP severity, so
/// there is no lower-confidence channel to downgrade into. Between a silent
/// miss and a false "unknown command" on working code, the miss is the lesser
/// error for a hint whose whole purpose is precision.
pub(in super::super) fn refine_original_w123_diagnostics(
    diags: Vec<tcl_compiler::analyser::Diagnostic>,
    available: &[tcl_lsp_core::package_resolver::PackageRequirementAdvice],
    resolver: &PackageResolver,
    sources: &provider_source::ProviderSourceAccess<'_>,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    use tcl_lsp_core::package_resolver::{
        PackageRequirementAdvice as Requirement, PackageRequirementAdviceKey as Key,
    };
    let target = tcl_registry::InvocationDialect::of_profile(dialect).tcl_version;
    let closure =
        resolver.transitive_requirement_advice_with_context(available, target, &|path, parent| {
            let unknown = || Requirement::new(Key::Unknown, Vec::new(), false, parent.prefer());
            let Ok(inventory) = sources.inventory(path, target, resolver) else {
                return vec![unknown()];
            };
            let analysis = &inventory.analysis;
            analysis
                .package_requires
                .iter()
                .map(|required| {
                    Requirement::from_source_requirement(
                        required,
                        tcl_lsp_core::package_resolver::package_prefer_at(
                            analysis,
                            required.range.start(),
                            parent.prefer(),
                        ),
                    )
                })
                .collect()
        });
    let incomplete = std::cell::Cell::new(
        closure
            .iter()
            .any(|requirement| requirement.key() == &Key::Unknown),
    );
    let commands = resolver.requirement_advice_defined_commands(
        &closure.into_iter().collect::<Vec<_>>(),
        target,
        &|path| {
            if let Ok(inventory) = sources.inventory(path, target, resolver) {
                defined_original_commands_from_analysis(&inventory.text, &inventory.analysis)
            } else {
                incomplete.set(true);
                Vec::new()
            }
        },
    );
    diags
        .into_iter()
        .filter(|diagnostic| {
            if diagnostic.code != DiagCode::W123 {
                return true;
            }
            if incomplete.get() {
                return false;
            }
            let Some(subject) = diagnostic.unresolved_command() else {
                return true;
            };
            !(resolver.auto_loads_original_command(subject.name_input(), subject.invocation())
                || resolver.package_defines_original_command(
                    subject.name_input(),
                    subject.invocation(),
                    &commands,
                ))
        })
        .collect()
}

/// Borrow the complete checked Logical reporting owner. Immutable structural
/// store currency is separate from its retained availability and lexer; a
/// nominal profile supplies neither original input nor Native source advice.
pub(in super::super) fn workspace_reporting_context<'a>(
    analysis: &'a AnalysisResult,
    registry: &CommandRegistry,
) -> Option<&'a tcl_registry::model::ContextRegistry> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let input = analysis.resolved_input.as_ref()?;
    let context = input.borrowed_context_registry();
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let config = analysis.body_lexer_config?;
    if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key()
        || !analysis.matches_original_source_image(image, config)
    {
        return None;
    }
    Some(context)
}

/// Existing workspace-symbol selection uses its complete source-owned
/// reporting context and checked source graph. W123 diagnostic settlement
/// below uses the stronger typed candidate request instead.
pub(in super::super) fn settle_call_against_workspace<'a>(
    index: &core_workspace_index::WorkspaceIndex,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    uri: &str,
    inv: &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    exports: &dyn tcl_lsp_core::namespace_import::NamespaceExportOracle,
    call_off: u32,
) -> Option<&'a str> {
    let context = workspace_reporting_context(analysis, registry)?;
    if core_definition::forced_import_shadows_call(
        analysis,
        core_definition::CallResolution::document_only().in_program(
            core_definition::ProgramExports {
                uri,
                oracle: exports,
            },
        ),
        &inv.name,
        &inv.resolution_candidates,
        call_off,
    ) {
        return None;
    }
    inv.resolution_candidates
        .iter()
        .map(String::as_str)
        .find(|candidate| {
            let has_builtin = context.resolve_command(candidate).is_some();
            if core_definition::indirection_pending_at(analysis, candidate, call_off) {
                return false;
            }
            index.workspace_command_exists_for_call(candidate, has_builtin)
        })
}

pub(in super::super) type ProcArityRange = (u32, Option<u32>);

pub(in super::super) type ProcArityUnion = Vec<ProcArityRange>;

/// Retain reporting settlement only for exact Logical source requests and
/// current external source declarations. Diagnostic names and workspace-wide
/// report-name sets do not supply lookup candidates.
pub(in super::super) fn settle_cross_file_calls(
    index: &core_workspace_index::WorkspaceIndex,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    uri: &str,
) -> CrossFileCalls {
    let mut out = CrossFileCalls::default();
    if workspace_reporting_context(analysis, registry).is_none() {
        return out;
    }
    let Some(source) = analysis
        .retained_command_realm()
        .and_then(|realm| realm.original_source_image())
        .and_then(|image| image.try_text().ok())
    else {
        return out;
    };
    let unresolved = analysis
        .unresolved_command_sites
        .iter()
        .map(|(span, _)| (span.start(), span.end()))
        .collect::<HashSet<_>>();
    out.command_queries = WorkspaceCommandQueries::Broad;
    for invocation in &analysis.command_invocations {
        let key = (invocation.range.start(), invocation.range.end());
        if invocation.is_mathfunc_call {
            let Some(occurrence) =
                analysis.logical_math_function_at(source, invocation.range.start())
            else {
                continue;
            };
            let Some(candidates) = index.logical_math_source_cards(&occurrence, analysis, uri)
            else {
                continue;
            };
            if !candidates.is_empty() {
                out.resolved.insert(key);
                out.source_candidates.extend(candidates);
            }
            continue;
        }
        if !unresolved.contains(&key) {
            continue;
        }
        let Some(request) =
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
                source,
                analysis,
                invocation.range.start(),
            )
        else {
            continue;
        };
        if request.has_earlier_unknown_mutations() || request.has_lookup_operand_effects() {
            continue;
        }
        let candidates = index
            .logical_command_candidates(&request, uri)
            .into_iter()
            .filter(|candidate| index.logical_command_candidate_is_current(candidate))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            continue;
        }
        out.resolved.insert(key);
        out.source_candidates.extend(candidates.iter().cloned());
        if candidates
            .iter()
            .all(|candidate| candidate.procedure_declaration().is_some())
        {
            let ranges = candidates
                .iter()
                .map(|candidate| {
                    candidate
                        .procedure_declaration()?
                        .formal_count_projection()
                        .map(tcl_compiler::signature_scan::formal_count::SourceFormalCountProjection::arity)
                })
                .collect::<Option<Vec<_>>>();
            if let Some(ranges) = ranges {
                let mut ranges = ranges
                    .into_iter()
                    .map(|arity| {
                        (
                            u32::from(arity.min),
                            (arity.max != tcl_registry::Arity::UNLIMITED)
                                .then(|| u32::from(arity.max)),
                        )
                    })
                    .collect::<Vec<_>>();
                ranges.sort_unstable();
                ranges.dedup();
                out.arity.insert(key, ranges);
            }
        }
    }
    out
}

/// Render a compact argument-count union for the E005 gap verdict.
fn display_arity_ranges(ranges: &[ProcArityRange]) -> String {
    ranges
        .iter()
        .map(|&(min, max)| match max {
            Some(max) if min == max => min.to_string(),
            Some(max) => format!("{min}..={max}"),
            None => format!("at least {min}"),
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// Argument-count diagnostics for explicitly selected Logical reporting calls.
/// Each compatible declaration retains its own accepted range; disjoint ranges
/// stay disjoint. Expanded or prefix callback counts remain unavailable. Native
/// callable identity, effective argv and argument topology require their own
/// original receipts and cannot be recovered from these reporting rows.
pub(in super::super) fn cross_file_arity_diagnostics(
    analysis: &AnalysisResult,
    calls: &CrossFileCalls,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    use tcl_compiler::analyser::types::Severity;
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for inv in &analysis.command_invocations {
        let key = (inv.range.start(), inv.range.end());
        let (Some(ranges), Some(argc)) = (calls.arity.get(&key), inv.argc) else {
            continue;
        };
        let argc = u32::try_from(argc).unwrap_or(u32::MAX);
        if ranges
            .iter()
            .any(|&(min, max)| argc >= min && max.is_none_or(|max| argc <= max))
        {
            continue;
        }
        let min = ranges.iter().map(|&(min, _)| min).min().unwrap_or(0);
        let max = ranges
            .iter()
            .filter_map(|&(_, max)| max)
            .max()
            .filter(|_| ranges.iter().all(|&(_, max)| max.is_some()));
        let (code, message) = if argc < min {
            (
                DiagCode::E002,
                format!(
                    "Too few arguments for '{}': expected at least {min}, got {argc}",
                    inv.name
                ),
            )
        } else if max.is_some_and(|max| argc > max) {
            (
                DiagCode::E003,
                format!(
                    "Too many arguments for '{}': expected at most {}, got {argc}",
                    inv.name,
                    max.unwrap_or(argc)
                ),
            )
        } else {
            (
                DiagCode::E005,
                format!(
                    "Wrong argument-count shape for '{}': expected {}, got {argc}",
                    inv.name,
                    display_arity_ranges(ranges),
                ),
            )
        };
        if is_disabled(code.as_str()) {
            continue;
        }
        out.push(tcl_compiler::analyser::Diagnostic {
            subject: None,
            signed_data_group_comparison: None,
            code,
            span: inv.range,
            message,
            severity: Severity::Error,
            fixes: Vec::new(),
        });
    }
    out
}

/// Refine W123 only when the exact Logical source owner selected a current
/// external declaration candidate. Display names and bare-tail equality do not
/// recreate source lookup or provider ownership.
pub(in super::super) fn refine_workspace_index_w123(
    diags: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    settled: &CrossFileCalls,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    if !analysis.allows_lexical_declaration_advice()
        || !diags.iter().any(|d| d.code == DiagCode::W123)
    {
        return diags;
    }
    diags
        .into_iter()
        .filter(|d| {
            if d.code != DiagCode::W123 {
                return true;
            }
            let span = (d.span.start(), d.span.end());
            !settled.resolved.contains(&span)
        })
        .collect()
}

/// Apply the workspace W123 refinement to `analyser_diags`: resolve
/// each unknown-command diagnostic against the shared package database and drop
/// any whose command an installed library / available package provides. Shared
/// by the push path ([`refine_and_lift_diagnostics`]) and the pull path
/// ([`Backend::full_diagnostics_for`]) so both stay behaviour-identical, and —
/// like the W120 refinement — always on, independent of the
/// `crossFileResolution` toggle: a library-provided command is ambient, like
/// a built-in, so suppressing its false "unknown" is pure precision, not a
/// cross-file inference the user opts into.
///
/// The common case (no W123) skips the resolver lock and all filesystem work.
pub(in super::super) async fn refine_workspace_w123(
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    inheritance: &SourceInheritance,
    package_resolver: &Arc<RwLock<PackageResolver>>,
    sources: &provider_source::ProviderSourceAccess<'_>,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    if !analyser_diags.iter().any(|d| d.code == DiagCode::W123) {
        return analyser_diags;
    }
    // A `source` whose target this server cannot pin to an
    // indexed document may define any command, so no name in this file can be
    // called unknown. Same bargain as `has_dynamic_providers`, applied at the
    // level that owns path resolution.
    if inheritance.unresolvable_source {
        return analyser_diags
            .into_iter()
            .filter(|d| d.code != DiagCode::W123)
            .collect();
    }
    if let Some(inherited) = &inheritance.original {
        use tcl_lsp_core::package_resolver::PackageRequirementAdvice as Requirement;
        let mut available = analysis
            .package_requires
            .iter()
            .map(|required| {
                Requirement::from_source_requirement(
                    required,
                    tcl_lsp_core::package_resolver::package_prefer_at(
                        analysis,
                        required.range.start(),
                        inherited.prefer,
                    ),
                )
            })
            .collect::<Vec<_>>();
        available.extend(inherited.ambient.iter().cloned());
        available.extend(inherited.placed.iter().map(|placed| placed.name.clone()));
        let resolver = package_resolver.read().await;
        return refine_original_w123_diagnostics(
            analyser_diags,
            &available,
            &resolver,
            sources,
            dialect,
        );
    }
    // The legacy metadata path below has only package-name strings. Keep a
    // Logical W123 when its source graph/key owner is unavailable.
    if analysis.allows_lexical_declaration_advice() {
        return analyser_diags;
    }
    // Packages available to the document: its own `package require`s (empty
    // whenever a W123 survived — the analyser drops every W123 once a file has
    // any `package require`) plus those inherited from entry points / `source`
    // ancestors and from the files it `source`s.
    //
    // Not position-gated, unlike the W120 path: this asks whether some package
    // *provides the command*, and a call to a command provided by a package
    // loaded later in the file is a runtime ordering bug, not an unknown
    // command — reporting it as "unknown" would be the wrong diagnostic.
    let mut available: Vec<String> = analysis
        .package_requires
        .iter()
        .map(|pr| pr.name.clone())
        .collect();
    available.extend(inheritance.ambient.iter().cloned());
    available.extend(inheritance.placed.iter().map(|p| p.name.clone()));
    let resolver = package_resolver.read().await;
    let roots = available
        .iter()
        .map(|name| {
            tcl_lsp_core::package_resolver::PackageRequirementAdvice::authored_metadata(
                name,
                tcl_lsp_core::package_resolver::PackagePrefer::Stable,
            )
        })
        .collect::<Vec<_>>();
    refine_original_w123_diagnostics(analyser_diags, &roots, &resolver, sources, dialect)
}
