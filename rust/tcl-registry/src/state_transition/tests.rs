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

use super::*;

#[test]
fn original_namespace_export_control_uses_the_selected_cstring_extent() {
    // Implementation contract: naming.namespace.original-byte-pattern-transfers
    // docs/design/analysis/name-resolution-proofs/namespace-original-byte-pattern-transfers.md
    for version in tcl_dialect::TclVersion::ALL {
        let policy = tcl_syntax::naming::NamePolicyProtocol::authored_tcl(version);
        let patterns = [b"-clear\0suffix".as_slice(), b"p\xed\xa0\x80"];
        let (clear, retained) =
            NamespaceTransition::export_pattern_byte_operands(&patterns, policy).unwrap();
        assert!(clear);
        assert_eq!(retained, &patterns[1..]);
        let modified = [b"-clear\xc0\x80suffix".as_slice()];
        assert_eq!(
            NamespaceTransition::export_pattern_byte_operands(&modified, policy),
            Some((false, modified.as_slice()))
        );
        assert_eq!(
            NamespaceTransition::import_preload_command(policy),
            Some("auto_import")
        );
    }
    let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("jimtcl")).unwrap();
    let policy = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point).unwrap();
    assert!(NamespaceTransition::export_pattern_byte_operands(&[], policy).is_none());
    assert_eq!(NamespaceTransition::import_preload_command(policy), None);
}

#[test]
fn original_namespace_patterns_close_only_their_represented_byte_ordinals() {
    // Implementation contract: naming.namespace.original-byte-pattern-transfers
    // docs/design/analysis/name-resolution-proofs/namespace-original-byte-pattern-transfers.md
    for version in tcl_dialect::TclVersion::ALL {
        let registry = crate::model::ingress::static_context_for(version.dialect_name()).commands();
        let namespace = registry.get("namespace").unwrap();
        let dialect = crate::InvocationDialect::for_version(version);
        for operation in ["export", "import", "forget"] {
            let descriptor = namespace
                .subcommands
                .iter()
                .find(|entry| entry.name == operation)
                .and_then(|entry| entry.state_transitions)
                .unwrap();
            let words = [
                crate::InvocationWord::Literal(operation),
                crate::InvocationWord::KnownBytes(b"p\xed\xa0\x80"),
            ];
            let transitions =
                descriptor.resolve(InvocationArguments::structured(&words).with_dialect(dialect));
            assert!(
                transitions
                    .facts()
                    .iter()
                    .all(|fact| !matches!(fact.transition, StateTransition::Widen(_))),
                "{version:?}: {operation}"
            );
            let pattern = match &transitions.facts()[0].transition {
                StateTransition::Namespace(
                    NamespaceTransition::Export { patterns, .. }
                    | NamespaceTransition::Forget { patterns, .. },
                ) => &patterns[0],
                StateTransition::Namespace(NamespaceTransition::Import {
                    patterns, force, ..
                }) => {
                    assert_eq!(*force, Some(false));
                    &patterns[0]
                }
                _ => panic!("selected namespace pattern operation"),
            };
            assert_eq!(pattern.native_bytes(), Some(b"p\xed\xa0\x80".as_slice()));
            assert_eq!(pattern.argument_index(), Some(1));
            let words = [
                crate::InvocationWord::Literal(operation),
                crate::InvocationWord::Dynamic,
            ];
            let unknown =
                descriptor.resolve(InvocationArguments::structured(&words).with_dialect(dialect));
            assert!(
                unknown
                    .facts()
                    .iter()
                    .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
            );
        }
    }
}

#[test]
fn native_byte_subjects_preserve_values_and_project_each_ordinal() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let payload = [b'n', 0, 0xff];
    let words = [
        crate::InvocationWord::KnownBytes(&payload),
        crate::InvocationWord::KnownBytes(&payload),
    ];
    let arguments = InvocationArguments::structured(&words);
    let first = TransitionSubject::from_argument(arguments, 0).unwrap();
    let second = TransitionSubject::from_argument(arguments, 1).unwrap();
    assert_eq!(first.native_bytes(), Some(payload.as_slice()));
    assert_eq!(first.literal(), None);
    assert_eq!(first.argument_index(), Some(0));
    assert_ne!(first, second);
    assert!(first.with_literal_value("replacement".to_owned()).is_none());
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::CommandBinding(
        CommandBindingTransition::Move {
            from: first.clone(),
            to: second.clone(),
        },
    ));
    let projected = transitions
        .project_argument_indices(|index| index.checked_add(2))
        .unwrap();
    let StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) =
        &projected.facts()[0].transition
    else {
        panic!("move");
    };
    assert_eq!(from.native_bytes(), first.native_bytes());
    assert_eq!(to.native_bytes(), second.native_bytes());
    assert_eq!(from.argument_index(), Some(2));
    assert_eq!(to.argument_index(), Some(3));
    assert!(
        transitions
            .project_argument_indices(|index| index.checked_sub(1))
            .is_none()
    );
}

#[test]
fn original_byte_rename_transitions_select_destination_extent_and_keep_ordinals() {
    // Implementation contract: naming.command.original-byte-rename-transition
    // docs/design/analysis/name-resolution-proofs/command-original-byte-rename-transition.md
    use crate::InvocationWord;
    let from = b"P\xed\xa0\x80";
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for destination in [b"Q\xed\xa0\x81".as_slice(), b"Q\xc0\x80tail"] {
            let words = [
                InvocationWord::KnownBytes(from),
                InvocationWord::KnownBytes(destination),
            ];
            let transitions = command_binding::RENAMES_COMMANDS
                .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
            let [fact] = transitions.facts() else {
                panic!("one rename transition")
            };
            let StateTransition::CommandBinding(CommandBindingTransition::Move {
                from: original,
                to,
            }) = &fact.transition
            else {
                panic!("byte move")
            };
            assert_eq!(original.native_bytes(), Some(from.as_slice()));
            assert_eq!(original.argument_index(), Some(0));
            assert_eq!(to.native_bytes(), Some(destination));
            assert_eq!(to.argument_index(), Some(1));
            assert_eq!(
                fact.commit,
                StateTransitionCommit::MayCommitBeforeAbruptCompletion
            );
            assert!(!transitions.widens(StateTransitionDomain::CommandBindings));
        }
        let destination = b"N\xed\xa0\x81::::Q\0ignored::later";
        let words = [
            InvocationWord::KnownBytes(from),
            InvocationWord::KnownBytes(destination),
        ];
        let transitions = command_binding::RENAMES_COMMANDS
            .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
        let StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(namespace),
        }) = &transitions.facts()[0].transition
        else {
            panic!("selected qualifier")
        };
        assert_eq!(namespace.native_bytes(), Some(b"N\xed\xa0\x81".as_slice()));
        assert_eq!(namespace.argument_index(), Some(1));
        let StateTransition::CommandBinding(CommandBindingTransition::Move { to, .. }) =
            &transitions.facts()[1].transition
        else {
            panic!("move retains full input")
        };
        assert_eq!(to.native_bytes(), Some(destination.as_slice()));
        let words = [
            InvocationWord::KnownBytes(from),
            InvocationWord::KnownBytes(b"\0tail"),
        ];
        let transitions = command_binding::RENAMES_COMMANDS
            .resolve(InvocationArguments::structured(&words).with_dialect(dialect));
        let StateTransition::CommandBinding(CommandBindingTransition::Delete { name, .. }) =
            &transitions.facts()[0].transition
        else {
            panic!("empty selected CString deletes")
        };
        assert_eq!(name.native_bytes(), Some(from.as_slice()));
    }
    let words = [
        InvocationWord::KnownBytes(from),
        InvocationWord::KnownBytes(b"Q"),
    ];
    for dialect in [
        None,
        Some(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        )),
    ] {
        let arguments = InvocationArguments::structured(&words);
        let arguments = dialect.map_or(arguments, |dialect| arguments.with_dialect(dialect));
        let transitions = command_binding::RENAMES_COMMANDS.resolve(arguments);
        assert!(transitions.facts().iter().any(|fact| matches!(
            fact.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Unknown { .. })
        )));
        assert!(transitions.widens(StateTransitionDomain::CommandBindings));
    }
    assert!(
        TransitionSubject::Literal("Q".to_owned())
            .with_native_bytes_value(vec![b'Q'])
            .is_none()
    );
}

#[test]
fn native_identity_values_do_not_close_unknown_control_operands() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let subject = TransitionSubject::LocatedNativeBytes {
        value: vec![0xff],
        argument_index: 0,
    };
    let different_ordinal = TransitionSubject::LocatedNativeBytes {
        value: vec![0xff],
        argument_index: 1,
    };
    let defined = StateTransition::CommandBinding(CommandBindingTransition::Define {
        name: subject.clone(),
        kind: CommandBindingDefinitionKind::Command,
    });
    assert!(defined.represents_native_identity_subject(&subject));
    assert!(!defined.represents_native_identity_subject(&different_ordinal));
    let unresolved = StateTransition::CommandBinding(CommandBindingTransition::Unknown {
        operands: vec![subject.clone()],
    });
    assert!(!unresolved.represents_native_identity_subject(&subject));
    let interpreter = TransitionSubject::Literal(String::new());
    let limit = StateTransition::Interpreter(InterpreterTransition::SetRecursionLimit {
        interpreter,
        limit: subject.clone(),
    });
    assert!(!limit.represents_native_identity_subject(&subject));
    let package = StateTransition::Package(crate::model::binding::PackageTransition::Provide {
        package: TransitionSubject::Literal("P".to_owned()),
        version: Some(subject.clone()),
    });
    assert!(!package.represents_native_identity_subject(&subject));
    let words = [crate::InvocationWord::KnownBytes(&[0xff])];
    let arguments = InvocationArguments::structured(&words);
    let rules = [StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[0]),
        domains: &[StateTransitionDomain::CommandBindings],
    }];
    let mut unknown = StateTransitions::default();
    unknown.push(unresolved);
    unknown.widen_dynamic_arguments(arguments, &rules, true);
    assert!(unknown.widens(StateTransitionDomain::CommandBindings));
    let mut represented = StateTransitions::default();
    represented.push(defined);
    represented.widen_dynamic_arguments(arguments, &rules, true);
    assert!(!represented.widens(StateTransitionDomain::CommandBindings));
}

#[test]
fn native_byte_alias_names_use_the_selected_name_purpose() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let input = TransitionSubject::LocatedNativeBytes {
        value: b"::scope::n\xed\xa0\x80".to_vec(),
        argument_index: 3,
    };
    for version in tcl_dialect::TclVersion::ALL {
        let local = local_alias_name(
            &input,
            3,
            VariableAliasNamePurpose::Global,
            Some(crate::InvocationDialect::for_version(version)),
        )
        .unwrap();
        assert_eq!(local.native_bytes(), Some(b"n\xed\xa0\x80".as_slice()));
        assert_eq!(local.literal(), None);
        assert_eq!(local.argument_index(), Some(3));
    }
    let jim = crate::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
    );
    assert!(local_alias_name(&input, 3, VariableAliasNamePurpose::Global, Some(jim)).is_none());
    let local = local_alias_name(
        &input,
        3,
        VariableAliasNamePurpose::NamespaceVariable,
        Some(jim),
    )
    .unwrap();
    assert_eq!(local.native_bytes(), Some(b"n\xed\xa0\x80".as_slice()));
    assert_eq!(local.argument_index(), Some(3));
    assert!(matches!(
        local_alias_name(&input, 3, VariableAliasNamePurpose::Global, None),
        Some(TransitionSubject::Unknown {
            argument_index: 3,
            word_kind: InvocationWordKind::KnownBytes
        })
    ));
}

#[test]
fn known_equal_operands_retain_distinct_effective_ordinals() {
    // Implementation contract: naming.invocation.effective-transition-operands
    // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

    let arguments = InvocationArguments::literals(&["same", "same"]);
    let first = TransitionSubject::from_argument(arguments, 0).unwrap();
    let second = TransitionSubject::from_argument(arguments, 1).unwrap();
    assert_eq!(first.literal(), second.literal());
    assert_ne!(first, second);
    assert_eq!(first.argument_index(), Some(0));
    assert_eq!(second.argument_index(), Some(1));
    assert_eq!(
        TransitionSubject::Literal("same".to_owned()).argument_index(),
        None
    );
    assert_eq!(
        second
            .with_literal_value("selected".to_owned())
            .unwrap()
            .argument_index(),
        Some(1)
    );
}

#[test]
fn argv_projection_retains_nested_operands_order_and_completion_commits() {
    // Implementation contract: naming.invocation.effective-transition-operands
    // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

    let subject = |index| TransitionSubject::LocatedLiteral {
        value: "same".to_owned(),
        argument_index: index,
    };
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::CommandBinding(
        CommandBindingTransition::Alias {
            target_lookup: AliasTargetLookup::Global,
            source_interpreter: TransitionSubject::Literal(String::new()),
            alias: subject(1),
            target_interpreter: subject(2),
            target: subject(3),
            arguments: vec![
                subject(4),
                TransitionSubject::Unknown {
                    argument_index: 5,
                    word_kind: InvocationWordKind::Dynamic,
                },
            ],
        },
    ));
    transitions.push(StateTransition::VariableCellAlias(
        VariableCellAliasTransition {
            destination: VariableAliasDestination::CurrentNamespaceOrLocal,
            local: subject(4),
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(subject(1)),
                variable: subject(2),
            },
            writes_value: false,
            words: AliasWords {
                local: 4,
                target: 2,
            },
        },
    ));
    transitions.set_commit(StateTransitionCommit::MayCommitBeforeAbruptCompletion);
    let projected = transitions
        .project_argument_indices(|index| index.checked_sub(1))
        .unwrap();
    assert!(
        projected
            .facts()
            .iter()
            .all(|fact| fact.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion)
    );
    let mut indices = Vec::new();
    let mut values = Vec::new();
    for mut fact in projected.facts().iter().cloned() {
        fact.transition.for_each_subject_mut(&mut |subject| {
            indices.push(subject.argument_index());
            values.push(subject.literal().map(str::to_owned));
        });
    }
    assert_eq!(
        indices,
        [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(3),
            Some(0),
            Some(1)
        ]
    );
    assert_eq!(values[0].as_deref(), Some(""));
    assert_eq!(values[5], None);
    assert!(
        transitions
            .project_argument_indices(|index| index.checked_sub(2))
            .is_none()
    );
    assert_eq!(
        transitions.facts()[0].commit,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion
    );
}
use crate::{InvocationWord, InvocationWords};

#[test]
fn logical_alias_dynamic_target_keeps_source_relation_and_original_widening() {
    use crate::frame_effect::FrameLevelWord;
    let words = [InvocationWord::Dynamic, InvocationWord::Literal("local")];
    let args = InvocationArguments::structured(&words);
    let ordinary = alias_pairs_resolver(FrameLevelWord::Upvar)(args);
    assert!(
        ordinary
            .facts()
            .iter()
            .all(|fact| !matches!(fact.transition, StateTransition::VariableCellAlias(_)))
    );
    let source = logical_source_alias_pairs_resolver(FrameLevelWord::Upvar)(args);
    assert!(
        ordinary
            .facts()
            .iter()
            .all(|fact| source.facts().contains(fact))
    );
    assert!(source.facts().iter().any(|fact| matches!(&fact.transition,
            StateTransition::VariableCellAlias(alias)
                if alias.local.literal() == Some("local")
                && matches!(&alias.target, VariableAliasTarget::CallerSelectedFrame {
                    variable: TransitionSubject::Unknown { argument_index: 0, .. }, .. }))));
    for words in [
        [InvocationWord::Literal("target"), InvocationWord::Dynamic],
        [InvocationWord::Dynamic, InvocationWord::Dynamic],
    ] {
        let facts = logical_source_alias_pairs_resolver(FrameLevelWord::Upvar)(
            InvocationArguments::structured(&words),
        );
        assert!(
            facts
                .facts()
                .iter()
                .all(|fact| !matches!(fact.transition, StateTransition::VariableCellAlias(_)))
        );
    }
    let words = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("target"),
        InvocationWord::Literal("local"),
    ];
    let facts = logical_source_alias_pairs_resolver(FrameLevelWord::Upvar)(
        InvocationArguments::structured(&words),
    );
    assert!(
        facts
            .facts()
            .iter()
            .all(|fact| !matches!(fact.transition, StateTransition::VariableCellAlias(_)))
    );
}

#[test]
fn logical_frame_alias_projection_preserves_custom_replacement_and_commit_metadata() {
    let frame = crate::FrameEffectSpec {
        level_word: crate::frame_effect::FrameLevelWord::Upvar,
        layout: crate::FrameArgLayout::AliasPairs,
    };
    let descriptor = StateTransitionDescriptor {
        resolver: Some(alias_pairs_resolver(frame.level_word)),
        logical_source_resolver: Some(LogicalSourceStateTransitionResolver::FrameAliases(
            logical_source_alias_pairs_resolver(frame.level_word),
        )),
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        ..StateTransitionDescriptor::EMPTY
    };
    let words = [
        InvocationWord::Literal("#0"),
        InvocationWord::Literal("source"),
        InvocationWord::Literal("local"),
    ];
    let arguments = InvocationArguments::structured(&words)
        .with_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
    let owner = ResolvedStateTransitions {
        command: Some(descriptor),
        ..Default::default()
    };
    let facts = owner.resolve_logical_source_frame_aliases(arguments);
    assert!(
        matches!(&facts.facts()[0].transition, StateTransition::VariableCellAlias(alias)
            if alias.local.literal() == Some("local") && alias.words.local == 2)
    );
    assert_eq!(
        facts.facts()[0].commit,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion
    );
    let replaced = ResolvedStateTransitions {
        command: Some(descriptor),
        form: Some(StateTransitionDescriptor {
            logical_source_resolver: None,
            composition: StateTransitionComposition::Replace,
            ..StateTransitionDescriptor::EMPTY
        }),
        ..Default::default()
    };
    assert_eq!(
        replaced
            .resolve_logical_source_frame_aliases(arguments)
            .facts()
            .len(),
        0
    );
}

fn derived_alias_transition(
    local: &str,
    frame: CallerFrameSelection,
    variable: &str,
    target: usize,
) -> StateTransition {
    StateTransition::VariableCellAlias(VariableCellAliasTransition {
        destination: VariableAliasDestination::CurrentNamespaceOrLocal,
        local: TransitionSubject::LocatedLiteral {
            value: local.to_owned(),
            argument_index: target + 1,
        },
        target: VariableAliasTarget::CallerSelectedFrame {
            frame,
            variable: TransitionSubject::LocatedLiteral {
                value: variable.to_owned(),
                argument_index: target,
            },
        },
        writes_value: false,
        words: AliasWords {
            local: target + 1,
            target,
        },
    })
}

/// The alias facts a `from-frame-effect` resolver derives, and its two
/// abstentions (`docs/design/spec-dsl-examples/README.md`, "Derivations,
/// exactly"): a computed level word aborts the whole call, and a
/// computed pair member skips its pair while the next pair resolves —
/// each widening the variable-cell domains for the word it could not
/// read.
#[test]
fn derived_alias_pairs_abstain_on_computed_words_and_widen() {
    use crate::frame_effect::FrameLevelWord;
    let resolve = |level_word, words: &[InvocationWord<'_>]| {
        alias_pairs_resolver(level_word)(InvocationArguments::structured(words))
    };
    // Each pair's `otherVar` word at `target`, its local one word later.
    let widen = |argument_index| {
        StateTransition::Widen(StateTransitionWidening {
            domains: VARIABLE_ALIAS_DOMAINS.to_vec(),
            subject: TransitionSubject::Unknown {
                argument_index,
                word_kind: InvocationWordKind::Dynamic,
            },
        })
    };
    let facts = |transitions: StateTransitions| -> Vec<StateTransition> {
        transitions
            .facts()
            .iter()
            .map(|fact| fact.transition.clone())
            .collect()
    };
    let level = |word: &str| {
        CallerFrameSelection::Explicit(TransitionSubject::LocatedLiteral {
            value: word.to_owned(),
            argument_index: 0,
        })
    };

    // `upvar`'s layout: an odd word count carries the level word.
    assert_eq!(
        facts(resolve(
            FrameLevelWord::ArityParity,
            &[
                InvocationWord::Literal("1"),
                InvocationWord::Literal("a"),
                InvocationWord::Literal("b"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("d"),
                InvocationWord::Literal("e"),
                InvocationWord::Literal("f"),
            ]
        )),
        [
            derived_alias_transition("b", level("1"), "a", 1),
            widen(3),
            derived_alias_transition("f", level("1"), "e", 5)
        ]
    );
    assert_eq!(
        facts(resolve(
            FrameLevelWord::ArityParity,
            &[InvocationWord::Literal("a"), InvocationWord::Literal("b")]
        )),
        [derived_alias_transition(
            "b",
            CallerFrameSelection::DefaultCaller,
            "a",
            0
        )]
    );
    // A computed level word: no alias for the call, not the default frame.
    assert_eq!(
        facts(resolve(
            FrameLevelWord::ArityParity,
            &[
                InvocationWord::Dynamic,
                InvocationWord::Literal("a"),
                InvocationWord::Literal("b"),
            ]
        )),
        [widen(0)]
    );
    // A leading probe reads the level from the first word's text.
    assert_eq!(
        facts(resolve(
            FrameLevelWord::LeadingProbe,
            &[
                InvocationWord::Literal("#0"),
                InvocationWord::Literal("a"),
                InvocationWord::Literal("b"),
            ]
        )),
        [derived_alias_transition("b", level("#0"), "a", 1)]
    );
    // An expansion hides the pairs: nothing is stated.
    assert_eq!(
        resolve(
            FrameLevelWord::None,
            &[InvocationWord::Expanded, InvocationWord::Literal("b")]
        )
        .facts()
        .len(),
        0
    );
}

/// Counted operands remain byte values in the shared alias projection.
/// `corner-upvar-counted-nul-target.md` records the engine-specific name
/// lookup question; this test establishes no successful lookup or cell.
#[test]
fn derived_alias_pairs_preserve_counted_values_without_text_projection() {
    use crate::{InvocationWord, frame_effect::FrameLevelWord};
    let words = [
        InvocationWord::KnownBytes(b"source\0tail"),
        InvocationWord::KnownBytes(b"local\0tail"),
    ];
    let transitions =
        alias_pairs_resolver(FrameLevelWord::ArityParity)(InvocationArguments::structured(&words));
    let [fact] = transitions.facts() else {
        panic!("one exact ordinary alias pair");
    };
    let StateTransition::VariableCellAlias(alias) = &fact.transition else {
        panic!("exact operands retain the alias projection");
    };
    assert_eq!(alias.local.native_bytes(), Some(b"local\0tail".as_slice()));
    assert_eq!(alias.local.literal(), None);
    let VariableAliasTarget::CallerSelectedFrame { variable, .. } = &alias.target else {
        panic!("the descriptor retains caller selection");
    };
    assert_eq!(variable.native_bytes(), Some(b"source\0tail".as_slice()));
    assert_eq!(variable.literal(), None);
    assert_eq!(
        alias.words,
        AliasWords {
            local: 1,
            target: 0
        }
    );
}

/// The subjects an alias or command-binding fact names.
fn alias_and_binding_subjects(transition: &StateTransition) -> Vec<&TransitionSubject> {
    match transition {
        StateTransition::VariableCellAlias(alias) => {
            let mut subjects = vec![&alias.local];
            match &alias.target {
                VariableAliasTarget::Global { variable }
                | VariableAliasTarget::CurrentNamespace { variable } => subjects.push(variable),
                VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                    if let CallerFrameSelection::Explicit(level) = frame {
                        subjects.push(level);
                    }
                    subjects.push(variable);
                }
                VariableAliasTarget::Namespace {
                    namespace,
                    variable,
                } => subjects.extend([namespace, variable]),
            }
            subjects
        }
        StateTransition::CommandBinding(binding) => match binding {
            CommandBindingTransition::Define { name, .. } => vec![name],
            CommandBindingTransition::Move { from, to } => vec![from, to],
            CommandBindingTransition::Delete { interpreter, name } => {
                interpreter.iter().chain([name]).collect()
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup: _,
            } => [source_interpreter, alias, target_interpreter, target]
                .into_iter()
                .chain(arguments)
                .collect(),
            CommandBindingTransition::Unknown { operands } => operands.iter().collect(),
        },
        _ => Vec::new(),
    }
}

fn assert_alias_subject_correspondence(
    subject: &TransitionSubject,
    command: &str,
    words: &[InvocationWord<'_>],
    computed: usize,
) {
    match subject {
        TransitionSubject::Unknown { argument_index, .. } => assert_eq!(
            *argument_index, computed,
            "`{command}` {words:?}: an unknown subject stands at the computed word"
        ),
        TransitionSubject::Literal(name) => assert!(
            words
                .iter()
                .any(|word| word.literal() == Some(name.as_str())),
            "`{command}` {words:?} states `{name}`, which no literal word spells"
        ),
        TransitionSubject::LocatedLiteral {
            value,
            argument_index,
        } => {
            assert_eq!(
                words.get(*argument_index).and_then(|word| word.literal()),
                Some(value.as_str()),
                "`{command}` {words:?}: known value retains its own operand"
            );
        }
        TransitionSubject::LocatedNativeBytes {
            value,
            argument_index,
        } => {
            assert_eq!(
                words
                    .get(*argument_index)
                    .and_then(|word| word.native_bytes()),
                Some(value.as_slice()),
                "`{command}` {words:?}: native value retains its own operand"
            );
        }
    }
}

fn dynamic_alias_cases() -> &'static [(
    &'static str,
    &'static [InvocationWord<'static>],
    usize,
    StateTransitionDomain,
)] {
    use crate::InvocationWord::{Dynamic, Literal};
    use StateTransitionDomain::{CommandBindings, VariableCells};
    &[
        ("global", &[Dynamic, Literal("g")], 0, VariableCells),
        ("variable", &[Dynamic, Literal("1")], 0, VariableCells),
        (
            "namespace",
            &[Literal("upvar"), Dynamic, Literal("o"), Literal("l")],
            1,
            VariableCells,
        ),
        (
            "namespace",
            &[Literal("upvar"), Literal("::a"), Literal("o"), Dynamic],
            3,
            VariableCells,
        ),
        (
            "proc",
            &[Dynamic, Literal("a"), Literal("b")],
            0,
            CommandBindings,
        ),
        ("rename", &[Dynamic, Literal("new")], 0, CommandBindings),
        ("rename", &[Literal("old"), Dynamic], 1, CommandBindings),
        (
            "interp",
            &[
                Literal("alias"),
                Literal("i"),
                Dynamic,
                Literal("j"),
                Literal("t"),
            ],
            2,
            CommandBindings,
        ),
    ]
}

/// The witness the analyser's isolated per-item pass needs before it
/// consumes transitions (`registry-consumer-contracts.md` § *The
/// analyser*): no shipped alias or command-binding resolver states a name
/// the source does not spell. A computed word reaches a fact only as a
/// typed unknown subject at its own index — the shipped `upvar` states
/// its alias with unknown frame/name operands, a derived resolver states
/// none. Unknown identities remain in positioned alias operands or widen
/// their declared domain; a consumer never binds a name it cannot read.
#[test]
fn alias_and_binding_resolvers_abstain_on_a_dynamic_word() {
    // Implementation contract: naming.invocation.effective-transition-operands
    // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md
    use crate::InvocationWord::{Dynamic, Literal};
    use StateTransitionDomain::VariableCells;
    let context = crate::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands();
    let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
    let cases = dynamic_alias_cases();
    for &(command, words, computed, domain) in cases {
        let invocation = registry
            .resolve_structured_invocation(
                InvocationWords::structured(Literal(command), words).with_dialect(dialect),
                Some(context.context().authoring_query()),
            )
            .resolved()
            .unwrap_or_else(|| panic!("`{command}` resolves"));
        let transitions = invocation.state_transitions();
        assert!(
            transitions.widens(domain),
            "`{command}` {words:?} widens {domain:?}: {transitions:#?}"
        );
        for fact in transitions.facts() {
            for subject in alias_and_binding_subjects(&fact.transition) {
                assert_alias_subject_correspondence(subject, command, words, computed);
            }
        }
    }

    // Shipped upvar retains the unknown identity in its exact alias
    // operands. Establishing a link does not erase unrelated cells or
    // traces, unlike the widening used by derived alias resolvers.
    for (words, computed) in [
        ([Literal("1"), Dynamic, Literal("l")], 1),
        ([Literal("1"), Literal("o"), Dynamic], 2),
        ([Dynamic, Literal("o"), Literal("l")], 0),
    ] {
        let invocation = registry
            .resolve_structured_invocation(
                InvocationWords::structured(Literal("upvar"), &words).with_dialect(dialect),
                Some(context.context().authoring_query()),
            )
            .resolved()
            .expect("selected upvar metadata");
        let transitions = invocation.state_transitions();
        assert!(!transitions.widens(VariableCells));
        assert!(!transitions.widens(StateTransitionDomain::VariableTraces));
        let [fact] = transitions.facts() else {
            panic!("one positioned alias: {transitions:#?}");
        };
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            panic!("unknown identity stays in the alias: {fact:#?}");
        };
        assert_eq!(
            alias.destination,
            VariableAliasDestination::CurrentNamespaceOrLocal
        );
        assert_eq!(
            alias.words,
            AliasWords {
                local: 2,
                target: 1
            }
        );
        assert!(!alias.writes_value);
        assert_eq!(
            fact.commit,
            StateTransitionCommit::MayCommitBeforeAbruptCompletion
        );
        assert!(matches!(
            alias.target,
            VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(_),
                ..
            }
        ));
        let subjects = alias_and_binding_subjects(&fact.transition);
        assert!(
            subjects.iter().any(|subject| matches!(
                subject,
                TransitionSubject::Unknown {
                    argument_index,
                    word_kind: InvocationWordKind::Dynamic,
                } if *argument_index == computed
            )),
            "{words:?}: computed operand remains unknown"
        );
        for subject in subjects {
            assert_alias_subject_correspondence(subject, "upvar", &words, computed);
        }
    }
}

const VARIABLE_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::VariableTraces,
];

#[test]
fn dynamic_operands_become_typed_widenings() {
    const DESCRIPTOR: StateTransitionDescriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: None,
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[1]),
            domains: VARIABLE_DOMAINS,
        }],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    };
    let arguments = [InvocationWord::Literal("fixed"), InvocationWord::Dynamic];
    let transitions = DESCRIPTOR.resolve(
        InvocationWords::structured(InvocationWord::Literal("fixture"), &arguments).arguments(),
    );

    assert_eq!(
        transitions.facts(),
        &[StateTransitionFact {
            transition: StateTransition::Widen(StateTransitionWidening {
                domains: vec![
                    StateTransitionDomain::VariableCells,
                    StateTransitionDomain::VariableTraces,
                ],
                subject: TransitionSubject::Unknown {
                    argument_index: 1,
                    word_kind: InvocationWordKind::Dynamic,
                },
            }),
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        }]
    );
}

#[test]
fn command_resolution_impact_distinguishes_namespace_cells_from_lookup() {
    let widening = |domain| {
        StateTransition::Widen(StateTransitionWidening {
            domains: vec![StateTransitionDomain::Namespaces, domain],
            subject: TransitionSubject::Unknown {
                argument_index: 0,
                word_kind: InvocationWordKind::Dynamic,
            },
        })
    };

    let mut variable_cell = StateTransitions::default();
    variable_cell.push(widening(StateTransitionDomain::VariableCells));
    assert_eq!(
        variable_cell.command_resolution_impact(),
        CommandResolutionImpact::None
    );

    let mut lookup = StateTransitions::default();
    lookup.push(widening(StateTransitionDomain::CommandResolution));
    assert_eq!(
        lookup.command_resolution_impact(),
        CommandResolutionImpact::Unbounded
    );
}

fn named_definition(name: &str) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::CommandBinding(
        CommandBindingTransition::Define {
            name: TransitionSubject::Literal(name.to_owned()),
            kind: CommandBindingDefinitionKind::Command,
        },
    ));
    transitions
}

fn command_definition(_: InvocationArguments<'_>) -> StateTransitions {
    named_definition("command")
}

fn subcommand_definition(_: InvocationArguments<'_>) -> StateTransitions {
    named_definition("subcommand")
}

fn form_definition(_: InvocationArguments<'_>) -> StateTransitions {
    named_definition("form")
}

#[test]
fn on_ok_transition_is_unchanged_on_an_abrupt_edge() {
    // A `proc`-like definition becomes precise only on the OK edge. An
    // exact CFG must not treat this as an unconditional binding fact.
    let descriptor = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(command_definition),
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };
    let transitions = descriptor.resolve(InvocationArguments::literals(&[]));
    let [fact] = transitions.facts() else {
        panic!("definition resolver must emit one transition");
    };

    assert_eq!(fact.commit, StateTransitionCommit::OnOkOnly);
    assert_eq!(
        fact.commit.abrupt_transfer(),
        AbruptTransitionTransfer::Unchanged
    );
}

#[test]
fn descriptors_resolve_command_subcommand_then_form() {
    let command = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(command_definition),
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };
    let subcommand = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(subcommand_definition),
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    };
    let form = StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Replace,
        success_resolver: None,
        resolver: Some(form_definition),
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };
    let transitions = ResolvedStateTransitions {
        command: Some(command),
        subcommand: Some(subcommand),
        form: Some(form),
    }
    .resolve(InvocationArguments::literals(&[]));

    assert_eq!(transitions.facts().len(), 1);
    assert_eq!(
        transitions.facts()[0].transition,
        StateTransition::CommandBinding(CommandBindingTransition::Define {
            name: TransitionSubject::Literal("form".to_owned()),
            kind: CommandBindingDefinitionKind::Command,
        })
    );
}

#[test]
fn qualified_aliases_use_the_current_frame_tail_name() {
    assert_eq!(
        local_alias_name(
            &TransitionSubject::Literal("::pkg::counter".to_owned()),
            0,
            VariableAliasNamePurpose::Global,
            Some(crate::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6
            )),
        ),
        Some(TransitionSubject::Literal("counter".to_owned()))
    );
}

#[test]
fn unversioned_c_alias_assistance_keeps_native_release_unknown() {
    let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert!(dialect.native_name_protocol().is_none());
    let subject = TransitionSubject::Literal("ns:::v".to_owned());
    assert_eq!(
        local_alias_name(&subject, 0, VariableAliasNamePurpose::Global, Some(dialect)),
        Some(TransitionSubject::Literal("v".to_owned())),
    );
    assert!(matches!(
        local_alias_name(
            &TransitionSubject::Literal("v\0tail".to_owned()),
            0,
            VariableAliasNamePurpose::Global,
            Some(dialect),
        ),
        Some(TransitionSubject::Unknown {
            argument_index: 0,
            ..
        }),
    ));
    assert!(dialect.native_name_protocol().is_none());
}

#[test]
fn alias_name_projection_keeps_operation_and_engine_separate() {
    let rooted = TransitionSubject::Literal("::pkg:::counter".to_owned());
    let jim = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").unit_profile(),
    );
    assert_eq!(
        local_alias_name(&rooted, 2, VariableAliasNamePurpose::Global, Some(jim)),
        None
    );
    assert_eq!(
        local_alias_name(
            &rooted,
            2,
            VariableAliasNamePurpose::NamespaceVariable,
            Some(jim)
        ),
        Some(TransitionSubject::Literal("counter".to_owned()))
    );
    for version in tcl_dialect::TclVersion::ALL {
        for purpose in [
            VariableAliasNamePurpose::Global,
            VariableAliasNamePurpose::NamespaceVariable,
        ] {
            assert_eq!(
                local_alias_name(
                    &rooted,
                    2,
                    purpose,
                    Some(crate::InvocationDialect::for_version(version))
                ),
                Some(TransitionSubject::Literal("counter".to_owned()))
            );
        }
    }
    for dialect in [
        None,
        Some(crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::irules(),
        )),
    ] {
        assert_eq!(
            local_alias_name(&rooted, 2, VariableAliasNamePurpose::Global, dialect),
            Some(TransitionSubject::Unknown {
                argument_index: 2,
                word_kind: InvocationWordKind::Opaque
            })
        );
    }
}
