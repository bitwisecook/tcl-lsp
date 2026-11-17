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

//! Registry-only checks for the commands recovered after a syntax cut.
//!
//! After an unterminated delimiter the proving walk stops at the cut, and
//! the commands the recovery segmenter finds past it are an editor tier: no
//! interpretation reaches them. For a recovered command whose head is one
//! literal bareword the reached prefix does not rebind, this pass reports
//! the registry's arity, subcommand and option errors over the written
//! words; for a literal head naming a procedure the reached prefix declares
//! once and nothing else rebinds, it reports that declaration's arity.
//! Nothing else is reported there.

use tcl_core_types::DiagCode;
use tcl_lexer::TokenType;

use crate::analyser::Analyser;
use crate::analyser::types::{Diagnostic, ProcDef, Severity};
use crate::segmenter::SegmentedCommand;

impl Analyser {
    /// The first syntax cut of a document that did not parse, with the
    /// registry's count errors for each command recovered after it.
    pub(in crate::analyser) fn recovered_tail_verdicts(
        &self,
        commands: &[SegmentedCommand],
    ) -> Option<(u32, Vec<Diagnostic>)> {
        // The cut is wherever an incompleteness diagnostic sits, whether or
        // not the segmenter split commands off after it.
        let cut = self
            .result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.code,
                    DiagCode::E200 | DiagCode::E201 | DiagCode::E202 | DiagCode::E203
                )
            })
            .map(|diagnostic| diagnostic.span.start())
            .min()?;
        let registry = self.registry.clone()?;
        let verdicts = commands
            .iter()
            .filter(|command| !command.is_partial && command.span.start() > cut)
            .filter_map(|command| self.recovered_tail_verdict(&registry, command, cut))
            .collect();
        Some((cut, verdicts))
    }

    /// Publish the tail's verdicts after every other emitter. Nothing but an
    /// error code is reported past the cut: the walk's advice there rests on
    /// facts no interpretation produced.
    pub(in crate::analyser) fn publish_recovered_tail_diagnostics(&mut self) {
        let Some((cut, verdicts)) = self.recovered_tail.take() else {
            return;
        };
        self.result
            .diagnostics
            .retain(|diagnostic| diagnostic.span.start() <= cut || diagnostic.code.is_error());
        self.result.diagnostics.extend(verdicts);
    }

    /// The registry's count errors for each command of a static concatenated
    /// eval-family script. The lowering keeps no command there, so its words
    /// are judged as written, as in a recovered tail, and only where no
    /// namespace declares the head: the script's namespace (`uplevel`'s is
    /// the caller's) is not tracked.
    pub(in crate::analyser) fn publish_concatenated_script_diagnostics(&mut self) {
        let Some(registry) = self.registry.clone() else {
            return;
        };
        let config =
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(self.file_lexer_config());
        let windows = std::mem::take(&mut self.concatenated_windows);
        let mut verdicts = Vec::new();
        for (script, span) in &windows {
            let commands = crate::segmenter::segment_commands_with_offset_and_config(
                script,
                span.start(),
                config,
            );
            for command in commands.iter().filter(|command| !command.is_partial) {
                let Some(head) = command.texts.first() else {
                    continue;
                };
                let names_head =
                    |name: &String| tcl_syntax::naming::key_tail(name) == head.as_str();
                let declared_anywhere = self.result.all_procs.keys().any(names_head)
                    || self.result.all_classes.keys().any(names_head)
                    || self.result.command_aliases.keys().any(names_head);
                if declared_anywhere
                    || self.result.diagnostics.iter().any(|diagnostic| {
                        diagnostic.code.is_error()
                            && command.span.start() <= diagnostic.span.start()
                            && diagnostic.span.end() <= command.span.end()
                    })
                {
                    continue;
                }
                verdicts.extend(self.recovered_tail_verdict(&registry, command, span.start()));
            }
        }
        self.result.diagnostics.extend(verdicts);
    }

    /// Whether the reached prefix of the document may have rebound `head`:
    /// a procedure, alias, class, rename or deletion under that name, an
    /// import that could bring one in, or a provider the walk could not see.
    fn recovered_tail_head_is_rebound(&self, head: &str) -> bool {
        let rooted = crate::naming::normalise_qualified_name(head);
        self.result.has_dynamic_providers
            || !self.result.namespace_imports.is_empty()
            || self.result.all_procs.contains_key("::unknown")
            || self.result.all_procs.contains_key(&rooted)
            || self.result.all_classes.contains_key(&rooted)
            || self.result.command_aliases.contains_key(&rooted)
            || self.result.destroyed_commands.contains_key(&rooted)
            || self.deleted_commands.contains_key(&rooted)
            || self
                .result
                .renamed_commands
                .iter()
                .any(|(new, old)| *new == rooted || *old == rooted)
    }

    /// The procedure a recovered head names when the reached prefix declares
    /// it exactly once, before the cut, with a literal parameter list, and
    /// nothing could have rebound or shadowed it since. Under a seeded
    /// source namespace a head may resolve elsewhere, so nothing is claimed.
    fn recovered_tail_procedure(&self, head: &str, cut: u32) -> Option<&ProcDef> {
        if self
            .source_analysis_entry
            .as_ref()
            .is_some_and(|entry| entry.source_point_entry.is_some())
        {
            return None;
        }
        let rooted = crate::naming::normalise_qualified_name(head);
        let procedure = self.result.all_procs.get(&rooted)?;
        let mut declarations = self
            .result
            .proc_declaration_sites
            .iter()
            .filter(|(name, _)| *name == rooted);
        let (_, declared_at) = declarations.next()?;
        let rebound = self.result.has_dynamic_providers
            || !self.result.namespace_imports.is_empty()
            || self.result.all_classes.contains_key(&rooted)
            || self.result.command_aliases.contains_key(&rooted)
            || self.result.destroyed_commands.contains_key(&rooted)
            || self.deleted_commands.contains_key(&rooted)
            || self
                .result
                .renamed_commands
                .iter()
                .any(|(new, old)| *new == rooted || *old == rooted);
        (declarations.next().is_none()
            && declared_at.start() < cut
            && !procedure.params_computed
            && !rebound)
            .then_some(procedure)
    }

    fn recovered_tail_verdict(
        &self,
        registry: &tcl_registry::CommandRegistry,
        command: &SegmentedCommand,
        cut: u32,
    ) -> Option<Diagnostic> {
        let literal = |index: usize| -> Option<&str> {
            let token = command.argv.get(index)?;
            let text = command.texts.get(index)?;
            let single = command.single_token_word.get(index).copied()?;
            let braced = token.kind == TokenType::Str && !token.in_quote;
            (single
                && (braced
                    || token.kind == TokenType::Esc && !crate::naming::is_dynamic_word(text)))
            .then_some(text.as_str())
        };
        let head_token = command.argv.first()?;
        let head = literal(0).filter(|_| head_token.kind == TokenType::Esc)?;
        if let Some(procedure) = self.recovered_tail_procedure(head, cut) {
            let arguments = (1..command.texts.len())
                .map(literal)
                .collect::<Option<Vec<&str>>>()?;
            return super::validity::arity_verdict(
                head,
                procedure.arity(),
                arguments.len(),
                false,
                command.span,
                None,
                None,
            );
        }
        if tcl_syntax::naming::is_qualified(head.as_bytes())
            || self.recovered_tail_head_is_rebound(head)
        {
            return None;
        }
        let arguments = (1..command.texts.len())
            .map(literal)
            .collect::<Option<Vec<&str>>>()?;
        let resolved = registry.resolve_call(head, &arguments, registry.own_surface_query())?;
        if resolved.sub.is_none() && !resolved.spec.subcommands.is_empty() {
            if !arguments.is_empty() {
                return Self::recovered_tail_selector_verdict(registry, head, &arguments, command);
            }
            return (resolved.arity().min > 0).then(|| {
                Diagnostic::new(
                    DiagCode::E001,
                    head_token.span,
                    format!("'{head}' requires a subcommand"),
                    Severity::Error,
                )
            });
        }
        if let Some(verdict) =
            Self::recovered_tail_selector_verdict(registry, head, &arguments, command)
        {
            return Some(verdict);
        }
        let words = tcl_registry::InvocationArguments::literals(&arguments);
        let count = resolved.argument_count_for_arity(words)?;
        let display_name = resolved
            .sub
            .map_or_else(|| head.to_owned(), |sub| format!("{head} {}", sub.name));
        super::validity::arity_verdict(
            &display_name,
            resolved.arity_for_arguments(words),
            usize::from(count),
            false,
            command.span,
            None,
            None,
        )
    }

    /// The registry's selector errors over a recovered command's literal
    /// words: an unknown, ambiguous or disabled subcommand, then an option
    /// the dialect excludes or an ambiguous option prefix. No fix is offered
    /// in the tail.
    fn recovered_tail_selector_verdict(
        registry: &tcl_registry::CommandRegistry,
        head: &str,
        arguments: &[&str],
        command: &SegmentedCommand,
    ) -> Option<Diagnostic> {
        use tcl_registry::{AuthoredSourceOptionBoundary, AuthoredSourceSubcommandDiagnostic};

        let invocation =
            registry.resolve_invocation(head, arguments, registry.own_surface_query())?;
        let word_span = |argument: usize| {
            command
                .argv
                .get(argument + 1)
                .map_or(command.span, |token| token.span)
        };
        let warning = |code, argument, message| {
            Some(Diagnostic::new(
                code,
                word_span(argument),
                message,
                Severity::Warning,
            ))
        };
        if let Some(outcome) = invocation.authored_source_subcommand_diagnostic() {
            let word = arguments.first()?;
            return match outcome {
                AuthoredSourceSubcommandDiagnostic::Disabled { canonical, surface } => warning(
                    DiagCode::W002,
                    0,
                    format!(
                        "'{head} {canonical}' is disabled in the active dialect profile{}",
                        super::validity::dialect_availability_suffix(surface)
                    ),
                ),
                AuthoredSourceSubcommandDiagnostic::Unknown { .. } => warning(
                    DiagCode::W001,
                    0,
                    format!("Unknown subcommand '{word}' for '{head}'"),
                ),
                AuthoredSourceSubcommandDiagnostic::Ambiguous { candidates } => warning(
                    DiagCode::W145,
                    0,
                    format!(
                        "Ambiguous abbreviation '{word}' for '{head}': matches {}.",
                        quoted_list(&candidates)
                    ),
                ),
            };
        }
        let scan = invocation.authored_source_diagnostic_options()?;
        let label = std::iter::once(head)
            .chain(scan.subcommands.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(option) = scan.options.iter().find(|option| !option.available) {
            let spelling = arguments.get(option.argument)?;
            return warning(
                DiagCode::W004,
                option.argument,
                format!("Option '{spelling}' on '{label}' is not available in the active dialect."),
            );
        }
        if let AuthoredSourceOptionBoundary::Ambiguous {
            argument,
            candidates,
        } = scan.boundary
        {
            let spelling = arguments.get(argument)?;
            return warning(
                DiagCode::W145,
                argument,
                format!(
                    "Ambiguous option '{spelling}' for '{label}': matches {}.",
                    quoted_list(&candidates)
                ),
            );
        }
        None
    }
}

fn quoted_list(candidates: &[&str]) -> String {
    candidates
        .iter()
        .map(|candidate| format!("'{candidate}'"))
        .collect::<Vec<_>>()
        .join(", ")
}
