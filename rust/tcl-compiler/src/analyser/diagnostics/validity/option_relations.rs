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

//! Relations between a call's options and positionals (E-R14) and the literal-argument validation built on them (W146).

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;
use tcl_registry::lifecycle::Lifecycle;

/// One leading option word the call supplied, as the relation checker needs
/// it: the canonical name, the flag word's span, and the option's first value
/// word when that word is statically known.
#[derive(Debug, Clone)]
pub(in crate::analyser) struct SeenOption {
    pub(in crate::analyser) name: &'static str,
    pub(in crate::analyser) span: tcl_lexer::Span,
    pub(in crate::analyser) value: Option<String>,
}

/// One positional word after the leading option run: its literal text when
/// statically known, and its span.
#[derive(Debug, Clone)]
pub(in crate::analyser) struct SeenPositional {
    pub(in crate::analyser) value: Option<String>,
    pub(in crate::analyser) span: tcl_lexer::Span,
}

/// Selected source options and positionals retained together for relationship
/// verdicts, including the independent completeness obligation.
#[derive(Debug, Default)]
pub(in crate::analyser) struct ScannedInvocation {
    pub(in crate::analyser) options: Vec<SeenOption>,
    pub(in crate::analyser) positionals: Vec<SeenPositional>,
    /// Whether the call was read to its end with every word statically known.
    /// **Only** this licenses proving a relation term *absent*.
    pub(in crate::analyser) complete: bool,
}

/// The span a relation violation points at: the run from the first to the last
/// word the violation names, or `fallback` when it names none the call
/// actually supplied (a `requires-one-of` that found nothing).
fn relation_span(
    present: &[tcl_registry::OptionTerm],
    seen_options: &[SeenOption],
    positionals: &[SeenPositional],
    fallback: tcl_lexer::Span,
) -> tcl_lexer::Span {
    let span_of = |term: tcl_registry::OptionTerm| -> Option<tcl_lexer::Span> {
        match term {
            tcl_registry::OptionTerm::Option(name)
            | tcl_registry::OptionTerm::OptionValue(name, _) => seen_options
                .iter()
                .find(|option| option.name == name)
                .map(|option| option.span),
            tcl_registry::OptionTerm::Argument(index)
            | tcl_registry::OptionTerm::ArgumentValue(index, _) => {
                positionals.get(usize::from(index)).map(|word| word.span)
            }
        }
    };
    let spans: Vec<tcl_lexer::Span> = present.iter().copied().filter_map(span_of).collect();
    match (
        spans.iter().map(|span| span.start()).min(),
        spans.iter().map(|span| span.end()).max(),
    ) {
        (Some(start), Some(end)) => tcl_lexer::Span::new(start, end),
        _ => fallback,
    }
}

/// **The whole E-R14 consumer**, shared by the ordinary command path and the
/// object-instance dispatch path so one invocation is judged by one rule.
///
/// Every relation is evaluated natively; the `constraints` hook is reached
/// only when a spec declares one *and* nothing declarative had anything to
/// report — so a registry with no `constraints` hook (which is every shipped
/// command) never enters the VM at any call site.
///
/// Each entry pairs the relation's [`Lifecycle`] with its diagnostic, so the
/// caller can buffer a version-gated relation for the post-walk floor and push
/// an ungated one inline.
pub(in crate::analyser) fn option_relation_diagnostics(
    display_name: &str,
    relations: &[&'static tcl_registry::OptionRelation],
    constraints_hook: Option<tcl_registry::ConstraintsHook>,
    call: &ScannedInvocation,
    fallback_span: tcl_lexer::Span,
) -> Vec<(Lifecycle, crate::analyser::types::Diagnostic)> {
    let seen_options = &call.options;
    let positionals = &call.positionals;
    if relations.is_empty() && constraints_hook.is_none() {
        tcl_registry::spec::note_relation_check(!seen_options.is_empty(), false, 0, false);
        return Vec::new();
    }
    let options: Vec<(&'static str, Option<&str>)> = seen_options
        .iter()
        .map(|option| (option.name, option.value.as_deref()))
        .collect();
    let positional_values: Vec<Option<&str>> = positionals
        .iter()
        .map(|word| word.value.as_deref())
        .collect();
    let facts = tcl_registry::OptionFacts {
        options: &options,
        positionals: &positional_values,
        complete: call.complete,
    };

    let mut out = Vec::new();
    for relation in relations {
        let tcl_registry::RelationVerdict::Violated(violation) = relation.evaluate(&facts) else {
            continue;
        };
        let span = relation_span(&violation.present, seen_options, positionals, fallback_span);
        let code = if violation.kind.is_exclusion() {
            DiagCode::W147
        } else {
            DiagCode::W152
        };
        out.push((
            relation.lifecycle,
            crate::analyser::types::Diagnostic::new(
                code,
                span,
                violation.message_for(display_name),
                Severity::Warning,
            ),
        ));
    }

    // The escape hatch, and only here: a spec that declares no `constraints`
    // hook never reaches the hook seam at all, and one that does is asked only
    // when the declarative relations found nothing to report.
    if !out.is_empty() {
        tcl_registry::spec::note_relation_check(
            !seen_options.is_empty(),
            true,
            relations.len(),
            false,
        );
        return out;
    }
    let Some(hook) = constraints_hook else {
        tcl_registry::spec::note_relation_check(
            !seen_options.is_empty(),
            true,
            relations.len(),
            false,
        );
        return out;
    };
    tcl_registry::spec::note_relation_check(!seen_options.is_empty(), true, relations.len(), true);
    for report in hook(&facts) {
        let span = match report.slot {
            tcl_registry::ConstraintSlot::Option(name) => seen_options
                .iter()
                .find(|option| option.name == name)
                .map_or(fallback_span, |option| option.span),
            tcl_registry::ConstraintSlot::Argument(index) => positionals
                .get(usize::from(index))
                .map_or(fallback_span, |word| word.span),
            tcl_registry::ConstraintSlot::Command => fallback_span,
        };
        let code = if report.conflict {
            DiagCode::W147
        } else {
            DiagCode::W152
        };
        out.push((
            Lifecycle::UNSPECIFIED,
            crate::analyser::types::Diagnostic::new(code, span, report.message, Severity::Warning),
        ));
    }
    out
}

impl Analyser {
    /// **W146.** Run the resolved invocation's registry-owned relationship or
    /// literal-content validator. The analyser knows only how to project
    /// source words, anchor the returned argument index, and render the typed
    /// issue; every legal set and cross-argument relationship stays in the
    /// command registry.
    pub(in crate::analyser) fn emit_w146_literal_argument_validation(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(tcl_registry::LiteralArgumentValidation::Invalid(issue)) = original
            .with_schema(crate::analyser::diagnostic_registry::source_literal_validation)
            .flatten()
        else {
            return;
        };
        let Some(operand) = original.word(issue.argument_index) else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::super::RegistrySourceDiagnosticKind::LiteralArgument,
            Some(issue.argument_index),
        ) else {
            return;
        };
        let span = operand.span();
        let allowed = issue.allowed_values.join(", ");
        let (message, invalid_description) = match &issue.reason {
            tcl_registry::LiteralArgumentIssueReason::Empty => (
                format!(
                    "Empty {}; expected one or more of: {allowed}",
                    issue.subject
                ),
                String::new(),
            ),
            tcl_registry::LiteralArgumentIssueReason::InvalidMembers(invalid) => {
                let quoted = invalid
                    .iter()
                    .map(|member| format!("'{member}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                (
                    format!(
                        "Invalid member(s) {quoted} in {}; expected one or more of: {allowed}",
                        issue.subject
                    ),
                    quoted,
                )
            }
        };
        let fixes = issue
            .replacement_value
            .and_then(|replacement| {
                original.written_index(issue.argument_index)?;
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &replacement,
                    operand.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    format!("Remove invalid member(s) {invalid_description}"),
                )])
            })
            .unwrap_or_default();
        let diagnostic = crate::analyser::types::Diagnostic::new(
            DiagCode::W146,
            span,
            message,
            Severity::Warning,
        )
        .with_fixes(fixes);
        self.result
            .diagnostics
            .push(diagnostic.with_subject(subject));
    }

    pub(in crate::analyser) fn emit_proven_source_option_relationships(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        self.emit_source_option_relationships(original, display_name);
    }

    pub(super) fn emit_source_option_relationships(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        use crate::analyser::diagnostic_registry::source_option_relationships;
        let Some(facts) = original.with_schema(source_option_relationships).flatten() else {
            return;
        };
        if facts.relations.is_empty() && facts.constraints.is_none() {
            return;
        }
        let fallback = original.head().span();
        let value = |argument: usize| {
            original
                .words()
                .arguments()
                .get(argument)
                .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
        };
        let call = ScannedInvocation {
            options: facts
                .scan
                .options
                .iter()
                .filter(|option| option.available)
                .map(|option| SeenOption {
                    name: option.option.name,
                    span: original
                        .word(option.argument)
                        .map_or(fallback, tcl_lexer::NativeWord::span),
                    value: option
                        .values
                        .as_ref()
                        .and_then(|values| value(values.start)),
                })
                .collect(),
            positionals: facts
                .positionals
                .iter()
                .map(|&argument| SeenPositional {
                    value: value(argument),
                    span: original
                        .word(argument)
                        .map_or(fallback, tcl_lexer::NativeWord::span),
                })
                .collect(),
            complete: facts.complete,
        };
        let axis = original
            .with_schema(crate::analyser::diagnostic_registry::source_descriptors)
            .and_then(|selected| self.lifecycle_axis(selected.command));
        for (lifecycle, diagnostic) in option_relation_diagnostics(
            display_name,
            &facts.relations,
            facts.constraints,
            &call,
            fallback,
        ) {
            let Some(subject) = original.subject_extent(
                crate::analyser::RegistrySourceDiagnosticKind::ArgumentRelationship,
                diagnostic.span,
            ) else {
                continue;
            };
            let diagnostic = diagnostic.with_subject(subject);
            if lifecycle.is_unspecified() {
                self.pending_arity.push((
                    original.command().to_owned(),
                    String::new(),
                    false,
                    diagnostic,
                ));
            } else {
                self.pending_option_conflicts.push(
                    super::super::version_gate::GatedOptionConflict {
                        axis,
                        lifecycle,
                        resolution_name: original.command().to_owned(),
                        namespace: String::new(),
                        enforce_order: false,
                        diagnostic,
                    },
                );
            }
        }
    }
}
