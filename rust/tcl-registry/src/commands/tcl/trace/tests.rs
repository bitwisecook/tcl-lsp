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

use tcl_dialect::model::{Family, SurfaceQuery};

use super::*;
use crate::{
    CommandRegistry, InvocationWord, InvocationWordKind, StateTransitionFact,
    StateTransitionWidening,
};

#[test]
fn original_trace_type_roles_use_selected_native_index_extent_without_cache_authority() {
    // Implementation contract: naming.variable.trace-source-receiver-purpose
    // docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for (bytes, kind) in [
            (b"var\0\xff".as_slice(), Some(TraceKind::Variable)),
            (b"comm\0\xff".as_slice(), Some(TraceKind::Command)),
            (b"exec\0\xff".as_slice(), Some(TraceKind::Execution)),
            (b"var\xc0\x80\xff".as_slice(), None),
            (b"\0variable".as_slice(), None),
            (b"variableX".as_slice(), None),
        ] {
            let words = [
                InvocationWord::KnownBytes(bytes),
                InvocationWord::KnownBytes(b"v\0tail(k)"),
            ];
            let arguments = InvocationArguments::structured(&words).with_dialect(dialect);
            assert!(resolve_trace_type_at(arguments, 0) == kind, "{version:?}");
            let roles = trace_subject_layout_roles(arguments, ArgRole::VarRead, 2);
            assert_eq!(
                roles,
                kind.map(|kind| vec![(
                    1,
                    if kind == TraceKind::Variable {
                        ArgRole::VarRead
                    } else {
                        ArgRole::CommandName
                    }
                )])
            );
            assert!(resolve_trace_type_at(InvocationArguments::structured(&words), 0).is_none());
        }
        let expanded = [
            InvocationWord::KnownBytes(b"variable"),
            InvocationWord::Expanded,
        ];
        assert!(
            trace_subject_layout_roles(
                InvocationArguments::structured(&expanded).with_dialect(dialect),
                ArgRole::VarRead,
                2
            )
            .is_none()
        );
    }
    let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    let words = [
        InvocationWord::KnownBytes(b"variable"),
        InvocationWord::Literal("v"),
    ];
    assert!(
        trace_subject_layout_roles(
            InvocationArguments::structured(&words).with_dialect(jim),
            ArgRole::VarRead,
            2
        )
        .is_none()
    );
}

#[test]
fn registry_validates_each_modern_operation_domain_and_builds_a_list_value_fix() {
    let registry = CommandRegistry::build_default();
    for (kind, operations, invalid, allowed, replacement) in [
        (
            "variable",
            "read bogus write",
            "bogus",
            VARIABLE_TRACE_OPERATIONS,
            "read write",
        ),
        (
            "command",
            "delete read rename",
            "read",
            COMMAND_TRACE_OPERATIONS,
            "delete rename",
        ),
        (
            "execution",
            "enter rename leavestep",
            "rename",
            EXECUTION_TRACE_OPERATIONS,
            "enter leavestep",
        ),
    ] {
        let arguments = ["add", kind, "target", operations, "callback"];
        let invocation = registry
            .resolve_invocation(
                "trace",
                &arguments,
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("modern trace form resolves");
        assert_eq!(
            invocation.validate_literal_arguments(),
            Some(LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
                argument_index: 3,
                subject: operation_subject(resolve_type(kind).unwrap()),
                reason: LiteralArgumentIssueReason::InvalidMembers(vec![invalid.to_owned()]),
                allowed_values: allowed,
                replacement_value: Some(replacement.to_owned()),
            }))
        );
    }
}

#[test]
fn registry_abstains_on_dynamic_malformed_incomplete_and_invalid_type_operations() {
    let registry = CommandRegistry::build_default();
    let dynamic = [
        InvocationWord::Literal("add"),
        InvocationWord::Literal("variable"),
        InvocationWord::Literal("target"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("callback"),
    ];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(InvocationWord::Literal("trace"), &dynamic),
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .resolved()
        .expect("dynamic one-word argument keeps the invocation resolvable");
    assert_eq!(
        invocation.validate_literal_arguments(),
        Some(LiteralArgumentValidation::Abstain(
            LiteralValidationDecline::NonLiteralArgument
        ))
    );

    for (arguments, decline) in [
        (
            &["add", "variable", "target", "{unterminated", "callback"][..],
            LiteralValidationDecline::MalformedLiteral,
        ),
        (
            &["add", "variable", "target", "read"][..],
            LiteralValidationDecline::IncompleteInvocation,
        ),
        (
            &["add", "not-a-type", "target", "read", "callback"][..],
            LiteralValidationDecline::InvalidDiscriminator,
        ),
    ] {
        let invocation = registry
            .resolve_invocation(
                "trace",
                arguments,
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("literal trace head resolves even when an argument is invalid");
        assert_eq!(
            invocation.validate_literal_arguments(),
            Some(LiteralArgumentValidation::Abstain(decline))
        );
    }
}

#[test]
fn empty_and_all_invalid_modern_lists_never_offer_an_invalid_empty_replacement() {
    let registry = CommandRegistry::build_default();
    for operations in ["", "bogus nope"] {
        let arguments = ["remove", "command", "target", operations, "callback"];
        let invocation = registry
            .resolve_invocation(
                "trace",
                &arguments,
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("modern trace remove resolves");
        let Some(LiteralArgumentValidation::Invalid(issue)) =
            invocation.validate_literal_arguments()
        else {
            panic!("empty/all-invalid operation list must be invalid");
        };
        assert_eq!(issue.replacement_value, None);
    }
}

#[test]
fn legacy_validator_exists_only_on_tcl8_forms() {
    let registry = CommandRegistry::build_default();
    for dialect in [
        Some(SurfaceQuery::core(Family::Tcl, "8.4")),
        Some(SurfaceQuery::core(Family::Tcl, "8.5")),
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    ] {
        let invocation = registry
            .resolve_invocation("trace", &["variable", "target", "rwx", "callback"], dialect)
            .expect("legacy trace variable resolves in Tcl 8.x");
        let Some(LiteralArgumentValidation::Invalid(issue)) =
            invocation.validate_literal_arguments()
        else {
            panic!("invalid legacy operation character must be reported");
        };
        assert_eq!(
            issue.reason,
            LiteralArgumentIssueReason::InvalidMembers(vec!["x".to_owned()])
        );
        assert_eq!(issue.allowed_values, LEGACY_VARIABLE_TRACE_OPERATIONS);
        assert_eq!(issue.replacement_value, None);
    }
    assert!(
        registry
            .resolve_invocation(
                "trace",
                &["variable", "target", "rwx", "callback"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .and_then(|invocation| invocation.validate_literal_arguments())
            .is_none(),
        "the removed legacy subcommand must not acquire a Tcl 9 validator"
    );
}

#[test]
fn execution_callback_arity_tracks_each_literal_operation_domain() {
    let registry = CommandRegistry::build_default();
    for (operations, expected) in [
        ("enter enterstep", AppendedArity::Exactly(2)),
        ("leave leavestep", AppendedArity::Exactly(4)),
        (
            "enter leavestep",
            AppendedArity::OneOf(EXECUTION_TRACE_CALLBACK_ARITIES),
        ),
    ] {
        assert_eq!(
            registry.command_prefixes(
                "trace",
                &["add", "execution", "target", operations, "callback"]
            ),
            vec![(4, expected)],
            "unexpected execution callback contract for {operations:?}"
        );
    }
}

#[test]
fn execution_callback_arity_abstains_on_unproved_operation_lists() {
    let registry = CommandRegistry::build_default();
    for operations in ["{enter", "", "enter invalid"] {
        assert_eq!(
            registry.command_prefixes(
                "trace",
                &["add", "execution", "target", operations, "callback"]
            ),
            vec![(4, AppendedArity::Unknown)],
            "malformed/empty/invalid operation lists must abstain"
        );
    }

    let spellings = ["add", "execution", "target", "$operations", "callback"];
    let words = [
        InvocationWord::Literal("add"),
        InvocationWord::Literal("execution"),
        InvocationWord::Literal("target"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("callback"),
    ];
    assert_eq!(
        registry.command_prefixes_structured("trace", &spellings, &words),
        vec![(4, AppendedArity::Unknown)],
        "a substituted operation list is not its source spelling"
    );
}

#[test]
fn modern_variable_add_canonicalises_duplicate_operations() {
    let transitions = TRACE_ADD_TRANSITIONS.resolve(InvocationArguments::literals(&[
        "add",
        "var",
        "item",
        "write read write unset",
        "{callback one}",
    ]));

    assert_eq!(
        transitions.facts(),
        &[StateTransitionFact {
            transition: StateTransition::Trace(TraceTransition::Add {
                target: TraceTarget::Variable(TransitionSubject::LocatedLiteral {
                    value: "item".to_owned(),
                    argument_index: 2
                }),
                // The canonical order is C's `trace info` render order
                // (`array read write unset`), not the `opStrings[]` table
                // order the bad-operation error enumerates.
                operations: TraceOperationSet::Known(vec![
                    TraceOperation::Read,
                    TraceOperation::Write,
                    TraceOperation::Unset,
                ]),
                prefix: TransitionSubject::LocatedLiteral {
                    value: "{callback one}".to_owned(),
                    argument_index: 4
                },
            }),
            commit: StateTransitionCommit::OnOkOnly,
        }]
    );
}

#[test]
fn dynamic_one_word_op_list_preserves_a_typed_trace_transition() {
    let arguments = [
        InvocationWord::Literal("add"),
        InvocationWord::Literal("command"),
        InvocationWord::Literal("worker"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("prefix"),
    ];
    let transitions = TRACE_ADD_TRANSITIONS.resolve(InvocationArguments::structured(&arguments));
    assert!(matches!(
        transitions.facts(),
        [StateTransitionFact {
            transition: StateTransition::Trace(TraceTransition::Add {
                target: TraceTarget::Command(TransitionSubject::LocatedLiteral { value: target, .. }),
                operations: TraceOperationSet::Unknown(TransitionSubject::Unknown {
                    argument_index: 3,
                    word_kind: InvocationWordKind::Dynamic,
                }),
                ..
            }),
            commit: StateTransitionCommit::OnOkOnly,
        }] if target == "worker"
    ));
}

#[test]
fn add_and_remove_widen_only_their_distinct_trace_unions() {
    let dynamic_type = [
        InvocationWord::Literal("add"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("target"),
        InvocationWord::Literal("read"),
        InvocationWord::Literal("prefix"),
    ];
    let dynamic_remove_type = [
        InvocationWord::Literal("remove"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("target"),
        InvocationWord::Literal("read"),
        InvocationWord::Literal("prefix"),
    ];
    let expanded_prefix = [
        InvocationWord::Literal("add"),
        InvocationWord::Literal("variable"),
        InvocationWord::Literal("target"),
        InvocationWord::Literal("read"),
        InvocationWord::Expanded,
    ];
    for (descriptor, arguments, expected_domains, expected_subject) in [
        (
            TRACE_ADD_TRANSITIONS,
            InvocationArguments::structured(&dynamic_type),
            TRACE_ADD_TRANSITION_DOMAINS,
            TransitionSubject::Unknown {
                argument_index: 1,
                word_kind: InvocationWordKind::Dynamic,
            },
        ),
        (
            TRACE_REMOVE_TRANSITIONS,
            InvocationArguments::structured(&dynamic_remove_type),
            TRACE_REMOVE_TRANSITION_DOMAINS,
            TransitionSubject::Unknown {
                argument_index: 1,
                word_kind: InvocationWordKind::Dynamic,
            },
        ),
        (
            TRACE_ADD_TRANSITIONS,
            InvocationArguments::structured(&expanded_prefix),
            TRACE_ADD_TRANSITION_DOMAINS,
            TransitionSubject::Unknown {
                argument_index: 4,
                word_kind: InvocationWordKind::Expanded,
            },
        ),
    ] {
        let transitions = descriptor.resolve(arguments);
        assert_eq!(
            transitions.facts(),
            &[StateTransitionFact {
                transition: StateTransition::Widen(StateTransitionWidening {
                    domains: expected_domains.to_vec(),
                    subject: expected_subject,
                }),
                commit: StateTransitionCommit::OnOkOnly,
            }]
        );
    }
}

#[test]
fn dynamic_prefix_has_no_immediate_callback_and_info_fallback_is_read_only() {
    let literal = TRACE_ADD_EFFECTS.resolve(InvocationArguments::literals(&[
        "add",
        "execution",
        "worker",
        "enter",
        "prefix",
    ]));
    assert_eq!(literal.callback(), CallbackEffect::NONE);
    assert!(literal.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::ExecutionTraces
            && access.mode == EffectAccessMode::ReadWrite
    }));

    let dynamic_prefix = [
        InvocationWord::Literal("add"),
        InvocationWord::Literal("variable"),
        InvocationWord::Literal("item"),
        InvocationWord::Literal("write"),
        InvocationWord::Dynamic,
    ];
    let dynamic = TRACE_ADD_EFFECTS.resolve(InvocationArguments::structured(&dynamic_prefix));
    assert_eq!(dynamic.callback(), CallbackEffect::NONE);
    assert!(dynamic.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::CommandTraces
            && access.mode == EffectAccessMode::ReadWrite
    }));

    let dynamic_info_type = [
        InvocationWord::Literal("info"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("target"),
    ];
    let info = TRACE_INFO_EFFECTS.resolve(InvocationArguments::structured(&dynamic_info_type));
    assert_eq!(info.callback(), CallbackEffect::NONE);
    assert!(
        info.accesses()
            .iter()
            .all(|access| access.mode == EffectAccessMode::Read)
    );
}

#[test]
fn resolved_trace_add_keeps_reads_but_commits_writes_on_the_ok_edge() {
    let registry = CommandRegistry::build_default();
    let facts = registry
        .resolve_invocation(
            "trace",
            &["add", "variable", "item", "write", "prefix"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("literal trace add resolves")
        .facts();

    assert!(facts.effects.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::VariableTraces && access.mode == EffectAccessMode::Read
    }));
    assert!(facts.effects.accesses().iter().all(|access| {
        !matches!(
            access.domain,
            WorldStateDomain::VariableStore | WorldStateDomain::VariableTraces
        ) || access.mode == EffectAccessMode::Read
    }));
    assert!(matches!(
        facts
            .state_transitions
            .declared()
            .map(StateTransitions::facts),
        Some([StateTransitionFact {
            transition: StateTransition::Trace(TraceTransition::Add { .. }),
            commit: StateTransitionCommit::OnOkOnly,
        }])
    ));
}

#[test]
fn oracle_profiles_keep_legacy_forms_in_tcl8_only() {
    let registry = CommandRegistry::build_default();
    for dialect in [
        Some(SurfaceQuery::core(Family::Tcl, "8.4")),
        Some(SurfaceQuery::core(Family::Tcl, "8.5")),
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    ] {
        for (arguments, expected) in [
            (&["variable", "item", "rw", "prefix"][..], "variable"),
            (&["vdelete", "item", "rw", "prefix"][..], "vdelete"),
            (&["vinfo", "item"][..], "vinfo"),
        ] {
            let canonical = registry
                .resolve_invocation("trace", arguments, dialect)
                .and_then(|invocation| {
                    invocation
                        .facts()
                        .subcommand
                        .canonical_name()
                        .map(str::to_owned)
                });
            assert_eq!(canonical.as_deref(), Some(expected));
        }
    }
    for dialect in [
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        Some(SurfaceQuery::core(Family::Tcl, "9.1")),
    ] {
        for arguments in [
            &["variable", "item", "rw", "prefix"][..],
            &["vdelete", "item", "rw", "prefix"][..],
            &["vinfo", "item"][..],
        ] {
            let canonical = registry
                .resolve_invocation("trace", arguments, dialect)
                .and_then(|invocation| {
                    invocation
                        .facts()
                        .subcommand
                        .canonical_name()
                        .map(str::to_owned)
                });
            assert_eq!(canonical, None);
        }
    }
}

#[test]
fn legacy_forms_resolve_to_the_same_typed_variable_trace_domains() {
    let registry = CommandRegistry::build_default();
    let add = registry
        .resolve_invocation(
            "trace",
            &["variable", "item", "awrw", "prefix"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("legacy add resolves")
        .facts();
    let remove = registry
        .resolve_invocation(
            "trace",
            &["vdelete", "item", "awrw", "prefix"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("legacy remove resolves")
        .facts();

    for (facts, expected) in [(&add, true), (&remove, false)] {
        assert!(facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode == EffectAccessMode::Read
        }));
        assert!(facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableTraces
                && access.mode == EffectAccessMode::Read
        }));
        let declared = facts
            .state_transitions
            .declared()
            .expect("legacy transition is declared");
        assert!(
            matches!(
                declared.facts(),
                [StateTransitionFact {
                    transition: StateTransition::Trace(TraceTransition::Add {
                        target: TraceTarget::Variable(TransitionSubject::LocatedLiteral { value: target, .. }),
                        operations: TraceOperationSet::Known(operations),
                        ..
                    }),
                    ..
                }] if expected && target == "item" && operations.as_slice() == [
                    TraceOperation::Array,
                    TraceOperation::Read,
                    TraceOperation::Write,
                ]
            ) || matches!(
                declared.facts(),
                [StateTransitionFact {
                    transition: StateTransition::Trace(TraceTransition::Remove {
                        target: TraceTarget::Variable(TransitionSubject::LocatedLiteral { value: target, .. }),
                        operations: TraceOperationSet::Known(operations),
                        ..
                    }),
                    ..
                }] if !expected && target == "item" && operations.as_slice() == [
                    TraceOperation::Array,
                    TraceOperation::Read,
                    TraceOperation::Write,
                ]
            )
        );
    }
}

#[test]
fn trace_info_results_and_dispatch_are_registry_versioned() {
    let registry = CommandRegistry::build_default();
    let facts = registry
        .resolve_invocation(
            "trace",
            &["info", "execution", "llength"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("trace info resolves")
        .facts();

    assert_eq!(
        facts.result_stability,
        ResultStability::ReadsVersionedWorld(TRACE_INFO_RESULT_DOMAINS)
    );
    assert_eq!(facts.dispatch_dependencies, DispatchDependencies::BASE);
}

#[test]
fn removing_a_variable_trace_touches_the_placeholder_cell() {
    let registry = CommandRegistry::build_default();
    let facts = registry
        .resolve_invocation(
            "trace",
            &["remove", "variable", "item", "write", "prefix"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("trace remove resolves")
        .facts();

    assert!(facts.effects.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::VariableStore && access.mode == EffectAccessMode::Read
    }));
    assert!(matches!(
        facts
            .state_transitions
            .declared()
            .map(StateTransitions::facts),
        Some([StateTransitionFact {
            transition: StateTransition::Trace(TraceTransition::Remove {
                target: TraceTarget::Variable(_),
                ..
            }),
            commit: StateTransitionCommit::OnOkOnly,
        }])
    ));
}
