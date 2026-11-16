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

//! Commands that run but should not be called: deprecated commands (IRULE2002) and Tcl's private namespaces (W143).

use crate::analyser::state::Analyser;
use crate::analyser::types::{CodeFix, Diagnostic, Severity};
use tcl_core_types::DiagCode;

/// Deferred original literal-head source policy. It retains the real word and
/// complete input; it never asserts private implementation execution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::analyser) struct PendingPrivateTclSourceAdvice {
    pub(super) subject: std::sync::Arc<crate::analyser::SourcePrivateTclNamespaceSubject>,
    pub(super) diagnostic: Diagnostic,
}

impl Analyser {
    /// IRULE2002 source deprecation advice from the selected original descriptor.
    /// Metadata can propose a drop-in spelling; source ownership alone makes
    /// the edit a review proposal, without entered replacement-handler proof.
    pub(in crate::analyser) fn emit_irule2002_deprecated_command(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_descriptors,
        };
        let Some(original) = original else {
            return;
        };
        if original
            .context()
            .context()
            .authoring_query()
            .core
            .nearest()
            .is_none_or(|(family, _)| family != tcl_dialect::model::Family::F5Irules)
        {
            return;
        }
        let Some(descriptors) = original.with_schema(source_descriptors) else {
            return;
        };
        let Some(replacement) = descriptors.command.deprecated_replacement else {
            return;
        };
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::DeprecatedCommand, None)
        else {
            return;
        };
        let span = original.head().span();
        let fixes = descriptors
            .command
            .deprecated_replacement_drop_in
            .then(|| self.deprecated_command_source_proposal(original, replacement))
            .flatten()
            .map(|new_text| CodeFix {
                span,
                new_text,
                description: format!("Replace with '{replacement}'"),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            })
            .into_iter()
            .collect();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::Irule2002,
                span,
                format!(
                    "'{}' is deprecated in iRules. Use '{replacement}' instead.",
                    original.command()
                ),
                Severity::Warning,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }

    fn deprecated_command_source_proposal(
        &self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        replacement: &str,
    ) -> Option<String> {
        if !original.matches_analysis(&self.result, &self.source) {
            return None;
        }
        let head = original.head();
        let config = head.config();
        let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(replacement, config)?;
        let mut proposed = replacement.clone();
        for ordinal in 0..original.words().arguments().len() {
            if original.written_index(ordinal) != Some(ordinal) {
                return None;
            }
            let word = original.word(ordinal)?;
            if word.image() != head.image() || word.config() != config || word.group().expand {
                return None;
            }
            let text =
                std::str::from_utf8(word.image().bytes().get(word.span().as_range())?).ok()?;
            proposed.push(' ');
            proposed.push_str(text);
        }
        let image = tcl_lexer::SourceImage::document(&proposed);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(proposed.len()).ok()?),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        original.words().with_stock_source_command_proposal(
            original.context(),
            &command.words,
            |_| (),
        )?;
        Some(replacement)
    }

    /// Whether the written head itself spells a private namespace. A bare
    /// `exists` written inside `::tcl::dict` reaches `::tcl::dict::exists` by
    /// ordinary lookup; it names no private namespace and has no public rewrite.
    fn spells_private_tcl_namespace(
        input: &crate::analyser::ResolvedAnalysisInput,
        head: &str,
    ) -> bool {
        let context = input.context_registry();
        tcl_registry::private_tcl_namespaces::classify_private_tcl_namespace_call(
            head,
            context.commands(),
            Some(context.context().authoring_query()),
        )
        .is_some()
    }

    /// W143 is selected Registry source advice where a genuine schema exists.
    /// Otherwise the same Registry-owned namespace policy reads the original
    /// static whole head, retaining explicitly unavailable lookup/handler
    /// applicability. Unregistered helpers cannot acquire a fake descriptor.
    /// Bare direct written heads alone retain the existing review-only public
    /// spelling edit; aliases/captures and delimited heads keep advice only.
    pub(in crate::analyser) fn emit_w143_private_tcl_namespace(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        _scope_path: &[usize],
    ) {
        let Some(input) = self.result.resolved_input.clone() else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(&self.source);
        let Some((head, site)) = self
            .head_identities
            .original_written_head_at_span_in_source(&image, cmd_tok.span, input.lexer_config())
        else {
            return;
        };
        if head.group().expand {
            return;
        }
        let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(&head) else {
            return;
        };
        let Some(literal_head) = std::str::from_utf8(&value)
            .ok()
            .filter(|head| Self::spells_private_tcl_namespace(&input, head))
        else {
            return;
        };
        let Some((segment, _)) = self
            .head_identities
            .invocation_at_source("", site.offset)
            .original_recorded_command()
        else {
            return;
        };
        let original = self.original_diagnostic_source_for_segment(&segment);
        let selected = original.as_ref().and_then(|source| source.registry());
        // An existing selected descriptor owns registered spelling/aliases.
        // Without one this remains the separate literal-head namespace policy,
        // required for unregistered Whole-namespace implementation helpers.
        let classified_name = selected
            .map_or(literal_head, |selected| selected.command())
            .to_owned();
        let context = input.context_registry();
        let Some(call) = tcl_registry::private_tcl_namespaces::classify_private_tcl_namespace_call(
            &classified_name,
            context.commands(),
            Some(context.context().authoring_query()),
        ) else {
            return;
        };
        let message = match &call.suggestion {
            Some(suggestion) => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; use the public ensemble command instead — e.g. '{suggestion}'."
            ),
            None => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; '{}' is not a subcommand of the public '{}' ensemble, so there is no supported spelling of this call.",
                call.tail, call.public_command
            ),
        };
        let direct_written = selected.is_none_or(|selected| {
            selected.head() == &head
                && (0..selected.words().arguments().len())
                    .all(|argument| selected.written_index(argument) == Some(argument))
        });
        let fixes = call
            .suggestion
            .as_ref()
            .and_then(|suggestion| {
                (direct_written
                    && !head.group().expand
                    && head.group().kind == tcl_lexer::WordKind::Bare)
                    .then_some(())?;
                let public = tcl_syntax::word_rules::lexical_unicode_source_word(
                    call.public_command,
                    head.config(),
                )?;
                let tail =
                    tcl_syntax::word_rules::lexical_unicode_source_word(&call.tail, head.config())?;
                Some(CodeFix {
                    span: head.span(),
                    new_text: format!("{public} {tail}"),
                    description: format!("Replace with '{suggestion}'"),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                })
            })
            .into_iter()
            .collect();
        let subject = std::sync::Arc::new(
            crate::analyser::SourcePrivateTclNamespaceSubject::from_original_source(
                head,
                input,
                site.offset,
                selected.map(|selected| selected.words().clone()),
                classified_name,
                call,
            ),
        );
        let diagnostic = Diagnostic::new(DiagCode::W143, cmd_tok.span, message, Severity::Warning)
            .with_fixes(fixes)
            .with_subject(crate::analyser::DiagnosticSubject::PrivateTclNamespace(
                subject.clone(),
            ));
        self.pending_w143.push(PendingPrivateTclSourceAdvice {
            subject,
            diagnostic,
        });
    }

    /// Whole-file source ownership suppressions are deliberately order-independent
    /// dependency advice. Every request offset reissues a genuine current
    /// selected Require operand; reporting package/name maps are not authority.
    /// Source calls/classes similarly use their authentic declaration receipts.
    /// Drains the original pending source-policy receipts once.
    pub(in crate::analyser) fn flush_w143_diagnostics(&mut self) {
        if self.pending_w143.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_w143);
        for candidate in pending {
            if !candidate.subject.matches_source(&self.source, &self.result) {
                continue;
            }
            let Some(input) = self.result.resolved_input.as_ref() else {
                continue;
            };
            let image = candidate.subject.original_head().image();
            let offset = candidate.subject.source_offset();
            // Genuine source declarations are positive suppression advice;
            // report QName/alias maps do not manufacture a call target.
            let declared = if input.has_logical_source_name_context() {
                crate::registry_invocation::source_structure::source_logical_procedure_call_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            } else {
                crate::registry_invocation::source_structure::source_original_procedure_call_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            };
            // No registered command owns an unregistered private-namespace
            // name, so an occupied slot there is the document's own command.
            let own_command = candidate.subject.selected_source_words().is_none()
                && self.head_identities.diagnostic_slot_presence_at(offset)
                    == crate::command_binding::SourceCommandSlotPresence::Present;
            if declared
                || own_command
                || crate::registry_invocation::source_structure::source_class_instance_words_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            {
                continue;
            }
            let context = input.context_registry();
            let package_covers = self.result.package_requires.iter().any(|required| {
                let Some((_, offset)) = self
                    .head_identities
                    .original_written_word_at_span_in_source(
                        image,
                        required.range,
                        input.lexer_config(),
                    )
                else {
                    return false;
                };
                let Some(words) =
                    crate::registry_invocation::source_structure::source_registry_words_at(
                        &self.source,
                        &self.result,
                        offset,
                    )
                else {
                    return false;
                };
                let Some(reference) = words.package_reference(&context) else {
                    return false;
                };
                if reference.kind()
                    != tcl_registry::source_navigation::SourcePackageReferenceKind::Require
                {
                    return false;
                }
                let Some(word) = words.original_argument_word(reference.argument()) else {
                    return false;
                };
                let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                else {
                    return false;
                };
                let Ok(package) = std::str::from_utf8(&value) else {
                    return false;
                };
                tcl_registry::private_tcl_namespaces::required_package_namespace_advice_covers(
                    package,
                    candidate.subject.classified_source_name(),
                )
            });
            if !package_covers {
                self.result.diagnostics.push(candidate.diagnostic);
            }
        }
    }

    /// Typed Registry source shape advice for the selected deprecated command.
    /// Complete original word spellings preserve braces, quotes, substitutions,
    /// escapes and newlines; captured or expanded argument layouts get no edit.
    pub(in crate::analyser) fn emit_source_deprecation_advice(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_descriptors,
        };
        let Some(original) = original else {
            return;
        };
        let Some(descriptors) = original.with_schema(source_descriptors) else {
            return;
        };
        let Some(advice) = descriptors.command.source_deprecation_advice else {
            return;
        };
        if !advice.applies_to(original.context().context().authoring_query()) {
            return;
        }
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::DeprecatedCommand, None)
        else {
            return;
        };
        let span = original.head().span();
        let count = original.words().arguments().len();
        let raw = (0..count)
            .map(|index| {
                (original.written_index(index) == Some(index)).then_some(())?;
                original.word(index)?.try_text().ok()
            })
            .collect::<Option<Vec<_>>>();
        let fixes = raw
            .as_deref()
            .and_then(|arguments| advice.replacement(arguments))
            .and_then(|new_text| {
                let last = original.word(count.checked_sub(1)?)?;
                Some(vec![CodeFix {
                    span: tcl_lexer::Span::new(span.start(), last.span().end()),
                    new_text,
                    description: advice.description().to_owned(),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }])
            })
            .unwrap_or_default();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                advice.code(),
                span,
                advice.message().to_owned(),
                Severity::Warning,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }
}
