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

//! Capturing an invocation's transition: its original words, registry words and subjects.

use super::advice_graph::root_context;
use super::{
    AdviceNamingPolicy, OriginalLogicalSourceNameInput, OriginalSourceCommandTransition,
    SourceAdviceNameInput, SourceAdviceWord,
};
use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::{CommandBindingTransition, StateTransition, TransitionSubject};

pub(super) fn original_words(
    native: &[NativeWord],
    policy: &AdviceNamingPolicy,
) -> Option<Vec<SourceAdviceWord>> {
    if matches!(policy, AdviceNamingPolicy::Logical(_)) {
        return Some(
            native
                .iter()
                .enumerate()
                .map(|(ordinal, original)| {
                    let value =
                        tcl_syntax::word_rules::original_static_word_unicode_value(original)
                            .filter(|bytes| {
                                std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0)
                            })
                            .map(Arc::<[u8]>::from);
                    SourceAdviceWord {
                        original: original.clone(),
                        input: value.as_ref().map(|bytes| {
                            SourceAdviceNameInput::Logical(OriginalLogicalSourceNameInput {
                                original: original.clone(),
                                bytes: Arc::clone(bytes),
                            })
                        }),
                        value,
                        origin: crate::registry_invocation::InvocationWordOrigin::Written(ordinal),
                    }
                })
                .collect(),
        );
    }
    let policy = policy.native()?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        native,
        policy.string_protocol(),
    )
    .ok()?;
    Some(
        native
            .iter()
            .enumerate()
            .map(|(ordinal, original)| SourceAdviceWord {
                original: original.clone(),
                input: SignatureSourceNameKey::from_original_native_word(
                    original,
                    tcl_syntax::word_rules::WordValueRules::from_config(&original.config()),
                    policy,
                )
                .map(SignatureSourceNameInput::OriginalWord)
                .map(SourceAdviceNameInput::Native),
                value: captured.literal(ordinal).map(Arc::from),
                origin: crate::registry_invocation::InvocationWordOrigin::Written(ordinal),
            })
            .collect(),
    )
}

pub(super) fn registry_words(
    arguments: &[SourceAdviceWord],
) -> Vec<tcl_registry::InvocationWord<'_>> {
    arguments
        .iter()
        .map(|argument| {
            if argument.original.group().expand {
                return tcl_registry::InvocationWord::Expanded;
            }
            argument
                .value
                .as_deref()
                .map_or(tcl_registry::InvocationWord::Dynamic, |bytes| {
                    crate::registry_invocation::source_structure::source_schema_word(
                        tcl_registry::InvocationWord::KnownBytes(bytes),
                    )
                })
        })
        .collect()
}

fn transition_subjects(transitions: &tcl_registry::StateTransitions) -> Vec<TransitionSubject> {
    let mut subjects = Vec::new();
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::CommandBinding(CommandBindingTransition::Define { name, .. }) => {
                subjects.push(name.clone());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) => {
                subjects.extend([from.clone(), to.clone()]);
            }
            StateTransition::CommandBinding(CommandBindingTransition::Delete {
                interpreter,
                name,
            }) => {
                subjects.extend(interpreter.iter().cloned());
                subjects.push(name.clone());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                ..
            }) => {
                subjects.extend([
                    source_interpreter.clone(),
                    alias.clone(),
                    target_interpreter.clone(),
                    target.clone(),
                ]);
                subjects.extend(arguments.iter().cloned());
            }
            StateTransition::Interpreter(
                tcl_registry::InterpreterTransition::Create {
                    interpreter: Some(subject),
                    ..
                }
                | tcl_registry::InterpreterTransition::Delete {
                    interpreter: subject,
                },
            )
            | StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
            }) => subjects.push(subject.clone()),
            StateTransition::Namespace(transition) => {
                use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget};
                let namespace = match transition {
                    NamespaceTransition::Export {
                        namespace,
                        patterns,
                    }
                    | NamespaceTransition::Import {
                        namespace,
                        patterns,
                        ..
                    }
                    | NamespaceTransition::Forget {
                        namespace,
                        patterns,
                    } => {
                        subjects.extend(patterns.iter().cloned());
                        namespace
                    }
                    NamespaceTransition::SetPath { namespace, path } => {
                        subjects.push(path.clone());
                        namespace
                    }
                    NamespaceTransition::Ensemble {
                        namespace,
                        arguments,
                        ..
                    } => {
                        subjects.extend(arguments.iter().cloned());
                        namespace
                    }
                    NamespaceTransition::Delete { namespace } => namespace,
                    _ => continue,
                };
                if let NamespaceTransitionTarget::Named(subject) = namespace {
                    subjects.push(subject.clone());
                }
            }
            _ => {}
        }
    }
    subjects
}

/// A qualifier selected by the Registry retains the complete original Move
/// operand. It never becomes a fabricated original namespace word.
pub(super) fn original_transition_subject_matches(
    subject: &TransitionSubject,
    input: &SourceAdviceNameInput,
    transitions: &tcl_registry::StateTransitions,
) -> bool {
    let Some(selected) = subject
        .native_bytes()
        .or_else(|| subject.literal().map(str::as_bytes))
    else {
        return false;
    };
    if selected == input.bytes() {
        return true;
    }
    let is_qualifier = transitions.facts().iter().any(|fact| {
        matches!(&fact.transition,
        StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
            namespace: tcl_registry::NamespaceTransitionTarget::Named(owned),
        }) if owned == subject)
    });
    if !is_qualifier {
        return false;
    }
    transitions.command_bindings().any(|transition| {
        let CommandBindingTransition::Move { to, .. } = transition else {
            return false;
        };
        if to.argument_index() != subject.argument_index()
            || to
                .native_bytes()
                .or_else(|| to.literal().map(str::as_bytes))
                != Some(input.bytes())
        {
            return false;
        }
        let destination = match input {
            SourceAdviceNameInput::Native(original) => original
                .policy()
                .recipe()
                .rename_destination_input(root_context(original.policy()), original.bytes())
                .ok()
                .map(|projection| projection.selected().to_vec()),
            SourceAdviceNameInput::Logical(original) => Some(original.bytes().to_vec()),
        };
        destination.is_some_and(|bytes| {
            tcl_registry::state_transition::namespace_qualifiers_bytes(&bytes) == selected
        })
    })
}

pub(super) fn capture_jim_alias_prefix(
    receipt: &Arc<OriginalSourceCommandTransition>,
    arguments: &[TransitionSubject],
) -> Option<Vec<SourceAdviceWord>> {
    trace_jim_alias_option(
        &receipt.descriptor,
        "jim.alias.prefix",
        arguments
            .iter()
            .enumerate()
            .map(|(ordinal, subject)| {
                let input = trace_jim_alias_option(
                    &receipt.descriptor,
                    "jim.alias.prefix_input",
                    receipt.input(subject),
                )?
                .clone();
                let original = trace_jim_alias_option(
                    &receipt.descriptor,
                    "jim.alias.prefix_word",
                    input.original_word(),
                )?
                .clone();
                Some(SourceAdviceWord {
                    original,
                    value: Some(Arc::from(input.bytes())),
                    input: Some(input),
                    origin: crate::registry_invocation::InvocationWordOrigin::BindingPrefix(
                        ordinal,
                    ),
                })
            })
            .collect::<Option<Vec<_>>>(),
    )
}

// WORK-only, same admission and nonwrapping process quota as the Jim owner trace.
pub(super) fn trace_jim_alias_option<T>(
    command: &str,
    stage: &'static str,
    value: Option<T>,
) -> Option<T> {
    // value-transfer-ok: irreducible — selects which calls a debug trace logs.
    // registry-axis-ok: irreducible — selects which calls a debug trace logs; until never
    if command == "alias" {
        crate::registry_invocation::source_structure::trace_jim_source_option(stage, value)
    } else {
        value
    }
}

pub(super) fn trace_jim_alias_guard(command: &str, stage: &'static str, value: bool) -> bool {
    // value-transfer-ok: irreducible — selects which calls a debug trace logs.
    // registry-axis-ok: irreducible — selects which calls a debug trace logs; until never
    if command == "alias" {
        let _ = crate::registry_invocation::source_structure::trace_jim_source_option(
            stage,
            value.then_some(()),
        );
    }
    value
}

pub(super) fn capture_transition(
    origin: &Arc<super::super::SourceOriginId>,
    native: &[NativeWord],
    command: &str,
    arguments: &[SourceAdviceWord],
    transitions: tcl_registry::StateTransitions,
    selection_lineage: &[Arc<OriginalSourceCommandTransition>],
) -> Option<Arc<OriginalSourceCommandTransition>> {
    let inputs = transition_subjects(&transitions)
        .into_iter()
        .map(|subject| {
            let Some(ordinal) = subject.argument_index() else {
                // Only the schema-owned current-interpreter constant is
                // supported without an operand producer. Its provenance stays
                // in the retained transition recipe, never in source inputs.
                return matches!(&subject, TransitionSubject::Literal(value) if value.is_empty())
                    .then_some(None);
            };
            let argument = trace_jim_alias_option(
                command,
                "jim.alias.capture.argument",
                arguments.get(ordinal),
            )?;
            let input = trace_jim_alias_option(
                command,
                "jim.alias.capture.input",
                argument.input.as_ref(),
            )?
            .clone();
            trace_jim_alias_option(
                command,
                "jim.alias.capture.subject",
                original_transition_subject_matches(&subject, &input, &transitions)
                    .then_some(Some((subject, input))),
            )
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    Some(Arc::new(OriginalSourceCommandTransition {
        site: super::super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.first()?.span().start(),
        },
        original: Arc::from(native),
        inputs,
        descriptor: command.to_owned(),
        transitions,
        selection_lineage: selection_lineage.to_vec(),
    }))
}
