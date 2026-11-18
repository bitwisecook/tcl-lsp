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

use super::{
    NamespaceTransition, NamespaceTransitionTarget, StateTransition, TransitionSubject,
    WHICH_OPTIONS, fold_qualifiers, fold_tail, namespace_delete_state_transitions,
    namespace_path_state_transitions, namespace_which_arg_roles, spec,
};
use crate::prelude::SideEffectTarget;
use crate::{
    CallbackEffect, CallbackKinds, InvocationArguments, StaticEffectFootprint,
    WorldEffectComposition, WorldEffectDescriptor, WorldEffectDynamicFallback,
    WorldEffectWriteSource, WorldStateDomain,
};
use tcl_dialect::model::surface_admits;
use tcl_dialect::model::{Family, SurfaceQuery};

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn source_namespace_eval_covers_only_its_declared_legacy_bridge() {
    const POLICY_WRITE: &[StaticEffectAccess] = &[StaticEffectAccess::new(
        WorldStateDomain::InterpreterPolicy,
        EffectAccessMode::Write,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    )];
    use crate::world_effect::{
        EffectAccessMode, StaticEffectAccess, StaticInterpreterScope, StaticNamespaceScope,
        StaticSubjectScope,
    };
    let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    let registry = crate::CommandRegistry::build_default();
    let arguments = ["eval", "n", "proc format {args} {return NS}"];
    let selected = registry.resolve_structured_invocation(
        crate::InvocationWords::literals("namespace", &arguments).with_dialect(dialect),
        None,
    );
    let schema = selected.resolved().unwrap();
    let (raw, _) = schema.authored_source_effects_and_state_transitions(dialect);
    assert!(raw.accesses().iter().any(|access| access.domain
        == WorldStateDomain::InterpreterPolicy
        && access.mode == EffectAccessMode::Write));
    let (residual, transitions) = schema
        .authored_source_delegated_script_effects(dialect)
        .unwrap();
    assert!(residual.accesses().iter().all(|access| access.domain
        != WorldStateDomain::InterpreterPolicy
        || access.mode == EffectAccessMode::Read));
    assert_eq!(residual.callback().kinds, CallbackKinds::SCRIPT);
    assert!(
        matches!(transitions.facts(), [fact] if matches!(&fact.transition, StateTransition::Namespace(NamespaceTransition::Ensure { .. })))
    );
    // Native/common facts continue to retain the original policy bridge.
    assert!(
        schema
            .facts()
            .effects
            .accesses()
            .iter()
            .any(
                |access| access.domain == WorldStateDomain::InterpreterPolicy
                    && access.mode == EffectAccessMode::Write
            )
    );
    for malformed in [&["eval", "n"][..], &["eval", "n", "body", "extra"][..]] {
        let selected = registry.resolve_structured_invocation(
            crate::InvocationWords::literals("namespace", malformed).with_dialect(dialect),
            None,
        );
        if let Some(schema) = selected.resolved() {
            assert!(
                schema
                    .authored_source_delegated_script_effects(dialect)
                    .is_none()
            );
        }
    }
    let dynamic = [
        crate::InvocationWord::Literal("eval"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("body"),
    ];
    let selected = registry.resolve_structured_invocation(
        crate::InvocationWords::structured(crate::InvocationWord::Literal("namespace"), &dynamic)
            .with_dialect(dialect),
        None,
    );
    if let Some(schema) = selected.resolved() {
        assert!(
            schema
                .authored_source_delegated_script_effects(dialect)
                .is_none()
        );
    }
    let mut altered = crate::CommandRegistry::build_default();
    altered.insert(crate::CommandSpec {
        world_effects: crate::knowledge::Knowledge::Partial(WorldEffectDescriptor {
            composition: WorldEffectComposition::Extend,
            static_footprint: StaticEffectFootprint {
                accesses: POLICY_WRITE,
                callback: CallbackEffect::NONE,
            },
            resolver: None,
            source_resolver: None,
            dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
        }),
        ..spec()
    });
    let selected = altered.resolve_structured_invocation(
        crate::InvocationWords::literals("namespace", &arguments).with_dialect(dialect),
        None,
    );
    let schema = selected.resolved().unwrap();
    assert!(
        schema
            .authored_source_delegated_script_effects(dialect)
            .is_none()
    );
}

#[test]
fn which_roles_use_the_selected_option_order_and_reserved_final_name() {
    let options = crate::resolved_invocation::InvocationOptions {
        availability: crate::resolved_invocation::InvocationAvailability::default(),
        parent_surface: None,
        form_surface: None,
        prefix_matching: crate::abbrev::PrefixMatching::Enabled,
        placement: crate::spec::OptionPlacement::Leading,
        positional_prefix_words: 0,
        reserved_trailing_words: 1,
        case_list: None,
        base: WHICH_OPTIONS,
        form: &[],
    };
    assert_eq!(
        namespace_which_arg_roles(
            crate::InvocationArguments::literals(&["-variable", "-command", "-dash"]),
            options,
        ),
        Some(vec![(2, crate::arg_role::ArgRole::CommandNameProbe)])
    );
    assert_eq!(
        namespace_which_arg_roles(
            crate::InvocationArguments::literals(&["-command", "-variable", "-dash"]),
            options,
        ),
        Some(vec![(2, crate::arg_role::ArgRole::VarRead)])
    );
    let expanded = [
        crate::InvocationWord::Literal("-variable"),
        crate::InvocationWord::Expanded,
        crate::InvocationWord::Literal("name"),
    ];
    assert!(
        namespace_which_arg_roles(crate::InvocationArguments::structured(&expanded), options,)
            .is_none()
    );
}

#[test]
fn jim_scripted_ensemble_metadata_requires_exact_words_and_separate_helper_binding() {
    // naming.namespace.jim-original-scripted-ensemble-publication-with-rooted-result
    // docs/design/analysis/name-resolution-proofs/namespace-jim-original-scripted-ensemble-publication-with-rooted-result.md
    // naming.namespace.jim-original-scripted-helper-availability-and-forwarding
    // docs/design/analysis/name-resolution-proofs/namespace-jim-original-scripted-helper-availability-and-forwarding.md
    // The original returned helper body rejects unsupported operations.
    // These are source-schema assertions, not current helper/Normal proofs.
    let spec = super::jim_spec();
    let ensemble = spec.subcommand("ensemble").expect("Jim helper schema");
    let query = Some(SurfaceQuery::core(Family::Jim, "0.84"));
    assert!(ensemble.surface.is_none());
    let context = crate::model::ingress::static_context_for("jim");
    let current = context
        .context()
        .resolve_spec(context.commands(), "namespace")
        .expect("Jim namespace source card");
    assert!(
        context
            .context()
            .available_subcommands(current)
            .iter()
            .any(|sub| sub.name == "ensemble")
    );
    assert_eq!(
        ensemble
            .available_sub_subcommands(query, None)
            .iter()
            .map(|sub| sub.name)
            .collect::<Vec<_>>(),
        ["create"]
    );
    for invalid in ["cre", "configure", "exists"] {
        assert!(
            ensemble
                .resolve_sub_subcommand_for_dialect(invalid, query)
                .is_none()
        );
    }
    assert_eq!(
        ensemble
            .resolve_sub_subcommand_for_dialect("create", query)
            .map(|sub| sub.name),
        Some("create")
    );
    assert_eq!(
        ensemble
            .options
            .iter()
            .map(|option| option.name)
            .collect::<Vec<_>>(),
        ["-automap"]
    );
    for (word, expected) in [
        ("-automap", Some("-automap")),
        ("-auto", None),
        ("-map", None),
    ] {
        assert_eq!(
            ensemble
                .resolve_option_word(word, query, None, None)
                .unique(),
            expected
        );
    }
    let dialect = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").unit_profile(),
    );
    let operands = [
        crate::InvocationWord::Literal("ensemble"),
        crate::InvocationWord::Literal("create"),
    ];
    let words =
        crate::InvocationWords::structured(crate::InvocationWord::Literal("namespace"), &operands)
            .with_dialect(dialect);
    let facts = context
        .commands()
        .resolve_structured_invocation(words, dialect.authoring_query())
        .resolved()
        .unwrap()
        .facts();
    assert!(facts.native_compilation.is_none());
    assert!(!ensemble.prefix_matching.accepts_prefixes());
    assert!(!ensemble.arg_values_accept_prefix);
    assert!(ensemble.native_compilation.is_none());
    assert!(ensemble.successful_handler.is_none());
    assert!(ensemble.analyser_hook.is_none());
    assert!(ensemble.state_transitions.stated().is_none());
    assert!(ensemble.world_effects.is_unknown());
}

#[test]
fn namespace_basic_members_use_versioned_private_invocation_protocol() {
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        NativeCompilationSelection, NativeCompilationWordShape,
    };
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        for member in ["import", "export", "path"] {
            let native = super::spec()
                .subcommands
                .iter()
                .find(|sub| sub.name == member)
                .unwrap()
                .native_compilation
                .unwrap();
            let arguments = [member, "::A"];
            let words = crate::InvocationWords::literals("namespace", &arguments);
            let shapes = [NativeCompilationWordShape::Literal; 2];
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            };
            if version >= tcl_dialect::TclVersion::V8_6 {
                assert_eq!(
                    native.select(words, &shapes, Some(dialect), context),
                    NativeCompilationSelection::Unknown
                );
            }
            let selection = native.select_with_named_operand_view(
                words,
                &shapes,
                Some(dialect),
                context,
                crate::native_compilation::NamedCompilerOperandView::OriginalArguments {
                    selectors_retained: 1,
                },
            );
            if version < tcl_dialect::TclVersion::V8_6 {
                assert!(native.implementation_lookup(dialect).is_none());
                assert_eq!(selection, NativeCompilationSelection::Generic);
            } else {
                assert!(matches!(selection,
                        NativeCompilationSelection::NamedInvocation {lookup,arguments_from: 1,..}
                            if lookup.member == member && lookup.ensemble == "::namespace"));
            }
        }
    }
}

#[test]
fn namespace_code_declares_deferred_execution_timing() {
    let registry = crate::CommandRegistry::build_default();

    assert_eq!(
        registry.script_timing("namespace", &["code", "rename set saved_set"], 1, None,),
        Some(crate::ScriptTiming::Deferred)
    );
}

#[test]
fn namespace_path_keeps_its_tcl_list_operand_whole() {
    let transitions = namespace_path_state_transitions(InvocationArguments::literals(&[
        "path",
        "::pkg {::other child}",
    ]));

    assert!(matches!(
        transitions.facts(),
        [fact]
            if matches!(
                &fact.transition,
                StateTransition::Namespace(NamespaceTransition::SetPath {
                    namespace: NamespaceTransitionTarget::Current,
                    path: TransitionSubject::LocatedLiteral { value: path, .. },
                }) if path == "::pkg {::other child}"
            )
    ));
}

#[test]
fn original_namespace_delete_roles_require_cardinality_without_requiring_text_values() {
    // naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    use crate::{ArgRole, InvocationWord, InvocationWords};
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let registry = crate::model::ingress::static_context_for(version.dialect_name()).commands();
        let exact = [
            InvocationWord::Literal("delete"),
            InvocationWord::KnownBytes(b"N\xed\xa0\x80"),
            InvocationWord::Dynamic,
        ];
        let selected = registry.resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("namespace"), &exact)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = selected.resolved().unwrap().facts();
        assert!(facts.arg_roles_complete, "{version:?}");
        assert_eq!(facts.argument_offset, 1);
        assert_eq!(
            facts.arg_roles,
            [(0, ArgRole::NamespaceName), (1, ArgRole::NamespaceName)]
        );
        let expanded = [InvocationWord::Literal("delete"), InvocationWord::Expanded];
        let selected = registry.resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("namespace"), &expanded)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = selected.resolved().unwrap().facts();
        assert!(!facts.arg_roles_complete, "{version:?}");
        assert_eq!(facts.arg_roles.len(), 0);
    }
}

#[test]
fn original_namespace_delete_effects_hand_only_legacy_state_to_the_recursive_transition() {
    // naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let registry = crate::model::ingress::static_context_for(version.dialect_name()).commands();
        let arguments = ["delete", "N"];
        let selected = registry.resolve_structured_invocation(
            crate::InvocationWords::literals("namespace", &arguments).with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = selected.resolved().unwrap().facts();
        assert!(facts.effects.accesses().is_empty(), "{version:?}");
        assert_eq!(facts.effects.callback().kinds, CallbackKinds::TRACE);
        assert_eq!(facts.effects.legacy().side_effects.len(), 2);
        assert!(
            facts
                .transition_effect_coverage
                .entries()
                .iter()
                .any(|coverage| coverage.covers(
                    WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
                    WorldStateDomain::InterpreterPolicy,
                ))
        );
        assert!(matches!(
            facts.state_transitions.declared().unwrap().facts(),
            [fact] if matches!(&fact.transition, StateTransition::Namespace(NamespaceTransition::Delete { .. }))
        ));
        let no_targets = ["delete"];
        let selected = registry.resolve_structured_invocation(
            crate::InvocationWords::literals("namespace", &no_targets).with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = selected.resolved().unwrap().facts();
        assert_eq!(facts.transition_effect_coverage.entries().len(), 0);
        assert!(facts.effects.accesses().iter().any(|access| access.domain
            == WorldStateDomain::InterpreterPolicy
            && access.mode == crate::world_effect::EffectAccessMode::Write));
    }
}

#[test]
fn original_namespace_delete_keeps_independent_policy_writes_and_trace_callbacks() {
    // naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    use crate::world_effect::{
        EffectAccessMode, StaticEffectAccess, StaticInterpreterScope, StaticNamespaceScope,
        StaticSubjectScope,
    };
    const POLICY_WRITE: &[StaticEffectAccess] = &[StaticEffectAccess::new(
        WorldStateDomain::InterpreterPolicy,
        EffectAccessMode::Write,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    )];
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        world_effects: crate::knowledge::Knowledge::Partial(WorldEffectDescriptor {
            composition: WorldEffectComposition::Extend,
            static_footprint: StaticEffectFootprint {
                accesses: POLICY_WRITE,
                callback: CallbackEffect::NONE,
            },
            resolver: None,
            source_resolver: None,
            dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
        }),
        ..spec()
    });
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let arguments = ["delete", "N"];
        let selected = registry.resolve_structured_invocation(
            crate::InvocationWords::literals("namespace", &arguments).with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = selected.resolved().unwrap().facts();
        assert!(facts.effects.accesses().iter().any(|access| access.domain
            == WorldStateDomain::InterpreterPolicy
            && access.mode == EffectAccessMode::Write));
        assert_eq!(facts.effects.callback().kinds, CallbackKinds::TRACE);
    }
}

/// tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0: a literal create installs one
/// command, at `-command` (the last one wins) or the current namespace's
/// name; a rejected value creates nothing; 8.5 ignores a trailing option
/// and rejects `-parameters`, which 8.6+ accept.
#[test]
fn only_a_literal_valid_ensemble_create_names_its_command() {
    use crate::state_transition::EnsembleCommandName;
    use tcl_dialect::TclVersion;
    let created = |version: Option<TclVersion>, words: &[&str]| {
        let arguments = InvocationArguments::literals(words);
        let arguments = match version {
            Some(version) => arguments.with_dialect(crate::InvocationDialect::for_version(version)),
            None => arguments.with_dialect(crate::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::plain_tcl(),
            )),
        };
        super::literal_ensemble_creation(arguments)
    };
    let v86 = Some(TclVersion::V8_6);
    assert_eq!(
        created(v86, &["ensemble", "create", "-map", "foo ::e::Foo"]),
        Some(EnsembleCommandName::CurrentNamespace)
    );
    assert_eq!(
        created(
            v86,
            &["ensemble", "create", "-command", "m8", "-command", "m9"]
        ),
        Some(EnsembleCommandName::Named(
            TransitionSubject::LocatedLiteral {
                value: "m9".to_owned(),
                argument_index: 5,
            }
        ))
    );
    assert!(
        created(
            v86,
            &[
                "ensemble",
                "create",
                "-command",
                "x",
                "-prefixes",
                "2",
                "-parameters",
                "p"
            ]
        )
        .is_some()
    );
    for rejected in [
        &["ensemble", "create", "-map"][..],
        &["ensemble", "create", "-com", "m"],
        &["ensemble", "create", "-namespace", "::m"],
        &["ensemble", "create", "-map", "a"],
        &["ensemble", "create", "-map", "a {}"],
        &["ensemble", "create", "-prefixes", "maybe"],
        &["ensemble", "create", "-subcommands", "{"],
        &["ensemble", "cr", "-command", "m"],
        &["ensemble", "configure", "::e", "-map", ""],
    ] {
        assert_eq!(created(v86, rejected), None, "{rejected:?}");
    }
    let parameters = ["ensemble", "create", "-parameters", "x"];
    assert_eq!(created(Some(TclVersion::V8_5), &parameters), None);
    assert!(created(Some(TclVersion::V8_5), &["ensemble", "create"]).is_some());
    assert_eq!(
        created(Some(TclVersion::V8_4), &["ensemble", "create"]),
        None
    );
    assert_eq!(created(None, &["ensemble", "create"]), None);
}

/// tclsh 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0: a relative `-map` head is
/// qualified in the ensemble's namespace, the last duplicate key wins, a
/// `-map` key overrides its `-subcommands` entry, and an unmapped
/// `-subcommands` entry is the namespace's member in 8.5 and a relative
/// lookup from it in 8.6+ (evidence/tcl/ensemble-call-dispatches-its-target.md).
#[test]
fn literal_ensemble_create_freezes_its_dispatch_table() {
    use crate::state_transition::EnsembleDispatchTarget::{Qualified, Relative};
    use tcl_dialect::TclVersion;
    let dispatch = |version: TclVersion, words: &[&str]| {
        super::literal_ensemble_dispatch(
            InvocationArguments::literals(words)
                .with_dialect(crate::InvocationDialect::for_version(version)),
        )
    };
    let owned = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
    let table = dispatch(
        TclVersion::V8_6,
        &[
            "ensemble",
            "create",
            "-map",
            "go A go {B x} two T",
            "-prefixes",
            "0",
        ],
    )
    .unwrap();
    assert_eq!(
        table.entries,
        [
            ("go".to_owned(), Qualified(owned(&["B", "x"]))),
            ("two".to_owned(), Qualified(owned(&["T"]))),
        ]
    );
    assert!(!table.prefixes && !table.unknown_handler);
    assert_eq!(table.select("go"), Some(&Qualified(owned(&["B", "x"]))));
    assert_eq!(table.select("g"), None);
    for (version, unmapped) in [
        (TclVersion::V8_5, Qualified(owned(&["y"]))),
        (TclVersion::V8_6, Relative("y".to_owned())),
    ] {
        let table = dispatch(
            version,
            &[
                "ensemble",
                "create",
                "-subcommands",
                "x y",
                "-map",
                "x ::m::y z ::m::z",
                "-unknown",
                "h",
            ],
        )
        .unwrap();
        assert_eq!(
            table.entries,
            [
                ("x".to_owned(), Qualified(owned(&["::m::y"]))),
                ("y".to_owned(), unmapped),
            ]
        );
        assert!(table.prefixes && table.unknown_handler);
        assert_eq!(table.select("x"), Some(&Qualified(owned(&["::m::y"]))));
    }
    for frozen_nothing in [
        &["ensemble", "create"][..],
        &["ensemble", "create", "-map", "", "-subcommands", ""],
        &["ensemble", "create", "-map", "a ::b", "-parameters", "p"],
        &["ensemble", "create", "-com", "x", "-map", "a ::b"],
        &["ensemble", "configure", "::e", "-map", "a ::b"],
    ] {
        assert_eq!(dispatch(TclVersion::V8_6, frozen_nothing), None);
    }
    assert!(
        dispatch(
            TclVersion::V8_6,
            &["ensemble", "create", "-map", "a ::b", "-parameters", ""]
        )
        .is_some()
    );
}

#[test]
fn namespace_delete_retains_each_recursive_root() {
    let transitions = namespace_delete_state_transitions(InvocationArguments::literals(&[
        "delete", "::pkg", "::other",
    ]));

    assert!(matches!(
        transitions.facts(),
        [first, second]
            if matches!(
                &first.transition,
                StateTransition::Namespace(NamespaceTransition::Delete {
                    namespace: NamespaceTransitionTarget::Named(TransitionSubject::LocatedLiteral { value: name, .. }),
                }) if name == "::pkg"
            ) && matches!(
                &second.transition,
                StateTransition::Namespace(NamespaceTransition::Delete {
                    namespace: NamespaceTransitionTarget::Named(TransitionSubject::LocatedLiteral { value: name, .. }),
                }) if name == "::other"
            )
    ));
}

/// The oracle table, transcribed from tclsh 9.0.4 and 8.6.14 (the four
/// edge rows tabulated here are the last four plus `:::`).  Both
/// interpreters produced **byte-identical**
/// output for every row, so one table pins both.
///
/// ```text
/// $ tclsh9.0 / tclsh8.6
/// namespace qualifiers ::a::b::c   -> ::a::b      tail -> c
/// namespace qualifiers a::b        -> a           tail -> b
/// namespace qualifiers c           -> {}          tail -> c
/// namespace qualifiers {}          -> {}          tail -> {}
/// namespace qualifiers ::          -> {}          tail -> {}
/// namespace qualifiers :::         -> {}          tail -> {}
/// namespace qualifiers a:::b       -> a           tail -> b
/// namespace qualifiers ::a::b::    -> ::a::b      tail -> {}
/// namespace qualifiers ::x:y       -> {}          tail -> x:y
/// namespace qualifiers ::foo       -> {}          tail -> foo
/// namespace qualifiers foo::       -> foo         tail -> {}
/// namespace qualifiers ::a::b::c:: -> ::a::b::c   tail -> {}
/// namespace qualifiers a           -> {}          tail -> a
/// namespace qualifiers :           -> {}          tail -> :
/// namespace qualifiers ::::        -> {}          tail -> {}
/// namespace qualifiers x::y::z     -> x::y        tail -> z
/// namespace qualifiers { a::b }    -> { a}        tail -> {b }
/// namespace qualifiers {a::b c::d} -> {a::b c}    tail -> d
/// namespace qualifiers a::         -> a           tail -> {}
/// namespace qualifiers ::a         -> {}          tail -> a
/// ```
const ORACLE: &[(&str, &str, &str)] = &[
    ("::a::b::c", "::a::b", "c"),
    ("a::b", "a", "b"),
    ("c", "", "c"),
    ("", "", ""),
    ("::", "", ""),
    (":::", "", ""),
    ("a:::b", "a", "b"),
    ("::a::b::", "::a::b", ""),
    ("::x:y", "", "x:y"),
    ("::foo", "", "foo"),
    ("foo::", "foo", ""),
    ("::a::b::c::", "::a::b::c", ""),
    ("a", "", "a"),
    (":", "", ":"),
    ("::::", "", ""),
    ("x::y::z", "x::y", "z"),
    ("::ticklecharts::Gauge", "::ticklecharts", "Gauge"),
    (" a::b ", " a", "b "),
    ("a::b c::d", "a::b c", "d"),
    ("a::", "a", ""),
    ("::a", "", "a"),
];

#[test]
fn qualifiers_and_tail_match_the_oracle_table() {
    for &(input, want_q, want_t) in ORACLE {
        assert_eq!(
            fold_qualifiers(&[input]).as_deref(),
            Some(want_q),
            "namespace qualifiers {input:?}"
        );
        assert_eq!(
            fold_tail(&[input]).as_deref(),
            Some(want_t),
            "namespace tail {input:?}"
        );
    }
}

#[test]
fn folds_are_utf8_safe_on_multibyte_names() {
    // `:` is ASCII, so it never appears inside a multi-byte sequence —
    // every cut point is a char boundary.  Slicing bytes would panic if
    // this were not so.
    assert_eq!(fold_qualifiers(&["é::b"]).as_deref(), Some("é"));
    assert_eq!(fold_tail(&["é::b"]).as_deref(), Some("b"));
    assert_eq!(fold_qualifiers(&["a::é"]).as_deref(), Some("a"));
    assert_eq!(fold_tail(&["a::é"]).as_deref(), Some("é"));
    assert_eq!(fold_qualifiers(&["日本::語"]).as_deref(), Some("日本"));
    assert_eq!(fold_tail(&["日本::語"]).as_deref(), Some("語"));
}

#[test]
fn folds_decline_on_a_wrong_argument_count() {
    // Both subcommands are `arity: exact(1)`; a call tclsh would reject
    // must not fold to a value.
    assert_eq!(fold_qualifiers(&[]), None);
    assert_eq!(fold_tail(&[]), None);
    assert_eq!(fold_qualifiers(&["a", "b"]), None);
    assert_eq!(fold_tail(&["a", "b"]), None);
}

#[test]
fn subcommands_carry_the_folds_through_the_registry() {
    // Registry-level wiring: the optimiser reaches the fold through
    // `SubCommand::run_const_fold`, never by calling the function
    // directly.
    let spec = super::spec();
    let q = spec
        .subcommands
        .iter()
        .find(|s| s.name == "qualifiers")
        .expect("qualifiers subcommand");
    let t = spec
        .subcommands
        .iter()
        .find(|s| s.name == "tail")
        .expect("tail subcommand");
    assert_eq!(
        q.run_const_fold(&["::a::b::c"], Some(tcl_dialect::TclVersion::V9_0))
            .as_deref(),
        Some("::a::b")
    );
    assert_eq!(
        t.run_const_fold(&["::a::b::c"], Some(tcl_dialect::TclVersion::V8_6))
            .as_deref(),
        Some("c")
    );
}

#[test]
fn upvar_forms_own_the_release_specific_zero_pair_arity() {
    let spec = super::spec();
    let upvar = spec
        .subcommands
        .iter()
        .find(|sub| sub.name == "upvar")
        .expect("upvar subcommand");
    let accepts = |dialect: Option<SurfaceQuery<'_>>, argc| {
        upvar.subcommand_forms.iter().any(|form| {
            form.surface
                .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
                && form.arity.accepts(argc)
        })
    };

    assert!(!accepts(Some(SurfaceQuery::core(Family::Tcl, "8.5")), 1));
    assert!(accepts(Some(SurfaceQuery::core(Family::Tcl, "8.5")), 3));
    for dialect in [
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        Some(SurfaceQuery::core(Family::Tcl, "9.1")),
    ] {
        assert!(accepts(dialect, 1));
        assert!(accepts(dialect, 3));
        assert!(!accepts(dialect, 2));
    }
}

#[test]
fn upvar_forms_preserve_namespace_and_repeated_local_roles() {
    use crate::ArgRole;

    let authored = spec();
    let upvar = authored
        .subcommands
        .iter()
        .find(|sub| sub.name == "upvar")
        .unwrap();
    for form in upvar.subcommand_forms {
        assert_eq!(form.arg_roles, [(0, ArgRole::NamespaceName)]);
    }
    for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(environment).commands();
        let args = ["upvar", "::ns", "remote", "local", "other", "second"];
        assert_eq!(
            registry.arg_indices_for_role("namespace", &args, ArgRole::NamespaceName),
            [1],
            "{environment}: selected release form retains the namespace operand",
        );
        assert_eq!(
            registry.arg_indices_for_role("namespace", &args, ArgRole::VarWrite),
            [3, 5],
            "{environment}: both local alias names retain their repeated role",
        );
        assert_eq!(
            registry.arg_indices_for_role(
                "namespace",
                &["tail", "::ns::v"],
                ArgRole::NamespaceName
            ),
            [] as [usize; 0]
        );
        assert_eq!(
            registry.arg_indices_for_role("namespace", &["tail", "::ns::v"], ArgRole::VarWrite),
            [] as [usize; 0]
        );
    }
    let registry = crate::model::ingress::static_context_for("tcl8.4").commands();
    let selected = registry
        .resolve_invocation(
            "namespace",
            &["upvar", "::ns", "remote", "local"],
            registry.own_surface_query(),
        )
        .unwrap();
    assert!(selected.subcommand.resolved().is_none());
}

#[test]
fn ensemble_management_operations_are_registry_nested_subcommands() {
    // The word after `namespace ensemble` is its own Tcl dispatcher.
    // Keep its exact closed surface and unique-prefix behaviour in the
    // registry so hover/completion/tokens never need a `namespace` branch.
    let spec = super::spec();
    let ensemble = spec
        .subcommands
        .iter()
        .find(|sub| sub.name == "ensemble")
        .expect("ensemble subcommand");
    let names: Vec<_> = ensemble
        .sub_subcommands
        .iter()
        .map(|sub| sub.name)
        .collect();
    assert_eq!(names, ["create", "configure", "exists"]);

    assert_eq!(
        ensemble
            .resolve_sub_subcommand_for_dialect("cre", Some(SurfaceQuery::core(Family::Tcl, "9.0")))
            .map(|sub| sub.name),
        Some("create")
    );
    assert_eq!(
        ensemble
            .resolve_sub_subcommand_for_dialect(
                "conf",
                Some(SurfaceQuery::core(Family::Tcl, "8.5"))
            )
            .map(|sub| sub.name),
        Some("configure")
    );
    assert!(
        ensemble
            .resolve_sub_subcommand_for_dialect("e", Some(SurfaceQuery::core(Family::Tcl, "8.4")))
            .is_none()
    );
    assert!(ensemble.resolve_sub_subcommand("c").is_none());
}

/// `namespace qualifiers` and `namespace tail` split the word they are given
/// and look no namespace up, on every release. Run with the current
/// namespace `::other` and namespaces that do not exist:
///
/// ```tcl
/// puts [list [namespace tail ::no::such::ns::x] [namespace qualifiers ::no::such::ns::x]]
/// namespace eval ::other {puts [list [namespace tail a::b] [namespace qualifiers a::b]]}
/// ```
///
/// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print `x ::no::such::ns`
/// and `b a`.
#[test]
fn namespace_string_subcommands_read_no_namespace() {
    let spec = spec();
    for name in ["qualifiers", "tail"] {
        let sub = spec
            .subcommands
            .iter()
            .find(|sub| sub.name == name)
            .expect("namespace subcommand");
        assert!(
            !sub.side_effects.is_unknown() && sub.side_effects.lower_bound().is_empty(),
            "namespace {name}: {:?}",
            sub.side_effects
        );
    }
}
