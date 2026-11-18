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

//! Option words: a missing `--` terminator (W304), options that consume every `unset` word (W217), an option the dialect excludes (W004) and an ambiguous abbreviation (W145).

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;

impl Analyser {
    /// **W304.** Original source advice before the first data-or-option
    /// boundary in a selected vocabulary that admits `--`. Unknown values
    /// remain conditional; a preceding textual assignment supplies no current
    /// cell value or severity promotion.
    pub(in crate::analyser) fn emit_w304_missing_option_terminator(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
        command: &str,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(scan) = original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        if !scan.accepts_terminator {
            return;
        }
        let (argument, dynamic) = match scan.boundary {
            tcl_registry::AuthoredSourceOptionBoundary::Dynamic(index) => (index, true),
            tcl_registry::AuthoredSourceOptionBoundary::Unknown(index) => (index, false),
            _ => return,
        };
        if original.is_captured(argument) {
            return;
        }
        let Some(word) = original.word(argument) else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::super::RegistrySourceDiagnosticKind::OptionTerminator,
            Some(argument),
        ) else {
            return;
        };
        let source_extent = word.span();
        let suffix = scan.subcommands.join(" ");
        let label = if suffix.is_empty() {
            command.to_owned()
        } else {
            format!("{command} {suffix}")
        };
        let (severity, message) = if dynamic {
            (
                Severity::Suggestion,
                format!(
                    "'{label}' parses leading '-' as options. Insert '--' before substituted input if it is intended as data."
                ),
            )
        } else {
            (
                Severity::Warning,
                format!(
                    "'{label}' argument starts with '-'. Add '--' before this value if it is intended as data."
                ),
            )
        };
        let fixes = original
            .written_index(argument)
            .and_then(|_| word.try_text().ok())
            .map_or_default(|spelling| {
                vec![super::super::types::CodeFix {
                    span: source_extent,
                    new_text: format!("-- {spelling}"),
                    description: "Insert '--' option terminator".to_owned(),
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                }]
            });
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W304,
                source_extent,
                message,
                severity,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }

    /// W217: the selected dialect option protocol consumed every unset word.
    /// A captured option can explain the source shape, but cannot acquire a
    /// written operand or an insertion that reclassifies the captured prefix.
    pub(in crate::analyser) fn emit_w217_unset_option_only(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_unset_option_only_arguments,
        };
        let Some(original) = original else { return };
        let Some(arguments) = original
            .with_schema(source_unset_option_only_arguments)
            .flatten()
        else {
            return;
        };
        let Some(last) = arguments.end.checked_sub(1) else {
            return;
        };
        let subject = original
            .subject_range(Kind::OptionOnly, arguments.start..=last)
            .or_else(|| original.subject(Kind::OptionOnly, None));
        let Some(subject) = subject else { return };
        let span = match &subject {
            crate::analyser::DiagnosticSubject::RegistrySource(subject) => subject.span(),
            _ => return,
        };
        let fixes = original
            .word(arguments.start)
            .filter(|_| original.written_index(arguments.start) == Some(0))
            .and_then(|word| Some((word.span(), word.try_text().ok()?)))
            .map_or_default(|(span, spelling)| {
                vec![super::super::types::CodeFix {
                    span,
                    new_text: format!("-- {spelling}"),
                    description: "Insert '--' so the following words are variable names".to_owned(),
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                }]
            });
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W217, span,
                "`unset` unsets no variable here — `-nocomplain` / `--` are consumed as options. To unset a variable whose name begins with `-`, put `--` before it (e.g. `unset -- -nocomplain`).".to_owned(),
                Severity::Warning,
            ).with_fixes(fixes).with_subject(subject),
        );
    }

    /// Clear compatibility buffers; source-owned W304 advice is emitted at
    /// its retained original invocation without whole-file value inference.
    pub(in crate::analyser) fn flush_w304_diagnostics(&mut self) {
        self.pending_w304.clear();
    }

    /// **W004.** Explain excluded exact option rows from the same retained
    /// source descriptor, vocabulary and value-width owner as lifecycle advice.
    /// Values and reserved operands never become new option candidates.
    pub(in crate::analyser) fn emit_w004_dialect_invalid_option(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
        command: &str,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(scan) = original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        let subcommand = (!scan.subcommands.is_empty()).then(|| scan.subcommands.join(" "));
        for selected in scan
            .options
            .into_iter()
            .filter(|selected| !selected.available && !original.is_captured(selected.argument))
        {
            let Some(word) = original.word(selected.argument) else {
                continue;
            };
            let Some(spelling) = original.literal(selected.argument) else {
                continue;
            };
            let Some(subject) = original.subject(
                super::super::super::RegistrySourceDiagnosticKind::DisabledOption,
                Some(selected.argument),
            ) else {
                continue;
            };
            let suffix = subcommand
                .as_deref()
                .map_or(String::new(), |sub| format!(" {sub}"));
            let fixes = Self::original_option_removal_fix(original, &selected, spelling);
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                DiagCode::W004, word.span(),
                format!("Option '{spelling}' on '{command}'{suffix} is not available in the active dialect ({}).", self.dialect()),
                Severity::Warning,
            ).with_fixes(fixes).with_subject(subject));
        }
        if let tcl_registry::AuthoredSourceOptionBoundary::Ambiguous {
            argument,
            candidates,
        } = scan.boundary
        {
            let Some(spelling) = original.literal(argument) else {
                return;
            };
            let Some(subject) = original.subject(
                super::super::super::RegistrySourceDiagnosticKind::DisabledOption,
                Some(argument),
            ) else {
                return;
            };
            let before = self.result.diagnostics.len();
            self.emit_w145_ambiguous_option(
                command,
                subcommand.as_deref(),
                spelling,
                &candidates
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect::<Vec<_>>(),
                original,
                argument,
            );
            if let Some(diagnostic) = self.result.diagnostics.get_mut(before) {
                diagnostic.subject = Some(subject);
            }
        }
    }

    fn original_option_removal_fix(
        original: &super::super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        selected: &tcl_registry::AuthoredSourceOption<'_>,
        spelling: &str,
    ) -> Vec<super::super::types::CodeFix> {
        let Some(values) = &selected.values else {
            return Vec::new();
        };
        let Some(first) = original.word(selected.argument) else {
            return Vec::new();
        };
        let last = if values.is_empty() {
            Some(first)
        } else {
            original.word(values.end - 1)
        };
        let Some(last) = last else {
            return Vec::new();
        };
        let Some(written) = original.written_index(selected.argument) else {
            return Vec::new();
        };
        if !(selected.argument..values.end).all(|argument| {
            original.written_index(argument) == Some(written + argument - selected.argument)
        }) {
            return Vec::new();
        }
        let end = original
            .word(values.end)
            .filter(|next| {
                next.span().start() >= last.span().end()
                    && original.written_index(values.end)
                        == Some(written + values.end - selected.argument)
            })
            .map_or(last.span().end(), |next| next.span().start());
        vec![super::super::types::CodeFix {
            span: tcl_lexer::Span::new(first.span().start(), end),
            new_text: String::new(),
            description: format!("Remove '{spelling}' option"),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        }]
    }

    /// **W145** at an option word. Same contract as the subcommand form:
    /// name the matching candidates and offer one manual-pick fix each.
    fn emit_w145_ambiguous_option(
        &mut self,
        cmd_name: &str,
        sub_name: Option<&str>,
        word: &str,
        candidates: &[String],
        original: &super::super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        argument: usize,
    ) {
        let Some(operand) = original.word(argument) else {
            return;
        };
        let span = operand.span();
        let sub_suffix = sub_name.map_or(String::new(), |n| format!(" {n}"));
        let listed = candidates
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let fixes: Vec<super::super::types::CodeFix> = candidates
            .iter()
            .filter_map(|candidate| {
                original.written_index(argument)?;
                let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                    candidate,
                    operand.config(),
                )?;
                Some(super::super::types::CodeFix {
                    span,
                    new_text,
                    description: format!("Expand to '{candidate}'"),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                })
            })
            .collect();
        self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W145,
    span,
    format!(
                "Ambiguous abbreviation '{word}' for '{cmd_name}{sub_suffix}': matches {listed}."
            ),
    Severity::Warning,
).with_fixes(fixes));
    }
}
