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

//! Whether the dialect has the command at all: an unknown or ambiguous subcommand (W001, W145), a disabled command (W002) and a context gate (W142).

use super::UserResolutionFacts;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SpecSurface;

/// Format the "(available in: …)" suffix for a W002 message from a
/// command/subcommand's registry-declared dialect restriction — entirely
/// data-driven from [`tcl_registry::CommandSpec::surface`] /
/// [`tcl_registry::SubCommand::surface`], never a per-command name check.
/// Empty when `surface` is `None` (unrestricted) or has no primitive
/// member (defensive; a restricted spec always has at least one).
///
/// Members print as the profile catalogue's `short_name` ("Tcl 8.5",
/// "iRules") — this suffix is the highest-traffic dialect naming in
/// editor-visible prose, so it uses the human spelling; a member with no
/// catalogue profile keeps its canonical name.
pub(in crate::analyser::diagnostics) fn dialect_availability_suffix(
    surface: Option<&'static [SpecSurface]>,
) -> String {
    surface.map_or_else(String::new, dialect_availability_suffix_for_rows)
}

/// [`dialect_availability_suffix`] over surface rows the caller already holds.
fn dialect_availability_suffix_for_rows(rows: &[SpecSurface]) -> String {
    // The dialect ids the rows name, in the registry's own projection —
    // the reader wants the dialects they can select, not the row spelling.
    let labels: Vec<String> = tcl_registry::model::surface::dialect_names_for_rows(rows)
        .into_iter()
        .map(|name| {
            tcl_dialect::DialectProfile::find(&name)
                .map_or(name.clone(), |profile| profile.short_name.to_owned())
        })
        .collect();
    if labels.is_empty() {
        return String::new();
    }
    format!(" (available in: {})", labels.join(", "))
}

impl Analyser {
    /// Whole-command availability advice from retained original source.
    /// Later original declarations can withdraw nominal Registry metadata.
    pub(in crate::analyser) fn emit_w002_disabled_command(
        &mut self,
        original: Option<crate::registry_invocation::OriginalSourceCommandAvailability>,
        scope_path: &[usize],
    ) {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        use tcl_registry::model::CommandSourceUnavailabilityKind;
        let Some(original) = original else {
            return;
        };
        let descriptor = original.descriptor();
        let reporting_name = original
            .original_head()
            .try_text()
            .unwrap_or_default()
            .to_owned();
        let (code, message) = match descriptor.kind() {
            CommandSourceUnavailabilityKind::RuleLoaderRefused => (
                DiagCode::Irule2004,
                format!(
                    "'{reporting_name}' is refused by the authored iRules rule-loader surface when written literally in rule source."
                ),
            ),
            CommandSourceUnavailabilityKind::Unavailable => {
                let suffix = dialect_availability_suffix_for_rows(&descriptor.providers().rows);
                (
                    DiagCode::W002,
                    format!("'{reporting_name}' is disabled in the active dialect profile{suffix}"),
                )
            }
        };
        let diag = crate::analyser::types::Diagnostic::new(
            code,
            original.original_head().span(),
            message,
            Severity::Warning,
        )
        .with_subject(crate::analyser::DiagnosticSubject::CommandAvailability(
            std::sync::Arc::new(original),
        ));
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        self.pending_disabled_commands
            .push((reporting_name, ns, enforce_order, diag));
    }

    /// W001, W002 and W145 selector diagnostics from original source schema.
    /// The retained context owns identity, availability and prefix resolution;
    /// a captured, dynamic or expanded selector has no written subject here.
    /// Suggestions replace the complete original selector word and need review.
    pub(in crate::analyser) fn emit_w001_unknown_subcommand(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_subcommand_diagnostic,
        };
        use tcl_registry::AuthoredSourceSubcommandDiagnostic;

        let Some(original) = original.filter(|original| !original.is_captured(0)) else {
            return;
        };
        let Some(word) = original.literal(0) else {
            return;
        };
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::Subcommand, Some(0))
        else {
            return;
        };
        let Some(outcome) = original.with_schema(source_subcommand_diagnostic).flatten() else {
            return;
        };
        let command = original.command();
        let span = original
            .word(0)
            .expect("subject retains original word")
            .span();
        let (code, message, fixes) = match outcome {
            AuthoredSourceSubcommandDiagnostic::Disabled { canonical, surface } => (
                DiagCode::W002,
                format!(
                    "'{command} {canonical}' is disabled in the active dialect profile{}",
                    dialect_availability_suffix(surface)
                ),
                Vec::new(),
            ),
            AuthoredSourceSubcommandDiagnostic::Unknown { candidates } => {
                let mut message = format!("Unknown subcommand '{word}' for '{command}'");
                let suggestions = crate::text::suggest_similar(
                    word,
                    candidates.iter().copied(),
                    1,
                    crate::text::scaled_max_distance(word),
                );
                let fixes = suggestions
                    .first()
                    .and_then(|best| {
                        use std::fmt::Write as _;
                        let _ = write!(message, "; did you mean '{best}'?");
                        original.written_index(0)?;
                        let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                            best,
                            original.word(0)?.config(),
                        )?;
                        Some(vec![super::super::types::CodeFix {
                            span,
                            new_text,
                            description: format!("Replace with '{best}'"),
                            safety: crate::irules_checks::FixSafety::RequiresReview,
                        }])
                    })
                    .unwrap_or_default();
                (DiagCode::W001, message, fixes)
            }
            AuthoredSourceSubcommandDiagnostic::Ambiguous { candidates } => {
                let listed = candidates
                    .iter()
                    .map(|candidate| format!("'{candidate}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let fixes = candidates
                    .into_iter()
                    .filter_map(|candidate| {
                        original.written_index(0)?;
                        let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                            candidate,
                            original.word(0)?.config(),
                        )?;
                        Some(super::super::types::CodeFix {
                            span,
                            new_text,
                            description: format!("Expand to '{command} {candidate}'"),
                            safety: crate::irules_checks::FixSafety::RequiresReview,
                        })
                    })
                    .collect();
                (
                    DiagCode::W145,
                    format!("Ambiguous abbreviation '{word}' for '{command}': matches {listed}."),
                    fixes,
                )
            }
        };
        // A disabled selector disables the command head through that selector.
        let span = match original.words().head_source().and_then(|head| head.word()) {
            Some(head) if code == DiagCode::W002 => {
                tcl_lexer::Span::new(head.span().start(), span.end())
            }
            _ => span,
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(code, span, message, Severity::Warning)
                .with_fixes(fixes)
                .with_subject(subject),
        );
    }

    /// Publish current whole-command advice and legacy subcommand reports.
    /// Typed original publication geometry owns whole-command shadowing.
    pub(in crate::analyser) fn flush_disabled_command_diagnostics(&mut self) {
        if self.pending_disabled_commands.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);
        let pending = std::mem::take(&mut self.pending_disabled_commands);
        for (cmd_name, ns, enforce_order, diag) in pending {
            if let Some(original) = diag.command_availability() {
                if original.matches_analysis(&self.result)
                    && !original.has_source_declaration_shadow(&self.result, enforce_order)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// Original selected lexical-context advice. The event flag is source
    /// context; this method grants no native event or procedure frame.
    pub(in crate::analyser) fn emit_w142_context_gate(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        let Some(message) = original
            .with_schema(|schema| schema.authored_source_context_gate(self.current_event.is_some()))
            .flatten()
        else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::super::RegistrySourceDiagnosticKind::ContextGate,
            None,
        ) else {
            return;
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W142,
                original.head().span(),
                message.to_owned(),
                Severity::Warning,
            )
            .with_subject(subject),
        );
    }
}
