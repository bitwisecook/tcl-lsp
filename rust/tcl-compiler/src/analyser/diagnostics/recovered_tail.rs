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
//! the registry's arity and missing-subcommand errors over the written
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
            return (arguments.is_empty() && resolved.arity().min > 0).then(|| {
                Diagnostic::new(
                    DiagCode::E001,
                    head_token.span,
                    format!("'{head}' requires a subcommand"),
                    Severity::Error,
                )
            });
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
}
