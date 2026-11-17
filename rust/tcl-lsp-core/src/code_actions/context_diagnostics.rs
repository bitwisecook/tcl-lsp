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

//! Source actions for the diagnostics a code-action request carries in its context.

use super::diagnostic_context::ContextDiagnosticData;
use super::{ActionKind, CodeAction, diagnostic_context};
use crate::definition::LspRange;
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::ContextActionTier;
use tcl_lexer::LineIndex;
use tcl_registry::events::{DataCollectionAction, EventRegistry};

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

pub(super) const HTML_ENCODE_PROC: &str =
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
        let mut bootstrap =
            collect_bootstrap_actions_current(source, analysis, registry, &commands, data);
        if data.action_tier() == ContextActionTier::SingleNotPreferred {
            bootstrap.truncate(1);
        } else {
            out.extend(taint_subject_actions(
                source, analysis, registry, &commands, data,
            ));
        }
        out.extend(bootstrap);
    }
    deduplicate_context_actions(&mut out);
    out
}

pub(super) fn deduplicate_context_actions(actions: &mut Vec<CodeAction>) {
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

pub(super) fn context_action(title: String, edits: Vec<crate::rename::TextEdit>) -> CodeAction {
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
pub(super) fn source_action_spec(
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

pub(super) fn selected_event_handlers<'a>(
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
    let Some(subject) = diagnostic.taint_subject() else {
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
