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

use super::{CommandRegistry, MethodDispatchKind};
use crate::ArgRole;
use crate::forms::LiteralArgumentPrefix;
use crate::spec::ArgRoleResolver;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::SpecProvider;
use tcl_dialect::model::{Family, SpecSurface, SurfaceLayer, SurfaceQuery};
use tcl_dialect::surface;

#[test]
fn native_registration_descriptor_queries_remove_one_root_marker() {
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for label in [
            "::::set",
            ":::::set",
            "::::tcl::info::exists",
            "::::oo::define::method",
        ] {
            assert_eq!(
                registry.native_compilation_for_registration(label, dialect),
                None,
                "{version:?}/{label}"
            );
            assert_eq!(
                registry.native_registration_lookup(label, dialect),
                None,
                "{version:?}/{label}"
            );
        }
        let normal = registry.native_compilation_for_registration("set", dialect);
        assert!(normal.is_some());
        assert_eq!(
            registry.native_compilation_for_registration("::set", dialect),
            normal
        );
        assert_eq!(
            registry.native_compilation_for_registration("set", dialect),
            normal
        );
        let private = registry.native_compilation_for_registration("tcl::info::exists", dialect);
        assert_eq!(private.is_some(), version >= tcl_dialect::TclVersion::V8_5);
        assert_eq!(
            registry.native_compilation_for_registration("::tcl::info::exists", dialect),
            private
        );
    }
}

#[test]
fn native_registration_cache_preserves_every_dialect_and_frozen_generation() {
    let mut registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for key in [
            "set",
            "tcl::info::exists",
            "tcl::namespace::origin",
            "missing::worker",
        ] {
            let expected = registry.native_compilation_for_registration_uncached(key, dialect);
            assert_eq!(
                registry.native_compilation_for_registration(key, dialect),
                expected
            );
            assert_eq!(
                registry.native_compilation_for_registration(&format!("::{key}"), dialect),
                expected
            );
            assert_eq!(
                registry.native_registration_lookup(key, dialect),
                registry.native_registration_lookup_uncached(key, dialect)
            );
        }
    }
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let snapshot = registry.snapshot();
    let previous = registry.native_compilation_for_registration("set", dialect);
    assert!(previous.is_some());
    let mut replacement = registry.get("set").unwrap().clone();
    replacement.native_compilation = None;
    registry.insert(replacement);
    assert!(
        registry
            .native_compilation_for_registration("set", dialect)
            .is_none()
    );
    assert_eq!(
        snapshot
            .registry()
            .native_compilation_for_registration("set", dialect),
        previous
    );
    assert!(!Arc::ptr_eq(
        &registry.native_registrations,
        &snapshot.registry().native_registrations
    ));
}

#[test]
fn representation_hint_requires_only_the_live_option_prefix() {
    use crate::InvocationWord::{Dynamic, Literal};
    let context = crate::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    for (arguments, subject) in [
        (vec![Literal("map"), Literal("a b"), Dynamic], 2),
        (
            vec![Literal("map"), Literal("-nocase"), Literal("a b"), Dynamic],
            3,
        ),
    ] {
        let words = crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            registry
                .arg_type_hint_words(words, subject)
                .unwrap()
                .expected,
            Some(crate::TclType::String)
        );
    }
    let arguments = [Literal("map"), Dynamic, Dynamic];
    let words = crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(registry.arg_type_hint_words(words, 2).is_none());
}

#[test]
fn form_selection_counts_available_options_on_the_declared_axis() {
    static OPTIONS: [crate::hover::OptionSpec; 2] = [
        crate::hover::OptionSpec {
            name: "-later",
            surface: Some(SpecSurface::TCL91),
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-package",
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("2.0"),
            ..crate::hover::OptionSpec::DEFAULT
        },
    ];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "positional",
        arity: crate::Arity::exact(1).with_positionals(),
        options: &OPTIONS,
        ..crate::forms::CommandForm::DEFAULT
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "counted",
        required_package: Some("Counted"),
        command_forms: &FORMS,
        ..crate::CommandSpec::DEFAULT
    });
    for (release, args, selected) in [
        ("9.0", &["-later", "x"][..], false),
        ("9.1", &["-later", "x"][..], true),
        ("9.1", &["-package", "x"][..], true),
    ] {
        let resolved = registry
            .resolve_invocation(
                "counted",
                args,
                Some(SurfaceQuery::core(Family::Tcl, release)),
            )
            .unwrap();
        assert_eq!(resolved.form.is_some(), selected, "{release} {args:?}");
    }
    registry.insert_ambient_package("Counted", "1.0");
    assert!(
        registry
            .resolve_invocation(
                "counted",
                &["-package", "x"],
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .unwrap()
            .form
            .is_none()
    );
    registry.insert_ambient_package("Counted", "2.0");
    let resolved = registry
        .resolve_invocation(
            "counted",
            &["-package", "x"],
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        )
        .unwrap();
    assert_eq!(resolved.argument_count_for_arity(), Some(1));
    assert_eq!(resolved.form.map(|form| form.name), Some("positional"));
}

#[test]
fn default_form_integer_uses_the_retained_invocation_grammar() {
    static SUB: [crate::SubCommand; 1] = [crate::SubCommand {
        name: "replace",
        ..crate::SubCommand::DEFAULT
    }];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "getter",
        arity: crate::Arity::exact(1),
        ..crate::forms::CommandForm::DEFAULT
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "payload_fixture",
        default_form_first_word: Some(crate::spec::DefaultFormFirstWord::Integer),
        subcommands: &SUB,
        command_forms: &FORMS,
        ..crate::CommandSpec::DEFAULT
    });
    for (release, spelling, selected) in [
        ("8.4", "100", true),
        ("9.1", "0d100", true),
        ("8.4", "0d100", false),
    ] {
        let args = [spelling];
        let invocation = registry
            .resolve_invocation(
                "payload_fixture",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, release)),
            )
            .unwrap();
        assert_eq!(
            invocation.form.map(|form| form.name),
            selected.then_some("getter")
        );
    }
    let args = [crate::InvocationWord::Dynamic];
    let unresolved = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("payload_fixture"),
                &args,
            ),
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        )
        .resolved()
        .unwrap();
    assert!(unresolved.form.is_none());
    assert!(matches!(
        unresolved.subcommand,
        crate::SubcommandResolution::Indeterminate { .. }
    ));
}

#[test]
fn private_registration_facts_keep_actual_operand_indices() {
    // Implementation contract: naming.invocation.effective-transition-operands
    // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

    let registry = CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
    for (head, args, canonical) in [
        ("::tcl::info::exists", vec!["x"], "info"),
        ("::tcl::string::equal", vec!["a", "b"], "string"),
        ("::tcl::dict::set", vec!["d", "k", "v"], "dict"),
        ("::tcl::array::exists", vec!["a"], "array"),
    ] {
        let words = crate::InvocationWords::literals(head, &args).with_dialect(dialect);
        let facts = registry
            .native_registration_invocation_facts(words)
            .unwrap();
        assert_eq!(facts.canonical_command, canonical, "{head}");
        assert_eq!(facts.argument_offset, 0, "{head}");
        assert!(
            registry
                .native_registration_completion_route(words, crate::VariableAliasFrame::Procedure)
                .is_some(),
            "{head}"
        );
        if let crate::native_compilation::NativeCompilationGrammar::Dictionary {
            ensemble, ..
        } = facts.native_compilation.unwrap().grammar
        {
            assert!(!ensemble);
        }
    }
    assert!(
        registry
            .native_registration_invocation_facts(
                crate::InvocationWords::literals("::tcl::info::missing", &[]).with_dialect(dialect)
            )
            .is_none()
    );
    assert!(
        registry
            .native_registration_invocation_facts(
                crate::InvocationWords::literals("::tcl::info::exists", &["x"]).with_dialect(
                    crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
                )
            )
            .is_none()
    );
    let args = [crate::InvocationWord::DynamicNonOption];
    let words = crate::InvocationWords::structured(
        crate::InvocationWord::Literal("::tcl::info::exists"),
        &args,
    )
    .with_dialect(dialect);
    assert_eq!(
        registry
            .native_registration_invocation_facts(words)
            .unwrap()
            .argument_offset,
        0
    );
    for argument in [
        crate::InvocationWord::Literal("::A"),
        crate::InvocationWord::Dynamic,
    ] {
        let arguments = [argument];
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("::tcl::namespace::path"),
            &arguments,
        )
        .with_dialect(dialect);
        let facts = registry
            .native_registration_invocation_facts(words)
            .unwrap();
        let path = facts
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .find_map(|fact| {
                if let crate::StateTransition::Namespace(crate::NamespaceTransition::SetPath {
                    path,
                    ..
                }) = &fact.transition
                {
                    Some(path)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(path.argument_index(), Some(0));
        assert_eq!(path.literal(), argument.literal());
    }
}

#[test]
fn native_registration_intrinsics_are_member_metadata_without_placeholder_arguments() {
    let registry = CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    assert_eq!(
        registry.native_registration_intrinsic_ids("tcl::string::length", dialect),
        Some(vec![crate::IntrinsicId::StringLength])
    );
    assert_eq!(
        registry.native_registration_intrinsic_ids("tcl::string::index", dialect),
        Some(vec![crate::IntrinsicId::StringIndex])
    );
    assert!(
        registry
            .native_registration_intrinsic_ids("unregistered", dialect)
            .is_none()
    );
    let root = registry
        .native_registration_intrinsic_ids("string", dialect)
        .unwrap();
    assert!(root.contains(&crate::IntrinsicId::StringLength));
    assert!(root.contains(&crate::IntrinsicId::StringIndex));
}

#[test]
fn original_registration_source_descriptor_retains_parent_and_override_provenance() {
    // Implementation contract: naming.namespace.original-quiet-pattern-completion
    // docs/design/analysis/name-resolution-proofs/namespace-original-quiet-pattern-completion.md
    let mut registry = CommandRegistry::build_default();
    let realm = tcl_dialect::model::InvocationRealm::RuleLoader;
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let root = registry
            .native_registration_source_descriptor("::namespace", dialect, realm)
            .unwrap();
        assert_eq!(root.name, "namespace");
        assert_eq!(super::spec_pack_of(root), Some("tcl"));
        let private = registry.native_registration_source_descriptor(
            "::tcl::namespace::export",
            dialect,
            realm,
        );
        if version < tcl_dialect::TclVersion::V8_6 {
            assert!(private.is_none(), "{version:?}");
        } else {
            assert!(std::ptr::eq(private.unwrap(), root), "{version:?}");
        }
        assert!(
            registry
                .native_registration_source_descriptor(
                    "tcl::namespace::unregistered",
                    dialect,
                    realm
                )
                .is_none()
        );
    }
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let surface = registry
        .get_for_surface("namespace", dialect.authoring_query())
        .unwrap()
        .surface;
    registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
        name: "tcl::namespace::export",
        surface,
        ..crate::CommandSpec::DEFAULT
    })));
    let authored = registry
        .native_registration_source_descriptor("tcl::namespace::export", dialect, realm)
        .unwrap();
    assert_eq!(authored.name, "tcl::namespace::export");
    assert_eq!(
        super::spec_pack_of(authored),
        None,
        "an explicit authored helper descriptor cannot borrow its stock parent's pack"
    );
    let unknown = crate::InvocationDialect {
        native_family: None,
        core_point: None,
        tcl_version: None,
        ..dialect
    };
    assert!(
        registry
            .native_registration_source_descriptor("namespace", unknown, realm)
            .is_none()
    );
}

#[test]
fn string_repeat_registration_has_its_actual_basic_worker_compiler() {
    use tcl_dialect::TclVersion;
    let registry = CommandRegistry::build_default();
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        let worker = registry.native_compilation_for_registration("tcl::string::repeat", dialect);
        if version == TclVersion::V8_4 {
            assert!(worker.is_none());
        } else {
            assert_eq!(
                worker.unwrap().compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_6)
            );
        }
    }
    let mut unknown = crate::InvocationDialect::for_version(TclVersion::V9_0);
    unknown.native_family = None;
    unknown.core_point = None;
    unknown.tcl_version = None;
    assert!(
        registry
            .native_compilation_for_registration("tcl::string::repeat", unknown)
            .is_none()
    );
}

#[test]
fn native_registration_compilation_keeps_private_member_and_version() {
    let registry = CommandRegistry::build_default();
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        let private = registry.native_compilation_for_registration("tcl::info::exists", dialect);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert!(private.is_none());
        } else {
            assert_eq!(
                private.unwrap().grammar,
                crate::native_compilation::NativeCompilationGrammar::InfoExists
            );
            assert_eq!(private.unwrap().compiler_hook_presence(dialect), Some(true));
            assert_eq!(
                private,
                registry.native_compilation_for_registration("::tcl::info::exists", dialect)
            );
        }
        assert!(
            registry
                .native_compilation_for_registration("unknown::worker", dialect)
                .is_none()
        );
    }
}

#[test]
fn normal_worker_registration_retains_its_independent_compiler_descriptor() {
    let mut registry = CommandRegistry::build_default();
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        let descriptor = registry
            .native_compilation_for_registration("tcl::namespace::origin", dialect)
            .expect("the original registered worker authors its own compiler hook");
        assert_eq!(descriptor.compiler_hook_presence(dialect), Some(true));
        assert_eq!(
            descriptor,
            registry
                .native_compilation_for_registration("::tcl::namespace::origin", dialect)
                .unwrap()
        );
        assert!(
            registry
                .native_compilation_for_registration("tcl::namespace::unregistered", dialect)
                .is_none()
        );
    }
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let unknown = crate::InvocationDialect {
        native_family: None,
        core_point: None,
        tcl_version: None,
        ..dialect
    };
    assert!(
        registry
            .native_compilation_for_registration("tcl::namespace::origin", unknown)
            .is_none()
    );
    let replacement_surface = registry
        .get_for_surface("namespace", Some(dialect.authoring_query().unwrap()))
        .unwrap()
        .surface;
    registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
        name: "namespace",
        surface: replacement_surface,
        subcommands: &[crate::SubCommand {
            name: "origin",
            ..crate::SubCommand::DEFAULT
        }],
        ..crate::CommandSpec::DEFAULT
    })));
    assert!(
        registry
            .native_compilation_for_registration("tcl::namespace::origin", dialect)
            .is_none(),
        "replacement metadata supplies no original worker registration or compiler descriptor"
    );
}

#[test]
fn nested_compiler_registration_retains_each_independent_path_row() {
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for (identity, hook) in [
            ("tcl::binary::encode", true),
            ("tcl::binary::decode", true),
            ("tcl::binary::encode::hex", true),
            ("tcl::binary::encode::base64", false),
            ("tcl::binary::encode::uuencode", false),
            ("tcl::binary::decode::hex", true),
            ("tcl::binary::decode::base64", true),
            ("tcl::binary::decode::uuencode", true),
        ] {
            let descriptor = registry.native_compilation_for_registration(identity, dialect);
            if version < tcl_dialect::TclVersion::V8_6 {
                assert!(descriptor.is_none(), "{version:?}: {identity}");
            } else {
                assert_eq!(
                    descriptor.and_then(|spec| spec.compiler_hook_presence(dialect)),
                    Some(hook),
                    "{version:?}: {identity}"
                );
                assert_eq!(
                    descriptor,
                    registry.native_compilation_for_registration(&format!("::{identity}"), dialect)
                );
            }
        }
    }
    let owner = crate::model::ingress::static_context_for("jim");
    let dialect = crate::InvocationDialect::of_profile(
        owner.commands().profile().expect("selected Jim fixture"),
    );
    assert!(
        registry
            .native_compilation_for_registration("tcl::binary::encode::hex", dialect)
            .is_none()
    );
}

#[test]
fn conflicting_private_registration_mappings_retain_uncertainty() {
    use crate::native_compilation::{
        NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
        NativeCompilerImplementationLookup,
    };
    const FIRST: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
        ensemble: "::alpha",
        member: "x",
        slot: "::shared_worker",
        command: "alpha",
        prepended: &[],
    };
    const SECOND: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
        ensemble: "::beta",
        member: "x",
        slot: "::shared_worker",
        command: "beta",
        prepended: &[],
    };
    let mut registry = CommandRegistry::build_default();
    for (name, lookup) in [("alpha", &FIRST), ("beta", &SECOND)] {
        registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
            name,
            arity: crate::Arity::exact(0),
            native_compilation: Some(NativeCompilationSpec {
                grammar: NativeCompilationGrammar::NamedEnsembleInvocation {
                    lookup,
                    implementation_from: tcl_dialect::TclVersion::V8_6,
                    hook_from: tcl_dialect::TclVersion::V8_6,
                    arity: crate::Arity::exact(0),
                    later_arity: None,
                },
                operation: crate::SemanticOperationId::Invoke,
                body: NativeBodyCompilation::Inherit,
            }),
            ..crate::CommandSpec::DEFAULT
        })));
    }
    let dialect =
        crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    assert!(
        registry
            .native_compilation_for_registration("shared_worker", dialect)
            .is_none()
    );
    assert!(
        registry
            .native_registration_invocation_facts(
                crate::InvocationWords::literals("shared_worker", &[]).with_dialect(dialect)
            )
            .is_none()
    );
}

fn body_backed(name: &'static str, backing: crate::RuntimeBacking) -> crate::CommandSpec {
    crate::CommandSpec {
        name,
        runtime_backing: backing,
        ..crate::CommandSpec::DEFAULT
    }
}

fn text_backing(text: &'static str) -> crate::RuntimeBacking {
    crate::RuntimeBacking::pack_text(text)
}

fn origin() -> crate::pack_origin::PackOrigin {
    crate::pack_origin::PackOrigin {
        pack: "vendor".to_owned(),
        content_hash: 7,
        vocabulary_version: "2".to_owned(),
        provenance: tcl_dialect::model::Provenance::BundledPack,
    }
}

/// The reference bodies a registry offers are the specs a pack installed,
/// whose backing is a Tcl body this registry holds the text of, and which
/// are still the answer for their name: the text a `PackText` carries or a
/// `PackageSource` resolved to at load, never an embedder's own spec, a
/// backing of another kind, a `PackageSource` nobody read, or a spec a later
/// insertion shadowed.
#[test]
fn a_registry_offers_the_live_pack_installed_bodies_it_holds_text_for() {
    let installed = |registry: &mut CommandRegistry, spec: crate::CommandSpec| {
        let spec: &'static crate::CommandSpec = Box::leak(Box::new(spec));
        registry.insert_static(spec);
        registry.insert_pack_origin(spec, origin());
        spec
    };
    let mut registry = CommandRegistry::build_default();
    let in_text = installed(
        &mut registry,
        body_backed("vendor::text", text_backing("proc vendor::text {} {}")),
    );
    let from_file = installed(
        &mut registry,
        body_backed(
            "vendor::file",
            crate::RuntimeBacking::package_source("lib.tcl"),
        ),
    );
    registry.insert_reference_text(from_file, Arc::from("proc vendor::file {} {}"));
    // Not offered: nobody read the file; the host owns the command; the
    // embedder inserted it; and a later spec shadowed it.
    installed(
        &mut registry,
        body_backed(
            "vendor::unread",
            crate::RuntimeBacking::package_source("absent.tcl"),
        ),
    );
    installed(
        &mut registry,
        body_backed("vendor::native", crate::RuntimeBacking::HostNative),
    );
    registry.insert(body_backed(
        "embedder::text",
        text_backing("proc embedder::text {} {}"),
    ));
    installed(
        &mut registry,
        body_backed(
            "vendor::shadowed",
            text_backing("proc vendor::shadowed {} {}"),
        ),
    );
    registry.insert(body_backed(
        "vendor::shadowed",
        crate::RuntimeBacking::HostNative,
    ));

    let offered: Vec<(&str, &str)> = registry
        .reference_bodies()
        .map(|(spec, text)| (spec.name, text))
        .collect();
    assert_eq!(
        offered,
        vec![
            ("vendor::text", "proc vendor::text {} {}"),
            ("vendor::file", "proc vendor::file {} {}"),
        ]
    );
    assert_eq!(
        registry.reference_body(in_text),
        Some("proc vendor::text {} {}")
    );
    assert_eq!(
        registry.reference_body(from_file),
        Some("proc vendor::file {} {}")
    );
    // A projection for a profile carries the text with the spec.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    assert_eq!(
        registry
            .project_for_profile(profile)
            .reference_bodies()
            .count(),
        2
    );
}

/// Tcl 9.0.4: inside a method the bare words resolve to the receiving
/// object's `my` command or the `::oo::Helpers` namespace path, while every
/// global `::keyword` spelling is absent. Generic registry lookup must not
/// turn those contextual bare specs into nonexistent global commands.
#[test]
fn rooted_contextual_commands_do_not_fall_back_to_bare_specs() {
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    for (head, expected) in [
        ("my", MethodDispatchKind::SelfDispatch),
        ("next", MethodDispatchKind::NextChain),
        ("nextto", MethodDispatchKind::NextChain),
        ("self", MethodDispatchKind::Introspection),
    ] {
        assert_eq!(registry.method_dispatch_keyword(head), Some(expected));
        assert_eq!(
            registry.method_dispatch_keyword(&format!("::{head}")),
            None,
            "tclsh9.0.4 has no global ::{head} command"
        );
    }
    for qualified in [
        "oo::Helpers::next",
        "::oo::Helpers::next",
        "oo::Helpers::nextto",
        "::oo::Helpers::nextto",
        "oo::Helpers::self",
        "::oo::Helpers::self",
    ] {
        assert_eq!(registry.method_dispatch_keyword(qualified), None);
    }

    assert!(registry.is_self_receiver_call("self", None));
    assert!(registry.is_self_receiver_call("self", Some("object")));
    assert!(!registry.is_self_receiver_call("::self", None));
    assert!(!registry.is_self_receiver_call("::self", Some("object")));
    for qualified in ["oo::Helpers::self", "::oo::Helpers::self"] {
        assert!(
            registry.is_self_receiver_call(qualified, None),
            "no-argument {qualified} is the real qualified helper command"
        );
        assert!(
            registry.is_self_receiver_call(qualified, Some("object")),
            "{qualified} is the real qualified helper command"
        );
    }

    for head in [
        "callback",
        "classvariable",
        "link",
        "my",
        "mymethod",
        "next",
        "nextto",
        "self",
    ] {
        let rooted = format!("::{head}");
        assert!(registry.get(head).is_some(), "bare {head}");
        assert!(registry.get(&rooted).is_none(), "rooted {head}");
        assert!(
            registry
                .get_for_surface(&rooted, registry.own_surface_query())
                .is_none(),
            "profiled rooted {head}"
        );
        assert!(!registry.has_command_in_this_dialect(&rooted), "{rooted}");
        assert_eq!(
            registry.known_in_any_dialect(&rooted),
            head == "next",
            "only BPF-Tcl independently declares a non-contextual global {rooted}"
        );
        assert!(!registry.declares_command_at(&rooted), "{rooted}");
        assert!(
            !registry.resolves_only_in_method_context(&rooted),
            "{rooted}"
        );
        assert!(!registry.requires_oo_method_frame(&rooted), "{rooted}");
    }
    assert!(!registry.binds_method_alias("::link"));

    for (rooted, canonical) in [
        ("::set", "set"),
        ("::oo::Helpers::self", "oo::Helpers::self"),
    ] {
        assert_eq!(registry.get(rooted).map(|spec| spec.name), Some(canonical));
        assert!(registry.has_command_in_this_dialect(rooted), "{rooted}");
        assert!(registry.known_in_any_dialect(rooted), "{rooted}");
        assert!(registry.declares_command_at(rooted), "{rooted}");
    }
}

/// Dynamic-role capability declarations are a closed, conservative
/// registry contract.  Source-aware callers cannot run a resolver after
/// expansion, so every emitted role must be declared here instead of
/// assuming a particular command's fallback shape.
/// Every shipped surface, not just the always-loaded Tcl, stdlib, tcllib,
/// Itcl, and Tk catalogue.  Resolver capability metadata is equally
/// load-bearing for optional dialect/package overlays.
fn registry_with_every_resolver_surface() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for layer in [
        SurfaceLayer::Package("bpf"),
        SurfaceLayer::Core(Family::F5Irules, ""),
        SurfaceLayer::Package("iapps"),
        SurfaceLayer::Package("tmsh"),
        SurfaceLayer::Package("expect"),
        SurfaceLayer::Package("spectcl"),
    ] {
        registry.load_surface(layer);
    }
    registry
}

fn assert_role_inputs_exclusive(registry: &CommandRegistry) {
    for specs in registry.by_name.values() {
        for spec in specs {
            for inputs in std::iter::once([
                spec.arg_role_resolver.is_some(),
                spec.arg_role_count_resolver.is_some(),
                spec.arg_role_layout_resolver.is_some(),
            ])
            .chain(spec.subcommands.iter().map(|sub| {
                [
                    sub.arg_role_resolver.is_some(),
                    sub.arg_role_count_resolver.is_some(),
                    sub.arg_role_layout_resolver.is_some(),
                ]
            })) {
                assert!(
                    inputs.into_iter().filter(|present| *present).count() <= 1,
                    "{} has conflicting role input contracts",
                    spec.name
                );
            }
        }
    }
}

fn assert_count_role_capabilities(registry: &CommandRegistry) {
    for specs in registry.by_name.values() {
        for spec in specs {
            for (resolver, declared) in
                std::iter::once((spec.arg_role_count_resolver, spec.arg_role_resolver_roles)).chain(
                    spec.subcommands
                        .iter()
                        .map(|sub| (sub.arg_role_count_resolver, sub.arg_role_resolver_roles)),
                )
            {
                if let Some(resolver) = resolver {
                    for count in [0, 3, 5, 256] {
                        for (_, role) in resolver(count) {
                            assert!(
                                declared.contains(&role),
                                "count resolver emitted {role:?} for argc {count}, outside {declared:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

fn assert_command_role_capabilities(registry: &CommandRegistry, name: &str, args: &[&str]) {
    let spec = registry.get(name).expect("registered command");
    let roles = if let Some(resolver) = spec.arg_role_count_resolver {
        resolver(args.len())
    } else if let Some(resolver) = spec.arg_role_layout_resolver {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let options = crate::resolved_invocation::invocation_options(
            spec,
            None,
            None,
            crate::resolved_invocation::InvocationAvailability {
                query: Some(profile.surface_query()),
                package_version: None,
            },
        );
        resolver(
            crate::InvocationArguments::literals(args)
                .with_dialect(crate::InvocationDialect::of_profile(profile)),
            options,
        )
        .expect("literal native layout")
    } else if let Some(resolver) = spec.arg_role_resolver {
        resolver(args)
    } else {
        assert!(!spec.has_dynamic_argument_roles());
        return;
    };
    for (_, role) in roles {
        assert!(
            spec.arg_role_resolver_roles.contains(&role),
            "resolver emitted {role:?} for {args:?}, outside {:?}",
            spec.arg_role_resolver_roles,
        );
    }
}

fn assert_subcommand_role_capabilities(
    registry: &CommandRegistry,
    command_name: &str,
    sub_name: &str,
    args: &[&str],
) {
    let spec = registry.get(command_name).expect("registered command");
    let sub = spec.subcommand(sub_name).expect("registered subcommand");
    let roles = if let Some(resolver) = sub.arg_role_count_resolver {
        resolver(args.len())
    } else if let Some(resolver) = sub.arg_role_layout_resolver {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let options = crate::resolved_invocation::invocation_options(
            spec,
            Some(sub),
            None,
            crate::resolved_invocation::InvocationAvailability {
                query: Some(profile.surface_query()),
                package_version: None,
            },
        );
        resolver(
            crate::InvocationArguments::literals(args)
                .with_dialect(crate::InvocationDialect::of_profile(profile)),
            options,
        )
        .expect("literal native layout")
    } else {
        sub.arg_role_resolver.expect("dynamic role resolver")(args)
    };
    for (_, role) in roles {
        assert!(sub.arg_role_resolver_roles.contains(&role));
    }
}

#[test]
fn dynamic_role_capabilities_cover_every_resolver_and_representative_output() {
    let registry = registry_with_every_resolver_surface();
    let mut resolver_count = 0;
    for specs in registry.by_name.values() {
        for spec in specs {
            if spec.has_dynamic_argument_roles() {
                resolver_count += 1;
                assert!(
                    !spec.arg_role_resolver_roles.is_empty(),
                    "{} has a dynamic role resolver without its closed capability set",
                    spec.name
                );
            }
            for sub in spec.subcommands {
                if sub.has_dynamic_argument_roles() {
                    resolver_count += 1;
                    assert!(
                        !sub.arg_role_resolver_roles.is_empty(),
                        "{} {} has a dynamic role resolver without its closed capability set",
                        spec.name,
                        sub.name
                    );
                }
            }
        }
    }
    assert!(
        resolver_count >= 52,
        "the resolver catalogue unexpectedly shrank"
    );

    assert_role_inputs_exclusive(&registry);

    assert_count_role_capabilities(&registry);

    // Representative resolver families, including the multi-role and
    // discriminator-dependent rows most likely to drift.
    assert_command_role_capabilities(&registry, "set", &["name"]);
    assert_command_role_capabilities(&registry, "set", &["name", "value"]);
    assert_command_role_capabilities(
        &registry,
        "regexp",
        &["pattern", "text", "whole", "capture"],
    );
    assert_command_role_capabilities(
        &registry,
        "regsub",
        &["pattern", "text", "replacement", "out"],
    );
    assert_command_role_capabilities(
        &registry,
        "if",
        &["expr", "then", "body", "else", "fallback"],
    );
    assert_command_role_capabilities(&registry, "try", &["body", "on", "0", "result", "handler"]);
    assert_subcommand_role_capabilities(&registry, "binary", "scan", &["bytes", "a*", "out"]);
    assert_subcommand_role_capabilities(
        &registry,
        "dict",
        "update",
        &["d", "key", "local", "body"],
    );
    assert_subcommand_role_capabilities(&registry, "namespace", "which", &["-variable", "name"]);
    assert_subcommand_role_capabilities(&registry, "namespace", "which", &["-command", "name"]);
    assert_subcommand_role_capabilities(
        &registry,
        "trace",
        "add",
        &["variable", "name", "write", "callback"],
    );
    // A clause grammar took the retired `if` / `try` resolvers' place; its
    // walk's flat roles stay inside the same closed capability set.
    let check_grammar = |name: &str, args: &[&str]| {
        let spec = registry.get(name).expect("command");
        let plan = spec.clause_plan(args, None).expect("a clause grammar");
        for (_, role) in plan.roles {
            assert!(
                spec.arg_role_resolver_roles.contains(&role)
                    || spec.arg_roles.iter().any(|(_, found)| *found == role),
                "{name}'s grammar emitted {role:?} for {args:?}, outside its declared roles"
            );
        }
    };
    check_grammar("if", &["expr", "then", "body", "else", "fallback"]);
    check_grammar("try", &["body", "on", "0", "result", "handler"]);
}

/// A clause grammar's flat roles are part of `may_have_arg_role`'s answer,
/// so every role a grammar can assign is one its spec already declares —
/// in the resolver's closed set (the set a grammar that replaced a
/// resolver keeps) or in the static table. Otherwise an expansion-blocked
/// call would abstain differently from a literal one.
#[test]
fn clause_grammar_roles_are_declared_capabilities() {
    let registry = registry_with_every_resolver_surface();
    let mut grammars = 0;
    let declared = |roles: &[ArgRole], table: &[(u8, ArgRole)], role: ArgRole| {
        roles.contains(&role) || table.iter().any(|(_, found)| *found == role)
    };
    for specs in registry.by_name.values() {
        for spec in specs {
            let owners = std::iter::once((
                spec.name.to_owned(),
                spec.clause_grammar,
                spec.arg_role_resolver_roles,
                spec.arg_roles,
            ))
            .chain(spec.subcommands.iter().map(|sub| {
                (
                    format!("{} {}", spec.name, sub.name),
                    sub.clause_grammar,
                    sub.arg_role_resolver_roles,
                    sub.arg_roles,
                )
            }));
            for (owner, grammar, roles, table) in owners {
                let Some(grammar) = grammar else { continue };
                grammars += 1;
                for &role in ArgRole::ALL {
                    assert!(
                        !grammar.may_assign(role) || declared(roles, table, role),
                        "{owner}'s clause grammar can assign {role:?}, which it does not declare"
                    );
                }
            }
        }
    }
    assert!(
        grammars >= 11,
        "the clause-grammar catalogue unexpectedly shrank"
    );
}

/// The representative rows in the sibling test are hand-picked, so a
/// resolver nobody thought to list stays unchecked — `control::do` emitted
/// `ArgRole::Expr` without declaring it for exactly that reason (#2068).
/// Sweep every resolver instead: feed each one the literals its own spec
/// knows about (option names, `arg_values` words, sibling subcommand
/// names), at every position of every arity up to the widest form a
/// resolver in this tree inspects, and require the closed capability set
/// to cover the result.
#[test]
fn dynamic_role_capabilities_cover_every_resolver_argument_shape() {
    let registry = registry_with_every_resolver_surface();
    let mut swept = 0;
    for specs in registry.by_name.values() {
        for spec in specs {
            if let Some(resolver) = spec.arg_role_count_resolver {
                swept += 1;
                for count in 0..=6 {
                    assert!(
                        resolver(count)
                            .iter()
                            .all(|(_, role)| spec.arg_role_resolver_roles.contains(role))
                    );
                }
            }
            if let Some(resolver) = spec.arg_role_layout_resolver {
                swept += 1;
                sweep_layout_resolver(spec, None, resolver);
            }
            if let Some(resolver) = spec.arg_role_resolver {
                swept += 1;
                sweep_resolver(
                    resolver,
                    spec.arg_role_resolver_roles,
                    &resolver_literals(spec, None),
                    spec.name,
                );
            }
            for sub in spec.subcommands {
                if let Some(resolver) = sub.arg_role_count_resolver {
                    swept += 1;
                    for count in 0..=6 {
                        assert!(
                            resolver(count)
                                .iter()
                                .all(|(_, role)| sub.arg_role_resolver_roles.contains(role))
                        );
                    }
                }
                if let Some(resolver) = sub.arg_role_layout_resolver {
                    swept += 1;
                    sweep_layout_resolver(spec, Some(sub), resolver);
                }
                if let Some(resolver) = sub.arg_role_resolver {
                    swept += 1;
                    sweep_resolver(
                        resolver,
                        sub.arg_role_resolver_roles,
                        &resolver_literals(spec, Some(sub)),
                        &format!("{} {}", spec.name, sub.name),
                    );
                }
            }
        }
    }
    // The same floor the capability test asserts, so the two stay in
    // step: a resolver added without a capability set fails there, and
    // one whose emitted roles drift fails here.
    assert!(swept >= 52, "the resolver catalogue unexpectedly shrank");
}

fn sweep_layout_resolver(
    spec: &'static crate::CommandSpec,
    sub: Option<&'static crate::spec::SubCommand>,
    resolver: crate::spec::ArgRoleLayoutResolver,
) {
    let declared = sub.map_or(spec.arg_role_resolver_roles, |sub| {
        sub.arg_role_resolver_roles
    });
    let options = crate::resolved_invocation::invocation_options(
        spec,
        sub,
        None,
        crate::resolved_invocation::InvocationAvailability::default(),
    );
    let literals = resolver_literals(spec, sub);
    for len in 0..=6 {
        let mut words = vec![crate::InvocationWord::Dynamic; len];
        for position in 0..len {
            for literal in &literals {
                words[position] = crate::InvocationWord::Literal(literal);
                if let Some(roles) =
                    resolver(crate::InvocationArguments::structured(&words), options)
                {
                    assert!(
                        roles.iter().all(|(_, role)| declared.contains(role)),
                        "{}: {words:?} -> {roles:?}",
                        spec.name
                    );
                }
            }
            words[position] = crate::InvocationWord::Dynamic;
        }
    }
}

/// The literal words a resolver is plausibly handed: the option names and
/// `arg_values` of the spec it belongs to, plus its sibling subcommand
/// names, which discriminator-dependent resolvers (`trace add`,
/// `namespace which`) branch on.  Deduplicated and bounded so the sweep
/// stays a fast unit test.
fn resolver_literals(
    spec: &'static crate::CommandSpec,
    sub: Option<&'static crate::spec::SubCommand>,
) -> Vec<&'static str> {
    let mut out: BTreeSet<&'static str> = BTreeSet::new();
    let collect = |options: &'static [crate::hover::OptionSpec],
                   arg_values: &'static [(u8, &'static [crate::hover::ArgValue])],
                   out: &mut BTreeSet<&'static str>| {
        for option in options {
            out.insert(option.name);
        }
        for (_, values) in arg_values {
            for value in *values {
                out.insert(value.value);
            }
        }
    };
    collect(spec.options, spec.arg_values, &mut out);
    if let Some(sub) = sub {
        collect(sub.options, sub.arg_values, &mut out);
    }
    for sibling in spec.subcommands {
        out.insert(sibling.name);
    }
    // A bare word and a lone dash stand in for "some value" and "an
    // option-shaped word the table does not know".
    out.insert("x");
    out.insert("-");
    out.into_iter().take(48).collect()
}

/// Call `resolver` with every one-literal substitution into an
/// all-placeholder argument vector, for each arity up to `MAX_WORDS`, and
/// assert each emitted role is declared.
fn sweep_resolver(
    resolver: ArgRoleResolver,
    declared: &[ArgRole],
    literals: &[&'static str],
    label: &str,
) {
    // The widest form any resolver in this tree inspects is `trace add
    // variable name ops callback` plus a trailing word; six covers it
    // with room to spare, and a resolver that only reads a prefix is
    // exercised by the shorter arities in the same loop.
    const MAX_WORDS: usize = 6;
    let check = |args: &[&str]| {
        for (_, role) in resolver(args) {
            assert!(
                declared.contains(&role),
                "`{label}`'s resolver emitted {role:?} for {args:?}, outside its declared {declared:?}"
            );
        }
    };
    for len in 0..=MAX_WORDS {
        let base = vec!["x"; len];
        check(&base);
        for position in 0..len {
            for literal in literals {
                let mut args = base.clone();
                args[position] = literal;
                check(&args);
            }
        }
    }
}

// Cross-language RPC roles.

/// The remote-method relation is registry data on the iRules surface, so
/// it exists there and nowhere else. A consumer asking the registry —
/// rather than naming `ILX::call` — is what makes the dialect gate
/// automatic.
#[test]
fn remote_method_commands_are_an_irules_only_surface() {
    let plain = CommandRegistry::build_default();
    assert!(
        plain.remote_method_commands().is_empty(),
        "stock Tcl has no cross-language RPC commands"
    );
    assert!(plain.remote_method("ILX::call").is_none());

    let mut irules = CommandRegistry::build_default();
    irules.load_irules();
    assert_eq!(
        irules.remote_method_commands(),
        vec!["ILX::call", "ILX::init", "ILX::notify"]
    );
    // The explicitly-global spelling is the same command.
    assert!(irules.remote_method("::ILX::call").is_some());
    assert!(
        irules
            .remote_method("ILX::init")
            .is_some_and(|role| role.opens_handle().is_some())
    );
    assert!(
        irules
            .remote_method("ILX::notify")
            .is_some_and(|role| role.calls_method().is_some())
    );
}

// Ambient packages.

/// Two packs naming the same package are two claims that the runtime
/// provides at least that version. The strongest claim is the one that
/// holds, so the floor is the greatest — never the last declared, which
/// would make the answer depend on pack load order.
#[test]
fn the_highest_declared_ambient_version_is_the_floor() {
    let mut registry = CommandRegistry::build_default();
    assert_eq!(
        registry.ambient_package_floor("Tk"),
        None,
        "a compiled-in registry declares none"
    );

    registry.insert_ambient_package("Tk", "8.6");
    registry.insert_ambient_package("Tk", "8.5");
    registry.insert_ambient_package("Itcl", "4.0");
    assert_eq!(registry.ambient_package_floor("Tk"), Some("8.6"));
    assert_eq!(registry.ambient_package_floor("Itcl"), Some("4.0"));
    assert_eq!(registry.ambient_package_floor("Expect"), None);

    // Order-independent: declaring the higher one last changes nothing.
    let mut reversed = CommandRegistry::build_default();
    reversed.insert_ambient_package("Tk", "8.5");
    reversed.insert_ambient_package("Tk", "8.6");
    assert_eq!(reversed.ambient_package_floor("Tk"), Some("8.6"));
}

// The complement of `arg_indices_for_role`: what the *next* words would
// mean, which is the question a splice-a-trailing-argument quick fix asks.

#[test]
fn unfilled_trailing_roles_reports_the_optional_capture_variables() {
    let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
    // `catch {body}` leaves both `VarWrite` slots open.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}"]),
        vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
    );
    // One supplied leaves one.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}", "res"]),
        vec![(2, ArgRole::VarWrite)]
    );
    // Fully supplied leaves none.
    assert_eq!(
        reg.unfilled_trailing_roles("catch", &["{body}", "res", "opts"])
            .len(),
        0
    );
}

#[test]
fn prospective_trailing_layout_preserves_unknown_source_values() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    let ordinary = [InvocationWord::Literal("aa"), InvocationWord::Dynamic];
    let roles =
        reg.unfilled_trailing_roles_words("regexp", InvocationArguments::structured(&ordinary));
    assert_eq!(roles.first(), Some(&(2, ArgRole::VarWrite)));
    assert!(roles.iter().all(|(_, role)| *role == ArgRole::VarWrite));
    let replacement = [
        InvocationWord::Literal("-command"),
        InvocationWord::Literal("aa"),
        InvocationWord::Dynamic,
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        reg.unfilled_trailing_roles_words("regsub", InvocationArguments::structured(&replacement),),
        vec![(4, ArgRole::VarWrite)]
    );
    for words in [
        vec![InvocationWord::Dynamic, InvocationWord::Dynamic],
        vec![
            InvocationWord::Literal("-inline"),
            InvocationWord::Literal("aa"),
            InvocationWord::Dynamic,
        ],
        vec![InvocationWord::Literal("aa"), InvocationWord::Expanded],
    ] {
        assert!(
            reg.unfilled_trailing_roles_words("regexp", InvocationArguments::structured(&words),)
                .is_empty(),
            "{words:?}"
        );
    }
}

#[test]
fn unfilled_trailing_roles_stops_at_the_arity_ceiling() {
    let reg = CommandRegistry::build_default();
    // Every position past `catch`'s maximum of three is absent, however
    // many are asked for.
    let roles = reg.unfilled_trailing_roles("catch", &["{body}"]);
    assert!(roles.iter().all(|(index, _)| *index < 3), "{roles:?}");
}

#[test]
fn unfilled_trailing_roles_is_empty_for_an_unknown_command() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.unfilled_trailing_roles("no_such_command", &[]).len(), 0);
}

#[test]
fn unfilled_trailing_roles_stops_at_an_undeclared_position() {
    let reg = CommandRegistry::build_default();
    // `puts` declares no trailing role a caller could fill with a
    // *known* meaning, so nothing is offered rather than an unbounded
    // run of `Value` slots.
    assert_eq!(reg.unfilled_trailing_roles("puts", &["hello"]).len(), 0);
}

/// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were the same bit.
///
/// Both were spelled `1 << 61`, so the 65th trait silently aliased the
/// 61st. Because `FRAME_SENSITIVE_TRAITS` unions in `TRANSFERS_CONTROL`,
/// every safe-interp-hidden command — `file`, `source`, `encoding`,
/// `open`, … — read as frame-sensitive and had the inline-proc code
/// action suppressed, while `break`/`continue`/`yield`/`yieldto`/
/// `tailcall` read as safe-interp-hidden.
///
/// The two are now separate enum variants, so the aliasing is
/// unrepresentable rather than merely fixed. This pins the *behaviour*
/// that was wrong, which a type-level guarantee alone does not cover.
#[test]
fn safe_interp_hidden_is_not_control_transfer() {
    let reg = CommandRegistry::build_default();
    assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);

    for name in ["break", "continue", "yield", "yieldto", "tailcall"] {
        let spec = reg.get(name).expect("control-transfer command");
        assert!(
            spec.traits.contains(Traits::TRANSFERS_CONTROL),
            "{name} must transfer control"
        );
        assert!(
            !spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
            "{name} is not hidden in a safe interpreter"
        );
        assert!(reg.is_frame_sensitive(name), "{name} is frame-sensitive");
    }

    // Safe-hidden commands carrying no *other* frame-sensitive trait.
    // These are the ones the alias was wrongly marking: each had the
    // inline-proc code action suppressed on it.
    for name in [
        "file", "encoding", "open", "socket", "exec", "cd", "pwd", "glob", "load", "unload",
    ] {
        let spec = reg.get(name).expect("safe-hidden command");
        assert!(
            spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
            "{name} is hidden in a safe interpreter"
        );
        assert!(
            !spec.traits.contains(Traits::TRANSFERS_CONTROL),
            "{name} does not transfer control"
        );
        assert!(
            !reg.is_frame_sensitive(name),
            "{name} must not be frame-sensitive — the alias suppressed the \
                 inline-proc code action on it"
        );
    }

    // The correction must not swing too far: `source` and `exit` are
    // frame-sensitive on their own merits (a barrier and a block
    // terminator respectively), and stay so.
    for name in ["source", "exit"] {
        assert!(
            reg.is_frame_sensitive(name),
            "{name} is frame-sensitive independently of the trait alias"
        );
    }
}

use super::*;

#[test]
fn get_for_dialect_picks_the_most_specific_visible_spec() {
    // §5.3/D6 — the single selection rule. Three specs of one name,
    // registered deliberately most-specific-FIRST so the old
    // last-match rule would pick the catch-all:
    //   1. a TCL86-scoped spec (tightest),
    //   2. a wider TCL86|TCL90 spec,
    //   3. a catch-all (`surface: None`).
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: Some(SpecSurface::TCL86),
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: Some(surface![SpecSurface::core_in(
            Family::Tcl,
            &[("8.6", Some("9.1"))]
        )]),
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: None,
        ..CommandSpec::DEFAULT
    });

    // Scoped beats catch-all, tighter beats wider — even though the
    // catch-all was registered last.
    let under_86 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert_eq!(under_86.and_then(|s| s.surface), Some(SpecSurface::TCL86));
    // Under 9.0 the tightest visible spec is the two-bit one.
    let under_90 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert_eq!(
        under_90.and_then(|s| s.surface),
        Some(surface![SpecSurface::core_in(
            Family::Tcl,
            &[("8.6", Some("9.1"))]
        )])
    );
    // Where no scoped spec is visible, the catch-all still resolves.
    let under_84 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
    assert!(under_84.is_some_and(|s| s.surface.is_none()));
}

#[test]
fn get_for_dialect_breaks_specificity_ties_by_last_registration() {
    // Curated pack overrides re-register a name at the same scope; the
    // later registration must keep winning (the old `.rev()` guarantee,
    // preserved as the D6 tie-break).
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "d6_tie",
        surface: Some(SpecSurface::TCL86),
        arity: Arity::exact(1), // base data
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_tie",
        surface: Some(SpecSurface::TCL86),
        arity: Arity::exact(2), // curated override
        ..CommandSpec::DEFAULT
    });
    let won = reg
        .get_for_surface("d6_tie", Some(SurfaceQuery::core(Family::Tcl, "8.6")))
        .expect("d6_tie resolves");
    assert_eq!(won.arity, Arity::exact(2), "later registration wins ties");
}

fn reference_best_visible(
    registry: &CommandRegistry,
    specs: &[&'static CommandSpec],
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<&'static CommandSpec> {
    specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| registry.spec_visible(spec, dialect))
        .max_by_key(|&(index, spec)| {
            let nearness = std::cmp::Reverse(
                spec.surface
                    .zip(dialect)
                    .and_then(|(rows, query)| surface_nearness(rows, &query))
                    .unwrap_or(usize::MAX),
            );
            let scope_tightness = std::cmp::Reverse(spec.surface.map_or(u32::MAX, surface_breadth));
            (spec.surface.is_some(), nearness, scope_tightness, index)
        })
        .map(|(_, spec)| *spec)
}

fn synthetic_spec(
    name: &'static str,
    surface: Option<&'static [SpecSurface]>,
    traits: Traits,
) -> &'static CommandSpec {
    Box::leak(Box::new(CommandSpec {
        name,
        traits,
        surface,
        ..CommandSpec::DEFAULT
    }))
}

fn assert_best_visible_matches_reference(
    registry: &CommandRegistry,
    specs: &[&'static CommandSpec],
    dialect: Option<SurfaceQuery<'_>>,
) {
    let expected = reference_best_visible(registry, specs, dialect);
    let actual = registry.best_visible(specs, dialect);
    assert_eq!(
        actual.map(|spec| spec.name),
        expected.map(|spec| spec.name),
        "optimised selector changed the selected name"
    );
    assert_eq!(
        actual.map(std::ptr::from_ref),
        expected.map(std::ptr::from_ref),
        "optimised selector changed the selected spec"
    );
}

#[test]
fn best_visible_matches_reference_for_surface_and_registration_cases() {
    let registry = CommandRegistry::build_default();
    let scoped = synthetic_spec(
        "d6_scoped_singleton",
        Some(SpecSurface::TCL86),
        Traits::empty(),
    );
    let catch_all = synthetic_spec("d6_catch_all_singleton", None, Traits::empty());

    // A singleton scoped row and a singleton catch-all row are both
    // visible exactly as the reference selector expects.
    assert_best_visible_matches_reference(
        &registry,
        &[scoped],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
    assert_best_visible_matches_reference(&registry, &[catch_all], None);

    let narrow = synthetic_spec("d6_narrow", Some(SpecSurface::TCL86), Traits::empty());
    let wide = synthetic_spec("d6_wide", Some(SpecSurface::TCL86_PLUS), Traits::empty());
    let disjoint_old = synthetic_spec("d6_disjoint_old", Some(SpecSurface::TCL85), Traits::empty());
    let disjoint_new = synthetic_spec("d6_disjoint_new", Some(SpecSurface::TCL90), Traits::empty());

    // Overlapping rows choose the narrow window, while disjoint rows
    // produce no result when neither row admits the queried release.
    assert_best_visible_matches_reference(
        &registry,
        &[wide, narrow],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
    assert_best_visible_matches_reference(
        &registry,
        &[disjoint_old, disjoint_new],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );

    let tie_first = synthetic_spec("d6_tie_first", Some(SpecSurface::TCL86), Traits::empty());
    let tie_last = synthetic_spec("d6_tie_last", Some(SpecSurface::TCL86), Traits::empty());
    assert_best_visible_matches_reference(
        &registry,
        &[tie_first, tie_last],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
}

#[test]
fn an_unscoped_pack_override_beats_a_scoped_shipped_spec() {
    let mut registry = CommandRegistry::build_default();
    let shipped = synthetic_spec(
        "d6_pack_override_precedence",
        Some(SpecSurface::TCL86),
        Traits::empty(),
    );
    let override_spec = synthetic_spec("d6_pack_override_precedence", None, Traits::empty());
    registry.insert_pack_origin(
        override_spec,
        crate::pack_origin::PackOrigin {
            pack: "precedence-test".to_owned(),
            content_hash: 1,
            vocabulary_version: "2.0".to_owned(),
            provenance: tcl_dialect::model::Provenance::WorkspaceTrusted,
        },
    );

    let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
    assert_eq!(
        registry
            .best_visible(&[override_spec, shipped], query)
            .map(std::ptr::from_ref),
        Some(std::ptr::from_ref(override_spec)),
        "an installed pack override has provenance precedence over core scope"
    );
    assert_eq!(
        registry
            .best_visible(&[shipped, override_spec], query)
            .map(std::ptr::from_ref),
        Some(std::ptr::from_ref(override_spec)),
        "candidate registration order does not let a scoped core row take precedence"
    );
}

#[test]
fn best_visible_prefers_the_nearer_core_point() {
    use tcl_dialect::model::CorePoints;

    let registry = CommandRegistry::build_default();
    let jim_rows = surface![SpecSurface::core_in(Family::Jim, &[("0.80", None)])];
    let own = synthetic_spec("near_own", Some(jim_rows), Traits::empty());
    let inherited = synthetic_spec("near_inherited", Some(SpecSurface::TCL86), Traits::empty());
    let inherited_wide = synthetic_spec(
        "near_inherited_wide",
        Some(SpecSurface::ALL_TCL),
        Traits::empty(),
    );
    let catch_all = synthetic_spec("near_catch_all", None, Traits::empty());
    let own_first = SurfaceQuery {
        realm: tcl_dialect::model::InvocationRealm::RuleLoader,
        core: CorePoints::two((Family::Jim, None), (Family::Tcl, Some("8.6"))),
        packages: tcl_dialect::model::PackageFloors::Borrowed(&[]),
    };

    // The own-family row wins over an inherited row registered after
    // it and just as narrow, and over a wider one registered before it.
    for specs in [
        [own, inherited],
        [inherited, own],
        [inherited_wide, own],
        [own, inherited_wide],
    ] {
        assert_best_visible_matches_reference(&registry, &specs, Some(own_first));
        assert_eq!(
            registry
                .best_visible(&specs, Some(own_first))
                .map(|spec| spec.name),
            Some("near_own")
        );
    }
    // A scoped row still beats a catch-all, and the inherited row is
    // still what a query without the own-family point selects.
    assert_best_visible_matches_reference(&registry, &[catch_all, inherited], Some(own_first));
    assert_eq!(
        registry
            .best_visible(
                &[own, inherited],
                Some(SurfaceQuery::core(Family::Tcl, "8.6"))
            )
            .map(|spec| spec.name),
        Some("near_inherited")
    );
}

#[test]
fn best_visible_matches_reference_for_package_rows() {
    let registry = CommandRegistry::build_default();
    let package = synthetic_spec("d6_package", Some(SpecSurface::EXPECT), Traits::empty());
    let core = synthetic_spec("d6_core", Some(SpecSurface::ALL_TCL), Traits::empty());
    let packages = [PackageFloor::named("expect")];
    let with_package = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&packages);

    // Package rows are visible through the package set and are narrower
    // than the whole core row, so they win the same reference decision
    // used by the unoptimised implementation.
    assert_best_visible_matches_reference(&registry, &[core, package], Some(with_package));
    assert_eq!(
        registry
            .best_visible(&[core, package], Some(with_package))
            .map(|spec| spec.name),
        Some("d6_package")
    );
    assert_best_visible_matches_reference(
        &registry,
        &[package],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
}

#[test]
fn best_visible_matches_reference_for_profile_operator_visibility() {
    let operator_and_core = surface![
        SpecSurface::core(Family::Tcl),
        SpecSurface::core(Family::F5Irules),
    ];
    let ordinary = synthetic_spec(
        "d6_profile_ordinary",
        Some(operator_and_core),
        Traits::empty(),
    );
    let operator = synthetic_spec(
        "d6_profile_operator",
        Some(operator_and_core),
        Traits::OPERATOR_COMMAND,
    );
    let specs = [ordinary, operator];

    // iRules does not treat math operators as command heads, so the
    // earlier ordinary row must remain selected despite the operator row
    // being registered later. Plain Tcl permits those heads, preserving
    // the later-registration tie-break.
    let irules = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::irules());
    assert_best_visible_matches_reference(irules, &specs, irules.own_surface_query());
    let plain = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert_best_visible_matches_reference(plain, &specs, plain.own_surface_query());
    assert_eq!(
        irules
            .best_visible(&specs, irules.own_surface_query())
            .map(|spec| spec.name),
        Some("d6_profile_ordinary")
    );
    assert_eq!(
        plain
            .best_visible(&specs, plain.own_surface_query())
            .map(|spec| spec.name),
        Some("d6_profile_operator")
    );
}

/// A registry whose pack declared a floor for a package its point carries
/// asks under a query that is not `==` its profile's — and is still that
/// profile's own point, so the profile's operator exclusion applies to it.
#[test]
fn a_floor_on_the_own_query_leaves_it_the_profiles_own_point() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let iapps = tcl_dialect::DialectProfile::find("f5-iapps").expect("catalogue profile");
    assert!(!iapps.operators_as_commands);
    let mut registry = CommandRegistry::build_default().project_for_profile(iapps);
    registry.insert_ambient_package("iapps", "1.0");
    let own = registry.own_surface_query();
    let query = own.expect("a profiled registry has a point");
    assert_eq!(
        query.package("iapps"),
        Some(PackageFloor::at("iapps", "1.0"))
    );
    assert_ne!(query, iapps.surface_query());
    assert!(query.same_point(&iapps.surface_query()));

    let operator = synthetic_spec("floor_probe_operator", None, Traits::OPERATOR_COMMAND);
    let ordinary = synthetic_spec("floor_probe_ordinary", None, Traits::empty());
    assert!(!registry.spec_visible(operator, own));
    assert!(registry.spec_visible(ordinary, own));
    assert!(
        registry.spec_visible(operator, Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        "asked at another point, the exclusion is not this profile's to apply"
    );
}

/// Whether a name is a command in this registry's own dialect is asked at
/// the point the registry's floors make, however late a floor arrives.
#[test]
fn a_command_is_known_here_only_where_the_floored_point_admits_it() {
    // The `tk` environment runs on its 8.6 core and ships the Tk that
    // tracks it: its own Tk 8.6 floor admits a Tk 8.6 command whatever
    // weaker floor is declared, and its core never admits a Tk 9 one.
    const FROM_8_6: &[tcl_dialect::model::SpecWindow] = &[("8.6", None)];
    const FROM_9_0: &[tcl_dialect::model::SpecWindow] = &[("9.0", None)];
    let probes = [
        synthetic_spec(
            "floor_probe_from_86",
            Some(surface![SpecSurface::package_in("Tk", FROM_8_6)]),
            Traits::empty(),
        ),
        synthetic_spec(
            "floor_probe_from_90",
            Some(surface![SpecSurface::package_in("Tk", FROM_9_0)]),
            Traits::empty(),
        ),
    ];
    for floor in [None, Some("8.5"), Some("9.0")] {
        let mut authored = CommandRegistry::build_default();
        for probe in probes {
            authored.insert_static(probe);
        }
        let mut registry = authored.project_for_profile(tcl_dialect::DialectProfile::tk());
        if let Some(floor) = floor {
            registry.insert_ambient_package("Tk", floor);
        }
        assert!(
            registry.has_command_in_this_dialect("floor_probe_from_86"),
            "under a floor of {floor:?}"
        );
        assert!(
            !registry.has_command_in_this_dialect("floor_probe_from_90"),
            "under a floor of {floor:?}"
        );
    }
}

// Versioned codegen-axis stamps: selected at the point a call is resolved
// at, and declined — never guessed — where that point does not settle them.

const HOOK_FROM_9: &[crate::stamp_window::StampWindow<crate::hooks::CodegenHookId>] =
    &[crate::stamp_window::StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: crate::hooks::CodegenHookId::Lassign,
    }];
const INLINE_BEFORE_9: &[crate::stamp_window::StampWindow<crate::hooks::InlineCodegenHookId>] =
    &[crate::stamp_window::StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("9.0"),
        value: crate::hooks::InlineCodegenHookId::Expr,
    }];

fn points() -> [(&'static str, Option<SurfaceQuery<'static>>); 5] {
    [
        ("8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        ("9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
        ("9.1", Some(SurfaceQuery::core(Family::Tcl, "9.1"))),
        (
            "the whole ladder",
            Some(SurfaceQuery::any_release(Family::Tcl)),
        ),
        ("no point", None),
    ]
}

#[test]
fn a_stamp_window_is_selected_at_the_primary_release_and_declines_where_it_is_not_settled() {
    use crate::hooks::{CodegenHookId, InlineCodegenHookId};

    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::probe",
        codegen_hook_windows: HOOK_FROM_9,
        inline_codegen_hook_windows: INLINE_BEFORE_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen, inline) in [
        ("8.6", points()[0].1, None, Some(InlineCodegenHookId::Expr)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign), None),
        ("9.1", points()[2].1, Some(CodegenHookId::Lassign), None),
        ("the whole ladder", points()[3].1, None, None),
        ("no point", points()[4].1, None, None),
    ] {
        let call = registry
            .resolve_call("stamp::probe", &["$l", "a"], query)
            .expect("the probe resolves");
        assert_eq!(call.codegen_hook, codegen, "codegen hook at {point}");
        assert_eq!(call.inline_codegen_hook, inline, "inline hook at {point}");
    }
}

#[test]
fn the_plain_stamp_stands_where_no_window_covers_and_a_window_wins_where_one_does() {
    use crate::hooks::CodegenHookId;

    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::beside",
        codegen_hook: Some(CodegenHookId::Llength),
        codegen_hook_windows: HOOK_FROM_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen) in [
        ("8.6", points()[0].1, Some(CodegenHookId::Llength)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
        ("the whole ladder", points()[3].1, None),
    ] {
        let call = registry
            .resolve_call("stamp::beside", &["$l"], query)
            .expect("the probe resolves");
        assert_eq!(call.codegen_hook, codegen, "{point}");
    }
}

/// A subcommand whose windows state nothing at the release is the command's
/// own call; one whose windows the point does not settle is dispatched
/// plain, and the command's hook is not the answer in its place.
#[test]
fn a_subcommand_inherits_where_its_windows_are_silent_and_does_not_where_they_decline() {
    use crate::hooks::CodegenHookId;

    let sub = SubCommand {
        name: "get",
        codegen_hook_windows: HOOK_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::ensemble",
        codegen_hook: Some(CodegenHookId::Dict),
        subcommands: Box::leak(Box::new([sub])),
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen) in [
        ("8.6", points()[0].1, Some(CodegenHookId::Dict)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
        ("the whole ladder", points()[3].1, None),
        ("no point", points()[4].1, None),
    ] {
        let call = registry
            .resolve_call("stamp::ensemble", &["get", "$d", "k"], query)
            .expect("the ensemble resolves");
        assert_eq!(call.codegen_hook, codegen, "{point}");
        // The command's own call is not the subcommand's to decline.
        let own = registry
            .resolve_call("stamp::ensemble", &["$d"], query)
            .expect("the command resolves");
        assert_eq!(own.codegen_hook, Some(CodegenHookId::Dict), "{point}");
    }
}

#[test]
fn a_semantic_operation_window_decides_the_operation_a_resolved_invocation_has() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::length",
        semantic_operation_windows: LENGTH_FROM_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, operation) in [
        ("8.6", points()[0].1, SemanticOperationId::Invoke),
        (
            "9.0",
            points()[1].1,
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        ),
        (
            "the whole ladder",
            points()[3].1,
            SemanticOperationId::Invoke,
        ),
        ("no point", points()[4].1, SemanticOperationId::Invoke),
    ] {
        let invocation = registry
            .resolve_invocation("stamp::length", &["abc"], query)
            .expect("the probe resolves");
        assert_eq!(invocation.semantics.operation, operation, "{point}");
    }
}

/// A subcommand whose operation windows state nothing at the release is the
/// command's own operation; one whose windows the point does not settle is
/// a plain invoke, and the command's operation is not the answer instead.
#[test]
fn a_subcommand_operation_inherits_where_silent_and_is_plain_where_it_declines() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
    let size = SubCommand {
        name: "size",
        semantic_operation_windows: LENGTH_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::inherits",
        semantic_operation: Some(concat),
        subcommands: Box::leak(Box::new([size])),
        ..CommandSpec::DEFAULT
    });
    for (point, query, operation) in [
        ("8.6", points()[0].1, concat),
        (
            "9.0",
            points()[1].1,
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        ),
        (
            "the whole ladder",
            points()[3].1,
            SemanticOperationId::Invoke,
        ),
        ("no point", points()[4].1, SemanticOperationId::Invoke),
    ] {
        let invocation = registry
            .resolve_invocation("stamp::inherits", &["size", "abc"], query)
            .expect("the ensemble resolves");
        assert_eq!(invocation.semantics.operation, operation, "{point}");
    }
}

/// The operation a codegen or inline hook names is read through the same
/// selection: a window gives it where it covers the release, and a level
/// that declines gives plain dispatch, not its command's operation.
#[test]
fn the_operation_a_windowed_hook_names_is_selected_and_declined_like_the_hook() {
    use crate::hooks::{CodegenHookId, InlineCodegenHookId};
    use crate::intrinsic::IntrinsicId;
    use crate::lifecycle::Lifecycle;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LLENGTH_FROM_9: &[StampWindow<CodegenHookId>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: CodegenHookId::Llength,
    }];
    const LINDEX_FROM_9: &[StampWindow<InlineCodegenHookId>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: InlineCodegenHookId::Lindex,
    }];
    let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
    let by_codegen = SubCommand {
        name: "count",
        codegen_hook_windows: LLENGTH_FROM_9,
        ..SubCommand::DEFAULT
    };
    let by_inline = SubCommand {
        name: "at",
        inline_codegen_hook_windows: LINDEX_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::hooked",
        semantic_operation: Some(concat),
        subcommands: Box::leak(Box::new([by_codegen, by_inline])),
        ..CommandSpec::DEFAULT
    });
    let operation = |sub: &str, query| {
        registry
            .resolve_invocation("stamp::hooked", &[sub, "abc"], query)
            .expect("the ensemble resolves")
            .semantics
            .operation
    };
    for (sub, at_9) in [
        (
            "count",
            SemanticOperationId::Intrinsic(IntrinsicId::ListLength),
        ),
        ("at", SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)),
    ] {
        assert_eq!(
            operation(sub, points()[0].1),
            concat,
            "{sub} at 8.6 inherits"
        );
        assert_eq!(operation(sub, points()[1].1), at_9, "{sub} at 9.0");
        for (point, query) in [
            ("the whole ladder", points()[3].1),
            ("no point", points()[4].1),
        ] {
            assert_eq!(
                operation(sub, query),
                SemanticOperationId::Invoke,
                "{sub} at {point} declines"
            );
        }
    }
}

#[test]
fn a_native_lowering_window_is_read_at_the_point_and_declines_across_its_edge() {
    use crate::completion::CompletionCode;
    use crate::lifecycle::Lifecycle;
    use crate::native_lowering::NativeLowering;
    use crate::stamp_window::StampWindow;

    const BREAK_FROM_9: &[StampWindow<NativeLowering>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: NativeLowering::Completion(CompletionCode::Break),
    }];
    let spec = CommandSpec {
        name: "stamp::native",
        native_lowering_windows: BREAK_FROM_9,
        ..CommandSpec::DEFAULT
    };
    let shapes = points().map(|(_, query)| spec.native_lowering_at(query.as_ref()));
    assert_eq!(
        shapes,
        [
            NativeLowering::Generic,
            NativeLowering::Completion(CompletionCode::Break),
            NativeLowering::Completion(CompletionCode::Break),
            NativeLowering::Generic,
            NativeLowering::Generic,
        ]
    );
    assert_eq!(
        spec.native_lowering(),
        NativeLowering::Generic,
        "the plain accessor reads the plain field alone"
    );
}

#[test]
fn native_lowering_descriptor_metadata_keeps_exact_keys_and_supplied_stamp_windows() {
    use crate::completion::CompletionCode;
    use crate::lifecycle::Lifecycle;
    use crate::native_lowering::NativeLowering;
    use crate::stamp_window::StampWindow;

    const BREAK_FROM_9: &[StampWindow<NativeLowering>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: NativeLowering::Completion(CompletionCode::Break),
    }];
    let selected = CommandSpec {
        name: "stamp::native",
        native_lowering_windows: BREAK_FROM_9,
        ..CommandSpec::DEFAULT
    };
    let rooted = CommandSpec {
        name: "::stamp::native",
        native_lowering: Some(NativeLowering::Completion(CompletionCode::Continue)),
        ..CommandSpec::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(selected.clone());
    assert_eq!(
        registry.native_lowering_for_descriptor(rooted.descriptor_key(), None),
        None,
        "an exact descriptor cannot borrow the unrooted row",
    );
    registry.insert(rooted.clone());
    for (_, query) in points() {
        assert_eq!(
            registry.native_lowering_for_descriptor(selected.descriptor_key(), query.as_ref()),
            Some(selected.native_lowering_at(query.as_ref())),
        );
        assert_eq!(
            registry.native_lowering_for_descriptor(rooted.descriptor_key(), query.as_ref()),
            Some(rooted.native_lowering_at(query.as_ref())),
        );
    }
}

/// Every operation a windowed stamp could give a command counts when the
/// question is surface-blind: which names could be this operation.
#[test]
fn a_windowed_operation_is_counted_by_the_questions_that_ask_no_release() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let spec = CommandSpec {
        name: "stamp::length",
        semantic_operation_windows: LENGTH_FROM_9,
        ..CommandSpec::DEFAULT
    };
    assert!(spec.intrinsic_ids().contains(&IntrinsicId::StringLength));
    let mut registry = CommandRegistry::build_default();
    registry.insert(spec);
    assert!(
        registry
            .command_names_for_semantic_operation(SemanticOperationId::Intrinsic(
                IntrinsicId::StringLength
            ))
            .any(|name| name == "stamp::length")
    );
    assert!(
        !CommandSpec {
            name: "stamp::plain",
            ..CommandSpec::DEFAULT
        }
        .intrinsic_ids()
        .contains(&IntrinsicId::StringLength)
    );
}

#[test]
fn profile_projection_keeps_surface_less_authored_override_of_core_name() {
    let mut registry = CommandRegistry::build_default();
    let mut override_spec = registry.get("set").expect("core set spec").clone();
    override_spec.surface = None;
    override_spec.arity = Arity::exact(17);
    registry.insert(override_spec);

    let profile = crate::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let projected = registry.project_for_profile(profile);
    assert_eq!(
        projected.get("set").map(|spec| spec.arity),
        Some(Arity::exact(17)),
        "the authored catch-all is the effective binding, not Tcl's scoped row"
    );
    assert!(
        projected.get("throw").is_none(),
        "preserving the overlay must not leak Tcl 8.6+ shipped commands into Tcl 8.4"
    );
}

#[test]
fn instance_method_walks_superclasses_breadth_first() {
    use crate::spec::{ObjectClassSpec, SubCommand};

    // Diamond hierarchy: Diamond → {Left, Right} → Base. Both Left and
    // Right (and Base) define `m`; a breadth-first walk in declaration
    // order must find Left's `m` first, whereas a `Vec::pop` LIFO walk
    // would reverse the siblings and return Right's.
    static M_LEFT: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "left",
        ..SubCommand::DEFAULT
    }];
    static M_RIGHT: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "right",
        ..SubCommand::DEFAULT
    }];
    static M_BASE: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "base",
        ..SubCommand::DEFAULT
    }];
    static DIAMOND: ObjectClassSpec = ObjectClassSpec {
        class_name: "Diamond",
        instance_methods: &[],
        superclasses: &["Left", "Right"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static LEFT: ObjectClassSpec = ObjectClassSpec {
        class_name: "Left",
        instance_methods: &M_LEFT,
        superclasses: &["Base"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static RIGHT: ObjectClassSpec = ObjectClassSpec {
        class_name: "Right",
        instance_methods: &M_RIGHT,
        superclasses: &["Base"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static BASE: ObjectClassSpec = ObjectClassSpec {
        class_name: "Base",
        instance_methods: &M_BASE,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };

    let mut reg = CommandRegistry::build_default();
    for oc in [&DIAMOND, &LEFT, &RIGHT, &BASE] {
        reg.insert(CommandSpec {
            name: oc.class_name,
            object_class: Some(oc),
            ..CommandSpec::DEFAULT
        });
    }

    let resolved = reg
        .instance_method("Diamond", "m")
        .expect("method resolves");
    assert_eq!(
        resolved.detail, "left",
        "breadth-first, declaration-ordered walk visits Left before Right"
    );
}

#[test]
fn instance_method_at_honours_explicit_package_and_dialect_gates() {
    use crate::spec::ObjectClassSpec;

    static METHODS: [SubCommand; 1] = [SubCommand {
        name: "later",
        surface: Some(SpecSurface::TCL91),
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..SubCommand::DEFAULT
    }];
    static CLASS: ObjectClassSpec = ObjectClassSpec {
        class_name: "VersionedClass",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: CLASS.class_name,
        required_package: Some("Demo"),
        object_class: Some(&CLASS),
        ..CommandSpec::DEFAULT
    });

    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.0"),
            Some(SurfaceQuery::core(Family::Tcl, "9.1"))
        )
        .is_none(),
        "the explicit owning-package floor rejects a future method"
    );
    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.1"),
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none(),
        "the method's own dialect gate remains independent of lifecycle"
    );
    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.1"),
            Some(SurfaceQuery::core(Family::Tcl, "9.1"))
        )
        .is_some(),
        "both registry-declared availability axes admit the method"
    );
}

#[test]
fn build_default_has_commands() {
    let reg = CommandRegistry::build_default();
    assert!(!reg.is_empty());
    assert!(reg.get("for").is_some());
    assert!(reg.get("set").is_some());
    assert!(reg.get("nonexistent_command").is_none());
}

#[test]
fn leading_zero_is_octal_tracks_tcl_version() {
    // Plain default registry (no Tcl release row) defaults to octal.
    assert!(CommandRegistry::build_default().leading_zero_is_octal());
    // tcl9.0 (TIP 114) reads leading zeros as decimal; everything else
    // (8.4/8.5/8.6 and the 8.x-derived F5 dialects) stays octal.
    let octal_cases = [
        SurfaceLayer::Core(Family::Tcl, "8.4"),
        SurfaceLayer::Core(Family::Tcl, "8.5"),
        SurfaceLayer::Core(Family::Tcl, "8.6"),
        SurfaceLayer::Core(Family::F5Irules, ""),
        SurfaceLayer::Package("iapps"),
    ];
    for layer in octal_cases {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(layer);
        assert!(reg.leading_zero_is_octal(), "{layer:?} should be octal");
    }
    let mut reg90 = CommandRegistry::build_default();
    reg90.load_surface(SurfaceLayer::Core(Family::Tcl, "9.0"));
    assert!(!reg90.leading_zero_is_octal(), "tcl9.0 should be decimal");
    // tcl9.1 keeps the TIP 114 decimal rule; a tcl9.1-only
    // registry (loads TCL91, not TCL90) must still read leading zeros as
    // decimal.
    let mut reg91 = CommandRegistry::build_default();
    reg91.load_surface(SurfaceLayer::Core(Family::Tcl, "9.1"));
    assert!(!reg91.leading_zero_is_octal(), "tcl9.1 should be decimal");
}

#[test]
fn irules_command_legality_matrix() {
    use crate::events::EventRegistry;
    use crate::profiles::ProfileRegistry;
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    // HTTP::respond is satisfied in HTTP_REQUEST (HTTP profile implied) but
    // not in the L4 CLIENT_ACCEPTED event.
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "HTTP_REQUEST",
        &events,
        &profiles
    ));
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "CLIENT_ACCEPTED",
        &events,
        &profiles
    ));
    // An unknown event is illegal for every command.
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "NOT_AN_EVENT",
        &events,
        &profiles
    ));
    // HA::status is explicitly excluded from RULE_INIT.
    assert!(!reg.is_irules_command_legal_in_event("HA::status", "RULE_INIT", &events, &profiles));

    // The inverse list is sorted and reflects the same matrix.
    let evs = reg.irules_events_for_command("HTTP::respond", &events, &profiles);
    assert!(evs.contains(&"HTTP_REQUEST"));
    assert!(!evs.contains(&"CLIENT_ACCEPTED"));
    assert!(
        evs.windows(2).all(|w| w[0] <= w[1]),
        "events must be sorted"
    );
}

#[test]
fn get_resolves_global_qualifier_to_builtin() {
    let reg = CommandRegistry::build_default();
    assert!(reg.get("::foreach").is_some());
    assert_eq!(
        reg.get("::for").map(|s| s.name),
        reg.get("for").map(|s| s.name)
    );
    assert!(reg.get("::nonexistent_command").is_none());
}

#[test]
fn switch_names_is_dialect_filtered() {
    let reg = CommandRegistry::build_default();
    let regsub = reg.get("regsub").expect("regsub spec");
    // `-command` is Tcl 9.0+ (TIP 463); the always-available
    // switches appear in every dialect.
    let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(in_86.contains(&"-all"), "{in_86:?}");
    assert!(in_86.contains(&"-nocase"), "{in_86:?}");
    assert!(
        !in_86.contains(&"-command"),
        "9.0-only -command leaked into 8.6: {in_86:?}",
    );
    let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert!(
        in_90.contains(&"-command"),
        "-command missing under 9.0: {in_90:?}",
    );
    // No filter → every declared option, no duplicates.
    let all = regsub.switch_names(None);
    assert!(all.contains(&"-command"));
    let mut dedup = all.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(dedup.len(), all.len(), "switch_names returned duplicates");
}

#[test]
fn tcl9_commands_from_pr_433_are_registered() {
    let reg = CommandRegistry::build_default();
    for name in [
        "foreachLine",
        "readFile",
        "writeFile",
        "lpop",
        "const",
        "tcl::idna",
        "::tcl::idna",
        "tcl::process",
        "::tcl::process",
    ] {
        assert!(
            reg.get(name).is_some(),
            "{name} not registered after SYNC-MAY19-tcl9-commands",
        );
    }
}

#[test]
fn coroinject_coroprobe_registered() {
    // Verify these two commands remain registered and visible to
    // the LSP.
    let reg = CommandRegistry::build_default();
    assert!(reg.get("coroinject").is_some());
    assert!(reg.get("coroprobe").is_some());
}

#[test]
fn tcl9_commands_gated_to_tcl90() {
    let reg = CommandRegistry::build_default();
    // `const` (Tcl 9.0, TIP 677) joins the list, gated `TCL90_PLUS`: it
    // does not exist in iRules' embedded Tcl 8.4.6, so it is (correctly)
    // neither pre-9.0 nor iRules-visible.
    for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
        let spec = reg.get(name).expect("registered");
        // A 9.0 addition is available in 9.0 *and* 9.1 (a `.1` release is
        // additive — verified against C Tcl 9.1b0 doc/*.n), so it is gated
        // `TCL90_PLUS`, not `TCL90`-only.
        assert_eq!(
            spec.surface,
            Some(SpecSurface::TCL90_PLUS),
            "{name} should be Tcl 9.0+",
        );
        assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
        assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
        assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
        assert!(!spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))));
    }
}

#[test]
fn every_command_has_hover_with_manpage_source() {
    // Every registered command must carry a hover snippet with a non-empty
    // summary and a manpage/source attribution. A short allowlist covers
    // internal pseudo-commands and dialect placeholders that have no user
    // documentation.
    const HOVERLESS_OK: &[&str] = &["disabled_in_irules"];
    let reg = CommandRegistry::build_default();
    let mut missing_hover = Vec::new();
    let mut missing_source = Vec::new();
    for name in reg.command_names() {
        if HOVERLESS_OK.contains(&name) {
            continue;
        }
        let spec = reg.get(name).expect("registered");
        match &spec.hover {
            None => missing_hover.push(name.to_string()),
            Some(h) => {
                if h.summary.trim().is_empty() || h.source.trim().is_empty() {
                    missing_source.push(name.to_string());
                }
            }
        }
    }
    missing_hover.sort();
    missing_source.sort();
    assert!(
        missing_hover.is_empty(),
        "commands without a hover snippet: {missing_hover:?}",
    );
    assert!(
        missing_source.is_empty(),
        "commands with an empty hover summary or manpage source: {missing_source:?}",
    );
}

#[test]
fn timerate_registered_with_body_and_int_hint() {
    // `timerate` measures the rate of execution of a script.
    use crate::arg_role::ArgRole;
    use crate::side_effects::SideEffectTarget;
    use crate::types::TclType;
    let reg = CommandRegistry::build_default();
    let spec = reg.get("timerate").expect("timerate registered");
    assert_eq!(spec.name, "timerate");
    // BODY role on arg 0, INT type hint (shimmers) on arg 1.
    assert_eq!(spec.arg_role_at(0), Some(ArgRole::Body));
    assert_eq!(
        spec.arg_types
            .iter()
            .find(|(i, _)| *i == 1)
            .map(|(_, h)| h.expected),
        Some(Some(TclType::Int)),
    );
    // Unbounded arity: at least the command word, no upper bound.
    assert!(!spec.arity.accepts(0));
    assert!(spec.arity.accepts(1));
    assert!(spec.arity.accepts(6));
    assert_eq!(spec.return_type, Some(TclType::String));
    // The body runs arbitrary code → an UNKNOWN-target read+write effect.
    assert!(
        spec.side_effects
            .lower_bound()
            .iter()
            .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
        "timerate should declare an UNKNOWN read+write side effect",
    );
}

#[test]
fn lookup_for_command() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get("for").unwrap();
    assert_eq!(spec.name, "for");
    assert!(spec.traits.contains(Traits::CONTROL_FLOW));
    assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
    assert_eq!(spec.arity, crate::arity::Arity::exact(4));
}

#[test]
fn arg_roles_for_static_command() {
    let reg = CommandRegistry::build_default();
    let bodies = reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
    assert!(bodies.contains(&0)); // init
    assert!(bodies.contains(&2)); // next
    assert!(bodies.contains(&3)); // body
    assert!(!bodies.contains(&1)); // cond is Expr, not Body
}

#[test]
fn arg_roles_for_expr() {
    let reg = CommandRegistry::build_default();
    let exprs = reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
    assert_eq!(exprs, vec![1]); // only the condition
}

#[test]
fn uplevel_body_arg_role_skips_optional_level() {
    // `uplevel ?level? {body}` — the body word's index depends
    // on whether a leading `level` word is present. The registry resolver
    // is the single source of truth every body consumer (semantic tokens,
    // green-tree descent, SSA) queries.
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    // Literal relative level → body at 1.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["1", "{set x 1}"], ArgRole::Body),
        vec![1]
    );
    // Absolute `#0` level → body at 1.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["#0", "{set x 1}"], ArgRole::Body),
        vec![1]
    );
    // No level → body at 0.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["{set x 1}"], ArgRole::Body),
        vec![0]
    );
    // The literal bytes `$lvl` are a script word, not a supplied level.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["$lvl", "{set x 1}"], ArgRole::Body),
        vec![0]
    );
    // A lone dynamic word is the body itself (implicit level 1) → body at 0.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["$body"], ArgRole::Body),
        vec![0]
    );
    // Bodyless `uplevel 1` (a wrong-#args error) exposes no body word — the
    // literal level must not be mis-tagged as a script.
    assert_eq!(
        reg.arg_indices_for_role("uplevel", &["1"], ArgRole::Body)
            .len(),
        0
    );
}

#[test]
fn if_marks_structural_keywords() {
    let reg = CommandRegistry::build_default();
    let args = [
        "1", "then", "{a}", "elseif", "2", "then", "{b}", "else", "{c}",
    ];
    let kw = reg.arg_indices_for_role("if", &args, ArgRole::Keyword);
    // then@1, elseif@3, then@5, else@7
    assert_eq!(kw, vec![1, 3, 5, 7], "{kw:?}");
    // The bodies and exprs still resolve too.
    let bodies = reg.arg_indices_for_role("if", &args, ArgRole::Body);
    assert!(bodies.contains(&2) && bodies.contains(&6) && bodies.contains(&8));
}

#[test]
fn try_marks_structural_keywords() {
    let reg = CommandRegistry::build_default();
    let args = ["{body}", "on", "error", "{e}", "{h}", "finally", "{f}"];
    let kw = reg.arg_indices_for_role("try", &args, ArgRole::Keyword);
    // on@1, finally@5
    assert_eq!(kw, vec![1, 5], "{kw:?}");
}

/// The format families answer from registry data alone:
/// the *position* from the `FormatString` / `ScanFormat` roles, the
/// *family* from `format_string_type`.
#[test]
fn format_string_args_cover_every_family() {
    use crate::patterns::FormatType;
    let reg = crate::model::ingress::static_context_for("tcl9.1").commands();
    let found = |name: &str, args: &[&str]| -> Vec<(usize, FormatType, bool)> {
        reg.format_string_args(name, args)
            .into_iter()
            .map(|f| (f.index, f.kind, f.scan))
            .collect()
    };
    // TP — a fixed argument index.
    assert_eq!(
        found("format", &["%d", "7"]),
        vec![(0, FormatType::Sprintf, false)]
    );
    // TP — a resolver-computed index, scan direction.
    assert_eq!(
        found("scan", &["$s", "%d", "v"]),
        vec![(1, FormatType::Sprintf, true)]
    );
    // TP — subcommand-relative indices.
    assert_eq!(
        found("binary", &["format", "c3", "$l"]),
        vec![(1, FormatType::Binary, false)]
    );
    assert_eq!(
        found("binary", &["scan", "$v", "c3", "out"]),
        vec![(2, FormatType::Binary, true)]
    );
    // TP — an *option value* position, both directions.
    assert_eq!(
        found("clock", &["format", "$t", "-format", "%Y"]),
        vec![(3, FormatType::Clock, false)]
    );
    assert_eq!(
        found("clock", &["scan", "$s", "-format", "%Y"]),
        vec![(3, FormatType::Clock, true)]
    );
    // TP — `regsub`'s replacement template, shifted past its switches.
    assert_eq!(
        found("regsub", &["-all", "--", "e", "$s", "X", "out"]),
        vec![(4, FormatType::Regsub, false)]
    );
    // FN guard — the explicitly global spellings resolve identically.
    assert_eq!(
        found("::format", &["%d", "7"]),
        found("format", &["%d", "7"])
    );
    assert_eq!(
        found("::clock", &["format", "$t", "-format", "%Y"]),
        found("clock", &["format", "$t", "-format", "%Y"])
    );
    // TN — a subcommand with no format string, and an unknown command.
    assert_eq!(found("binary", &["encode", "hex", "$d"]).len(), 0);
    assert_eq!(found("clock", &["seconds"]).len(), 0);
    assert_eq!(found("no::such::command", &["%d"]).len(), 0);
    assert_eq!(found("puts", &["%d"]).len(), 0);
    // TN — `regsub -command` makes that position a callback, not a
    // template, so it declares no replacement word.
    assert!(
        !found("regsub", &["-command", "--", "e", "$s", "cb"])
            .iter()
            .any(|(i, _, _)| *i == 3)
    );
}

#[test]
fn source_aware_format_layouts_abstain_only_while_option_selection_is_dynamic() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::FormatType;

    let dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    let fixed_start_value = [
        InvocationWord::Literal("-start"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        assert!(
            registry
                .format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&dynamic_prefix),
                )
                .is_empty(),
            "{dialect}: $mode could shift regsub's replacement position"
        );
        assert_eq!(
            registry.format_string_args_words(
                "regsub",
                InvocationArguments::structured(&fixed_start_value),
            ),
            vec![FormatStringArg {
                index: 4,
                kind: FormatType::Regsub,
                scan: false,
            }],
            "{dialect}: a known -start still consumes exactly its dynamic value"
        );
    }

    let invalid_command_abbrev = [
        InvocationWord::Literal("-c"),
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    let exact_command = [
        InvocationWord::Literal("-command"),
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("callback"),
    ];
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert!(
        registry
            .format_string_args_words(
                "regsub",
                InvocationArguments::structured(&invalid_command_abbrev),
            )
            .is_empty(),
        "-c is not a Tcl regsub abbreviation and cannot claim a replacement template"
    );
    assert!(
        registry
            .format_string_args_words("regsub", InvocationArguments::structured(&exact_command))
            .is_empty(),
        "only exact Tcl 9 -command selects a callback rather than a replacement template"
    );
}

#[test]
fn clock_format_keeps_fixed_payload_and_option_selection_separate() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::FormatType;
    let clock = [
        InvocationWord::Literal("format"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("-format"),
        InvocationWord::Literal("%Y"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .format_string_args_words("clock", InvocationArguments::structured(&clock),),
        vec![FormatStringArg {
            index: 3,
            kind: FormatType::Clock,
            scan: false,
        }],
        "a fixed positional time value cannot hide an after-positional option value"
    );
    let unknown_clock_option = [
        InvocationWord::Literal("format"),
        InvocationWord::Dynamic,
        InvocationWord::Dynamic,
        InvocationWord::Literal("%Y"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .format_string_args_words(
                "clock",
                InvocationArguments::structured(&unknown_clock_option),
            )
            .is_empty(),
        "an unknown option word does not establish the format operand"
    );
}

#[test]
fn source_aware_pattern_layouts_abstain_for_dynamic_leading_options() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    let dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a+}"),
        InvocationWord::Dynamic,
    ];
    for name in ["regexp", "regsub"] {
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(name, InvocationArguments::structured(&dynamic_prefix))
                .is_empty(),
            "{name}: $mode could be a leading option"
        );
    }
    let glob_dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("/tmp"),
        InvocationWord::Literal("*.tcl"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "glob",
                InvocationArguments::structured(&glob_dynamic_prefix)
            )
            .is_empty(),
        "glob $mode /tmp *.tcl cannot choose a pattern tail before $mode resolves"
    );
    let string_dynamic_prefix = [
        InvocationWord::Literal("match"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("a*"),
        InvocationWord::Literal("abc"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "string",
                InvocationArguments::structured(&string_dynamic_prefix),
            )
            .is_empty(),
        "string match $mode a* abc cannot treat $mode as the pattern"
    );
}

#[test]
fn source_aware_pattern_layouts_accept_proven_options_and_terminators() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    let regexp_start = [
        InvocationWord::Literal("-start"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("a+"),
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words("regexp", InvocationArguments::structured(&regexp_start)),
        vec![PatternArg {
            index: 2,
            kind: PatternType::Regex,
        }],
        "a fixed value-taking option may safely carry a dynamic value"
    );
    let non_option_template = [InvocationWord::DynamicNonOption, InvocationWord::Dynamic];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "regexp",
                InvocationArguments::structured(&non_option_template),
            ),
        vec![PatternArg {
            index: 0,
            kind: PatternType::Regex,
        }],
        "a template with a direct non-dash prefix remains a positional pattern"
    );
    let glob_terminator = [
        InvocationWord::Literal("--"),
        InvocationWord::Literal("-literal"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words("glob", InvocationArguments::structured(&glob_terminator)),
        vec![PatternArg {
            index: 1,
            kind: PatternType::Glob,
        }],
        "-- freezes glob's option prefix before an option-shaped pattern"
    );
}

#[test]
fn source_aware_pattern_layouts_respect_release_and_reserved_suffixes() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    let strict_abbreviation = [
        InvocationWord::Literal("-sta"),
        InvocationWord::Literal("2"),
        InvocationWord::Literal("a+"),
        InvocationWord::Literal("aaa"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "regexp",
                InvocationArguments::structured(&strict_abbreviation)
            )
            .is_empty(),
        "regexp's strict option table rejects a non-exact abbreviation"
    );
    let stride_abbreviation = [
        InvocationWord::Literal("-str"),
        InvocationWord::Literal("2"),
        InvocationWord::Literal("{a b}"),
        InvocationWord::Literal("a+"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl8.6")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&stride_abbreviation)
            )
            .is_empty(),
        "a Tcl 9-only option abbreviation is invalid before Tcl 9"
    );
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&stride_abbreviation),
            ),
        vec![PatternArg {
            index: 3,
            kind: PatternType::Glob,
        }],
        "the same abbreviation follows Tcl 9's profile-filtered option table"
    );
    let option_shaped_suffix = [
        InvocationWord::Literal("-regexp"),
        InvocationWord::Literal("--"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&option_shaped_suffix),
            ),
        vec![PatternArg {
            index: 1,
            kind: PatternType::Glob,
        }],
        "lsearch's reserved list/pattern suffix remains positional"
    );
}

const MUTATED_PATTERN_OPTIONS: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
    name: "-skip",
    value: crate::hover::OptionValue::value("count"),
    detail: "fixture value",
    surface: None,
    aliases: &[],
    lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];
fn mutated_pattern_literal_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let first = crate::spec::leading_option_word_count_with(
        MUTATED_PATTERN_OPTIONS,
        args,
        PrefixMatching::Strict,
    );
    [
        (first, ArgRole::Pattern),
        (first + 2, ArgRole::FormatString),
    ]
    .into_iter()
    .filter_map(|(index, role)| {
        (index < args.len())
            .then(|| u8::try_from(index).ok().map(|index| (index, role)))
            .flatten()
    })
    .collect()
}
fn mutated_pattern_layout_roles(
    args: crate::invocation_words::InvocationArguments<'_>,
    options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let first = options.leading_word_count(args)?;
    let count = args.exact_argv_len()?;
    Some(
        [
            (first, ArgRole::Pattern),
            (first + 2, ArgRole::FormatString),
        ]
        .into_iter()
        .filter_map(|(index, role)| {
            (index < count)
                .then(|| u8::try_from(index).ok().map(|index| (index, role)))
                .flatten()
        })
        .collect(),
    )
}

/// The source proof follows descriptors installed in the registry, not a
/// closed list of Tcl command names. A pack or a later registry mutation
/// can introduce the same option-dependent pattern/replacement grammar
/// and all generic consumers receive the new answer unchanged.
#[test]
fn source_aware_pattern_and_format_queries_follow_mutated_descriptors() {
    const SPEC: CommandSpec = CommandSpec {
        name: "mutated-pattern-format-owner",
        options: MUTATED_PATTERN_OPTIONS,
        prefix_matching: PrefixMatching::Strict,
        arg_role_layout_resolver: Some(mutated_pattern_layout_roles),
        pattern_type: Some(PatternType::Regex),
        format_string_type: Some(FormatType::Regsub),
        ..CommandSpec::DEFAULT
    };
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{FormatType, PatternArg, PatternType};

    assert_eq!(
        mutated_pattern_literal_roles(&["-skip", "1", "a", "s", "replacement"]),
        vec![(2, ArgRole::Pattern), (4, ArgRole::FormatString)]
    );

    let mut registry = CommandRegistry::build_default();
    registry.insert(SPEC);
    let dynamic = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    assert_eq!(
        registry
            .pattern_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&dynamic),
            )
            .len(),
        0
    );
    assert_eq!(
        registry
            .format_string_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&dynamic),
            )
            .len(),
        0
    );

    let fixed = [
        InvocationWord::Literal("-skip"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    assert_eq!(
        registry.pattern_args_words(
            "mutated-pattern-format-owner",
            InvocationArguments::structured(&fixed),
        ),
        vec![PatternArg {
            index: 2,
            kind: PatternType::Regex,
        }]
    );
    assert_eq!(
        registry.format_string_args_words(
            "mutated-pattern-format-owner",
            InvocationArguments::structured(&fixed),
        ),
        vec![FormatStringArg {
            index: 4,
            kind: FormatType::Regsub,
            scan: false,
        }]
    );
}

/// Pattern locations come from the owning specs' option descriptors, not
/// from an LSP walk guessing where a `-start` value ends.  Exercise the
/// three shapes prone to diverging: abbreviations, value-taking
/// options, and glob's unbounded pattern tail.
#[test]
fn pattern_roles_follow_declared_option_prefixes() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role("lsearch", &["-sta", "2", "{a b c}", "b*"], ArgRole::Pattern,),
        vec![3],
    );
    assert_eq!(
        reg.arg_indices_for_role("string", &["match", "-noc", "a*", "abc"], ArgRole::Pattern,),
        vec![2],
    );
    assert_eq!(
        reg.arg_indices_for_role("glob", &["-di", "tmp", "*.tcl", "*.tm"], ArgRole::Pattern,),
        vec![2, 3],
    );
    assert_eq!(
        reg.arg_indices_for_role("glob", &["--", "-literal", "*.tcl"], ArgRole::Pattern),
        vec![1, 2],
    );
    assert_eq!(
        reg.arg_indices_for_role("regexp", &["-start", "2", "a+", "aaa"], ArgRole::Pattern,),
        vec![2],
    );
    assert_eq!(
        reg.pattern_args("lsearch", &["-regexp", "{a b}", "a+"]),
        vec![crate::patterns::PatternArg {
            index: 2,
            kind: crate::patterns::PatternType::Regex,
        }],
    );
    assert_eq!(
        reg.pattern_args("lsearch", &["-exact", "{a b}", "a+"])
            .len(),
        0
    );
}

#[test]
fn source_aware_lsearch_pattern_layout_respects_outer_option_boundary() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    // Tcl's `Tcl_LsearchObjCmd` has used `for (i = 1; i < objc - 2;
    // i++)` since 8.4: it stops scanning exactly when list and pattern
    // remain. Keep the source-aware resolver aligned across every
    // supported core release, including the 9.0 -stride expansion.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let proven_outer_option = [
            InvocationWord::Literal("-glob"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("ba*"),
        ];
        assert_eq!(
            reg.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&proven_outer_option),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Glob,
            }],
            "{dialect}: lsearch -glob $list {{ba*}} has proven operands and glob mode"
        );

        let expanded_list = [
            InvocationWord::Literal("-glob"),
            InvocationWord::Expanded,
            InvocationWord::Literal("ba*"),
        ];
        assert!(
            reg.pattern_args_words("lsearch", InvocationArguments::structured(&expanded_list))
                .is_empty(),
            "{dialect}: lsearch -glob {{*}}$list {{ba*}} has no known argv suffix"
        );

        let ambiguous_option_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        // `$mode` remains inside the `i < objc - 2` option region and
        // may become a value-taking switch, so it cannot be reclassified
        // as the list operand to claim a glob pattern.
        assert!(
            reg.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&ambiguous_option_prefix),
            )
            .is_empty(),
            "{dialect}: lsearch $mode {{a b}} {{a+}} remains ambiguous"
        );
    }
}

#[test]
fn lsearch_pattern_resolver_uses_profiled_options_and_the_reserved_suffix() {
    use crate::patterns::{PatternArg, PatternType};

    // These are oracle-shaped against Tcl's `i < objc - 2` scanner.  The
    // first two words of the short form are list + pattern, even though
    // both spell like lsearch switches.
    let option_shaped_operands = ["-regexp", "-glob"];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let patterns = registry.pattern_args("lsearch", &option_shaped_operands);
        assert_eq!(
            patterns,
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "{dialect}: lsearch -regexp -glob keeps both mandatory operands out of option scanning"
        );
        assert_eq!(
            registry.arg_indices_for_role("lsearch", &option_shaped_operands, ArgRole::Pattern),
            patterns
                .iter()
                .map(|pattern| usize::from(pattern.index))
                .collect::<Vec<_>>(),
            "{dialect}: ArgRole::Pattern must share the resolver answer"
        );
    }

    // `-str` can name only -stride in Tcl 9.  Pre-9 interpreters reject
    // it, so the resolver must abstain rather than treating an invalid
    // dash-prefixed word as the list operand and inventing a pattern.
    let stride_abbrev = ["-str", "2", "{a b}", "a+"];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        assert!(
            registry.pattern_args("lsearch", &stride_abbrev).is_empty(),
            "{dialect}: unavailable -stride abbreviation invalidates the invocation"
        );
        assert!(
            registry
                .arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern)
                .is_empty(),
            "{dialect}: pattern roles abstain with the invalid invocation"
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let patterns = registry.pattern_args("lsearch", &stride_abbrev);
        assert_eq!(
            patterns,
            vec![PatternArg {
                index: 3,
                kind: PatternType::Glob,
            }],
            "{dialect}: -str is the available -stride abbreviation"
        );
        assert_eq!(
            registry.arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern),
            vec![3],
            "{dialect}: roles, hover, and tokens share the Tcl 9 layout"
        );
    }

    // In Tcl 9 `-st` is ambiguous between -start and -stride. Unknown
    // words and unsupported `--` likewise cannot be reclassified as the
    // list operand while the mandatory suffix is still reserved.
    for args in [
        ["-st", "2", "{a b}", "a+"].as_slice(),
        ["-unknown", "{a b}", "a+"].as_slice(),
        ["--", "{a b}", "a+"].as_slice(),
    ] {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            registry.pattern_args("lsearch", args).is_empty(),
            "Tcl 9 invalid option prefix {args:?} must abstain"
        );
        assert!(
            registry
                .arg_indices_for_role("lsearch", args, ArgRole::Pattern)
                .is_empty(),
            "Tcl 9 invalid option prefix {args:?} must have no pattern role"
        );
    }

    // The final two words are always the mandatory list + pattern, even
    // when their spelling would otherwise be an invalid option.
    for args in [
        ["{a b}", "-unknown"].as_slice(),
        ["-regexp", "--"].as_slice(),
        ["-regexp", "-st"].as_slice(),
    ] {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert_eq!(
            registry.pattern_args("lsearch", args),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "Tcl 9 final operands {args:?} stay positional"
        );
    }
}

#[test]
fn source_aware_lsearch_dynamic_prefix_and_suffix_have_distinct_obligations() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    for dialect in ["tcl8.6", "tcl9.0"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let dynamic_early = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        assert!(
            registry
                .pattern_args_words("lsearch", InvocationArguments::structured(&dynamic_early))
                .is_empty(),
            "{dialect}: a dynamic word before the mandatory operands can still be a switch"
        );

        let dynamic_list_operand = [
            InvocationWord::Literal("-regexp"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a+"),
        ];
        assert_eq!(
            registry.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&dynamic_list_operand),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }],
            "{dialect}: the dynamic mandatory list is beyond the option boundary"
        );
    }
}

/// The repeated argument tails a fixed index table cannot
/// express answer through the ordinary role query.
#[test]
fn repeated_layouts_answer_through_arg_indices_for_role() {
    let reg = CommandRegistry::build_default();
    let vars = |name: &str, args: &[&str]| reg.arg_indices_for_role(name, args, ArgRole::VarWrite);
    // TP — every argument of `global`.
    assert_eq!(vars("global", &["a", "b", "c"]), vec![0, 1, 2]);
    // TP — every *even* argument of `variable` (names, not values).
    assert_eq!(vars("variable", &["x", "1", "y", "2"]), vec![0, 2]);
    // TP — the local of each `namespace upvar` pair, past the namespace.
    assert_eq!(
        vars("namespace", &["upvar", "::ns", "o1", "l1", "o2", "l2"]),
        vec![3, 5]
    );
    // TP — each `dict update` varName, with the trailing body excluded.
    // Index 1 is the dictionary variable itself, which the subcommand
    // already declared (it is read on entry and written back after the
    // body).  The pair locals answer as `LoopVarList` — bound once
    // before the body, and only when the key is present — NOT as
    // `VarWrite`: an unconditional SSA def both defeated the key-aware
    // read-before-set suppression and would hide the genuine
    // absent-key warning.
    assert_eq!(
        vars("dict", &["update", "d", "k1", "v1", "k2", "v2", "{body}"]),
        vec![1]
    );
    assert_eq!(
        reg.arg_indices_for_role(
            "dict",
            &["update", "d", "k1", "v1", "k2", "v2", "{body}"],
            ArgRole::LoopVarList
        ),
        vec![3, 5]
    );
    // TP — `foreach` / `lmap` variable specs, body excluded.
    for name in ["foreach", "lmap"] {
        assert_eq!(
            reg.arg_indices_for_role(
                name,
                &["{a b}", "$l1", "c", "$l2", "{body}"],
                ArgRole::LoopVarList
            ),
            vec![0, 2],
            "{name}"
        );
    }
    // FN guard — the explicitly global spellings resolve identically.
    assert_eq!(vars("::global", &["a", "b"]), vars("global", &["a", "b"]));
    // TN — a command with no repeated tail is unaffected.
    assert_eq!(vars("puts", &["a", "b"]), Vec::<usize>::new());
    // FP guard — a short call names nothing it should not.
    assert_eq!(vars("global", &[]), Vec::<usize>::new());
    assert_eq!(
        reg.arg_indices_for_role("foreach", &["{body}"], ArgRole::LoopVarList),
        Vec::<usize>::new()
    );
}

#[test]
fn option_value_arity_shifts_registry_argument_roles() {
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert_eq!(
        reg.arg_indices_for_role(
            "regexp",
            &["-start", "2", "pattern", "$text", "match"],
            ArgRole::VarWrite,
        ),
        vec![4]
    );
    assert_eq!(
        reg.arg_indices_for_role(
            "regsub",
            &["-start", "2", "pattern", "$text", "replacement", "result"],
            ArgRole::VarWrite,
        ),
        vec![5]
    );
    assert_eq!(
        reg.arg_indices_for_role("upvar", &["0", "remote", "local"], ArgRole::VarWrite,),
        vec![2]
    );
}

/// Each `otherVar` of `upvar` and `namespace upvar` names the cell the
/// pair links (#2259), wherever the optional level puts it, beside the
/// `myVar` local the link writes.
#[test]
fn upvar_other_vars_carry_the_cell_role() {
    let reg = CommandRegistry::build_default();
    for (args, cells, locals) in [
        (&["a", "b"][..], vec![0], vec![1]),
        (&["1", "a", "b"][..], vec![1], vec![2]),
        (&["a", "b", "c", "d"][..], vec![0, 2], vec![1, 3]),
        (&["#0", "a", "b", "c", "d"][..], vec![1, 3], vec![2, 4]),
    ] {
        assert_eq!(
            reg.arg_indices_for_role("upvar", args, ArgRole::VarCell),
            cells,
            "{args:?}"
        );
        assert_eq!(
            reg.arg_indices_for_role("upvar", args, ArgRole::VarWrite),
            locals,
            "{args:?}"
        );
    }
    let args = ["upvar", "::ns", "a", "b", "c", "d"];
    assert_eq!(
        reg.arg_indices_for_role("namespace", &args, ArgRole::VarCell),
        vec![2, 4]
    );
    assert_eq!(
        reg.arg_indices_for_role("namespace", &args, ArgRole::VarWrite),
        vec![3, 5]
    );
}

/// `for`'s three script arguments share the semantic
/// [`ArgRole::Body`], and the *presentation* fact is what separates the
/// trailing body (block-expanded) from `start` / `next` (inline).
#[test]
fn for_declares_inline_presentation_for_start_and_next() {
    use crate::presentation::ArgPresentation;
    let reg = CommandRegistry::build_default();
    let args = ["{set i 0}", "{$i < 3}", "{incr i}", "{puts $i}"];
    // Semantics unchanged: all three scripts are still bodies.
    assert_eq!(
        reg.arg_indices_for_role("for", &args, ArgRole::Body),
        vec![0, 2, 3]
    );
    // Presentation splits them.
    assert_eq!(
        reg.arg_presentation("for", &args, 0),
        ArgPresentation::InlineScript
    );
    assert_eq!(
        reg.arg_presentation("for", &args, 2),
        ArgPresentation::InlineScript
    );
    assert_eq!(
        reg.arg_presentation("for", &args, 3),
        ArgPresentation::BlockScript
    );
    assert!(reg.arg_presentation("for", &args, 3).is_block());
    // The absolute global spelling resolves to the same spec.
    assert_eq!(
        reg.arg_presentation("::for", &args, 0),
        ArgPresentation::InlineScript
    );
    // A command with nothing to say answers the block default, and so
    // does one the registry does not know at all.
    assert_eq!(
        reg.arg_presentation("while", &["{$x}", "{body}"], 1),
        ArgPresentation::BlockScript
    );
    assert_eq!(
        reg.arg_presentation("no::such::command", &["{a}"], 0),
        ArgPresentation::BlockScript
    );
}

// Option-value roles.

fn opt(name: &'static str, value: crate::hover::OptionValue) -> crate::hover::OptionSpec {
    crate::hover::OptionSpec {
        name,
        value,
        ..crate::hover::OptionSpec::DEFAULT
    }
}

fn opt_with_alias(
    name: &'static str,
    aliases: &'static [&'static str],
    value: crate::hover::OptionValue,
) -> crate::hover::OptionSpec {
    crate::hover::OptionSpec {
        name,
        value,
        aliases,
        ..crate::hover::OptionSpec::DEFAULT
    }
}

fn indices(options: &[crate::hover::OptionSpec], args: &[&str], role: ArgRole) -> Vec<usize> {
    let mut out = Vec::new();
    push_option_value_roles(
        &mut out,
        &options.iter().collect::<Vec<_>>(),
        args,
        0,
        &|wanted| wanted == role,
        PrefixMatching::Enabled,
    );
    out.into_iter().map(|(index, _)| index).collect()
}

#[test]
fn option_value_role_emits_body_index() {
    use crate::hover::OptionValue;
    let options = [
        opt("-command", OptionValue::script()),
        opt("-flag", OptionValue::flag()),
    ];
    let args = ["-command", "{puts hi}", "-flag", "x"];
    // The `-command` value is a Body; the flag consumes nothing.
    assert_eq!(indices(&options, &args, ArgRole::Body), vec![1]);
    // It is not a generic Value role.
    assert_eq!(indices(&options, &args, ArgRole::Value).len(), 0);
}

/// Explicit metadata environment; this does not install a source receiver.
fn callback_metadata_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    for (package, version) in [("Tk", "9.1"), ("tcltest", "2.5.8"), ("bibtex", "0.8")] {
        registry.insert_ambient_package(package, version);
    }
    registry
}

#[test]
fn script_timing_is_exact_for_mixed_immediate_and_deferred_arguments() {
    use crate::hover::{OptionSpec, OptionValue, ScriptTiming};

    static OPTIONS: &[OptionSpec] = &[OptionSpec {
        name: "-later",
        value: OptionValue::deferred_script(),
        detail: "stored callback",
        ..OptionSpec::DEFAULT
    }];
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "mixed-script-timing",
        arity: crate::Arity::exact(3),
        arg_roles: &[(0, ArgRole::Body)],
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    });

    let args = ["{error now}", "-later", "{error later}"];
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 0, None),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 2, None),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 1, None),
        None,
    );
}

#[test]
fn shipped_script_options_distinguish_tk_callbacks_from_tcltest_bodies() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let button = [".b", "-command", "{return}"];
    assert_eq!(
        reg.script_timing("button", &button, 2, None),
        Some(ScriptTiming::Deferred),
    );

    let test = ["sample", "description", "-body", "{error failure}"];
    assert_eq!(
        reg.script_timing("tcltest::test", &test, 3, None),
        Some(ScriptTiming::SameInvocation),
    );

    let dump = ["dump", "-command", "visit", "1.0"];
    assert_eq!(
        reg.instance_script_timing("text", ".receiver", &dump, 2, None),
        Some(ScriptTiming::SameInvocation),
        "text dump calls its prefix synchronously for each returned item"
    );
    let sync = ["sync", "-command", "ready"];
    assert_eq!(
        reg.instance_script_timing("text", ".receiver", &sync, 2, None),
        Some(ScriptTiming::Deferred),
        "text sync schedules its prefix after metrics become current"
    );
}

#[test]
fn callback_taint_inputs_are_exact_and_exclude_tk_bookkeeping() {
    use crate::CallbackTaintInput;

    let reg = callback_metadata_registry();
    let validation = [".entry", "-validatecommand", "{eval %P}"];
    assert_eq!(
        reg.callback_taint_inputs("entry", &validation, 2, None),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ],
    );
    let bind = [".entry", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.callback_taint_inputs("bind", &bind, 2, None),
        &[CallbackTaintInput::TK_EVENT_CHAR],
    );
    let plain_command = [".entry", "-command", "{eval %P}"];
    assert!(
        reg.callback_taint_inputs("button", &plain_command, 2, None)
            .is_empty(),
        "a deferred callback without user substitutions must stay clean"
    );

    let canvas_bind = ["bi", "item", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("canvas", ".canvas", &canvas_bind, 3, None),
        &[CallbackTaintInput::TK_EVENT_CHAR],
        "a unique-prefix widget subcommand must select its callback table"
    );

    let abbreviated_configure = ["conf", "-validatecommand", "{eval %P}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("entry", ".entry", &abbreviated_configure, 2, None,),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ],
        "a unique-prefix widget method must inherit constructor callback options"
    );

    let instance_bind = ["bind", "all", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("canvas", ".canvas", &instance_bind, 3, None,),
        &[CallbackTaintInput::TK_EVENT_CHAR],
        "a positional instance-method callback must use its method table"
    );
    assert!(
        reg.instance_callback_taint_inputs(
            "entry",
            ".entry",
            &["configure", "-validatecommand"],
            1,
            None,
        )
        .is_empty(),
        "the option word itself is not callback text"
    );
}

/// Classic `entry` and `spinbox` share `tkEntry.c` in Tk 8.4, 8.5, 8.6,
/// 9.0 and 9.1. Each declares `-vcmd` / `-invcmd` as `TK_OPTION_SYNONYM`s
/// of the validation options, and `ExpandPercents` gives the spinbox
/// `-command` (`VALIDATE_BUTTON`) only `%s` (current text), `%d` (arrow
/// direction) and `%W`.
#[test]
fn classic_entry_callback_synonyms_and_spinbox_command_carry_user_text() {
    use crate::CallbackTaintInput;

    let validation = [
        CallbackTaintInput::TK_PROPOSED_VALUE,
        CallbackTaintInput::TK_CURRENT_VALUE,
        CallbackTaintInput::TK_EDIT_TEXT,
    ];
    for (tcl, tk) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
    ] {
        let mut reg = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).unwrap());
        reg.insert_ambient_package("Tk", tk);
        for command in ["entry", "spinbox"] {
            for option in ["-vcmd", "-invcmd"] {
                let words = [".w", option, "{eval %P}"];
                assert_eq!(
                    reg.callback_taint_inputs(command, &words, 2, None),
                    &validation,
                    "Tk {tk} {command} {option} is a synonym of a validation callback"
                );
                let method = ["configure", option, "{eval %P}"];
                assert_eq!(
                    reg.instance_callback_taint_inputs(command, ".w", &method, 2, None),
                    &validation,
                    "Tk {tk} {command} configure {option}"
                );
            }
        }
        let command = [".w", "-command", "{eval %s}"];
        assert_eq!(
            reg.callback_taint_inputs("spinbox", &command, 2, None),
            &[CallbackTaintInput::TK_CURRENT_VALUE],
            "Tk {tk} spinbox -command receives the editable text as %s"
        );
        assert!(
            reg.callback_taint_inputs("ttk::spinbox", &command, 2, None)
                .is_empty(),
            "Tk {tk} ttk::spinbox runs -command with `uplevel #0`, no % substitution"
        );
    }
}

/// The registry states which stored scripts are callbacks — the words a
/// consumer must treat as run later, outside the registering frame — and
/// which are a definition's body or run now.
#[test]
fn callback_scripts_are_the_deferred_words_of_a_command_that_stores_no_definition() {
    let reg = CommandRegistry::build_default();
    let callbacks: &[(&str, &[&str], &[usize])] = &[
        ("after", &["100", "{set done 1}"], &[1]),
        ("after", &["idle", "{set done 1}"], &[1]),
        (
            "trace",
            &["add", "variable", "x", "write", "{set go 0 ;#}"],
            &[4],
        ),
        ("bind", &[".", "<Key>", "{set go 0}"], &[2]),
        ("fileevent", &["stdin", "readable", "{set go 0}"], &[2]),
        ("chan", &["event", "stdin", "readable", "{set go 0}"], &[3]),
        ("interp", &["bgerror", "{}", "{set go 0 ;#}"], &[2]),
        ("button", &[".b", "-command", "{set go 0}"], &[2]),
    ];
    for (name, args, expected) in callbacks {
        assert_eq!(
            reg.callback_script_indices(name, args, None),
            *expected,
            "{name} {args:?}"
        );
    }
    // A definition's body is dormant and runs in a frame of its own, and
    // the scripts these run now or only name are no callback.
    let none: &[(&str, &[&str])] = &[
        ("proc", &["p", "{}", "{set x 1}"]),
        ("snit::method", &["T", "m", "{}", "{set x 1}"]),
        ("lambda", &["{}", "{set x 1}"]),
        ("catch", &["{set x 1}"]),
        ("if", &["1", "{set x 1}"]),
        ("uplevel", &["#0", "{set x 1}"]),
        ("trace", &["remove", "variable", "x", "write", "cb"]),
        ("set", &["x", "1"]),
    ];
    for (name, args) in none {
        assert!(
            reg.callback_script_indices(name, args, None).is_empty(),
            "{name} {args:?}"
        );
    }
}

#[test]
fn typed_callback_inputs_preserve_unknown_prefix_slots_and_decline_expansion() {
    use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
    let registry = callback_metadata_registry();
    let unknown_window = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
    ];
    let invocation = InvocationWords::structured(InvocationWord::Literal("bind"), &unknown_window);
    assert_eq!(
        registry.callback_taint_inputs_words(invocation, 2, None),
        &[CallbackTaintInput::TK_EVENT_CHAR]
    );
    assert_eq!(
        registry.callback_taint_inputs_words(invocation, 1, None),
        &[] as &[CallbackTaintInput]
    );
    for arguments in [&unknown_window[..1], &unknown_window[..2]] {
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("bind"), arguments),
                2,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
    }
    let invalid = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
        InvocationWord::Literal("extra"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("bind"), &invalid),
            2,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let canvas = [
        InvocationWord::Literal("bind"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
    ];
    assert_eq!(
        registry.instance_callback_taint_inputs_words(
            "canvas",
            InvocationWords::structured(InvocationWord::Literal(".canvas"), &canvas),
            3,
            None,
        ),
        &[CallbackTaintInput::TK_EVENT_CHAR]
    );
    let expanded_window = [InvocationWord::Expanded, InvocationWord::Literal("eval %A")];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("bind"), &expanded_window),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let unknown_option = [
        InvocationWord::Literal(".entry"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("eval %P"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("entry"), &unknown_option),
            2,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
}

#[test]
fn typed_callback_inputs_do_not_override_argument_sensitive_timing() {
    use crate::hover::ScriptTiming;
    use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "timed-callback",
        arity: crate::Arity::exact(2),
        arg_roles: &[(1, ArgRole::Body)],
        traits: Traits::DEFERS_BODY,
        callback_taint_inputs: &[(1, &[CallbackTaintInput::TK_EVENT_CHAR])],
        script_timing_resolver: Some(|_| vec![(1, ScriptTiming::SameInvocation)]),
        ..CommandSpec::DEFAULT
    });
    let unknown = [InvocationWord::Dynamic, InvocationWord::Literal("eval %A")];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("timed-callback"), &unknown),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let known = [
        InvocationWord::Literal("window"),
        InvocationWord::Literal("eval %A"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("timed-callback"), &known),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
}

#[test]
fn argument_sensitive_timing_distinguishes_registration_and_send_forms() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    assert_eq!(
        reg.instance_script_timing(
            "canvas",
            ".receiver",
            &["bind", "tag", "<Button-1>", "clicked"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.instance_script_timing(
            "canvas",
            ".receiver",
            &["bi", "tag", "<Button-1>", "clicked"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
        "canvas bind's unique prefix must select the positional timing resolver",
    );
    assert_eq!(
        reg.script_timing(
            "wm",
            &["protocol", ".w", "WM_DELETE_WINDOW", "close"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("wm", &["prot", ".w", "WM_DELETE_WINDOW", "close"], 3, None,),
        Some(ScriptTiming::Deferred),
        "wm protocol's unique prefix must not fall back to same-invocation timing",
    );
    assert_eq!(
        reg.script_timing("send", &["other", "work"], 1, None),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.script_timing("send", &["-async", "other", "work"], 2, None,),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("send", &["-async", "other", "work", "argument"], 2, None,),
        None,
        "a concatenated multi-word script is not a fictional Body position",
    );
    assert_eq!(
        reg.script_timing(
            "selection",
            &["handle", "-type", "UTF8_STRING", ".w", "provide"],
            4,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing(
            "selection",
            &["h", "-type", "UTF8_STRING", ".w", "provide"],
            4,
            None,
        ),
        Some(ScriptTiming::Deferred),
        "selection handle's unique prefix must select its positional timing resolver",
    );
    assert_eq!(
        reg.script_timing("selection", &["o", "-command", "lost", ".w"], 2, None,),
        Some(ScriptTiming::Deferred),
        "subcommand option timing must use the same unique-prefix resolution",
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn command_prefix_hosts_report_their_exact_phase() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let cases: &[(&str, &[&str], usize, ScriptTiming)] = &[
        // Immediate consumers execute the prefix before returning.
        (
            "lsort",
            &["-command", "compare", "{b a}"],
            1,
            ScriptTiming::SameInvocation,
        ),
        (
            "coroprobe",
            &["worker", "inspect"],
            1,
            ScriptTiming::SameInvocation,
        ),
        (
            "generator",
            &["reduce", "combine", "0", "values"],
            1,
            ScriptTiming::SameInvocation,
        ),
        // Registration and asynchronous forms store the prefix.
        (
            "socket",
            &["-server", "accept", "0"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "fcopy",
            &["in", "out", "-command", "done"],
            3,
            ScriptTiming::Deferred,
        ),
        (
            "chan",
            &["copy", "in", "out", "-command", "done"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "interp",
            &["alias", "", "aliasName", "", "target"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "interp",
            &["bgerror", "", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "namespace",
            &["unknown", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "package",
            &["unknown", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "coroinject",
            &["worker", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "tcltest::customMatch",
            &["mode", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "trace",
            &["add", "variable", "v", "write", "handler"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "generator",
            &["map", "transform", "values"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "hook",
            &["bind", "subject", "name", "observer", "handler"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "comm::comm",
            &["send", "-command", "handler", "peer", "work"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "mime::getbody",
            &["token", "-command", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "stringprep::register",
            &["profile", "-prohibitedCommand", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "tcl::chan::halfpipe",
            &["-close-command", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        // Removal forms name executable text without running or storing it.
        (
            "trace",
            &["remove", "variable", "v", "write", "handler"],
            4,
            ScriptTiming::ReferenceOnly,
        ),
    ];

    for &(command, args, index, expected) in cases {
        assert_eq!(
            reg.script_timing(command, args, index, None),
            Some(expected),
            "wrong phase for {command} {args:?} argument {index}",
        );
    }

    assert_eq!(
        reg.script_timing("trace", &["a", "v", "v", "write", "handler"], 4, None,),
        Some(ScriptTiming::Deferred),
        "unique-prefix subcommands and trace types preserve phase metadata",
    );
}

#[test]
fn legacy_trace_prefix_timing_requires_a_release_with_that_form() {
    use crate::hover::ScriptTiming;
    for (profile, available) in [
        ("tcl8.4", true),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", false),
        ("tcl9.1", false),
    ] {
        let owner = crate::model::ingress::static_context_for(profile);
        let registry = owner.commands();
        for (operation, timing) in [
            ("variable", ScriptTiming::Deferred),
            ("vdelete", ScriptTiming::ReferenceOnly),
        ] {
            assert_eq!(
                registry.script_timing("trace", &[operation, "v", "w", "handler"], 3, None),
                available.then_some(timing),
                "{profile}: {operation}"
            );
        }
    }
}

#[test]
fn option_timing_resolver_overrides_static_default_for_bibtex_input_mode() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let in_memory = ["-recordcommand", "record", "@book{x}"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &in_memory, 1, None),
        Some(ScriptTiming::SameInvocation),
    );

    let channel = ["-recordcommand", "record", "-channel", "input"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &channel, 1, None),
        Some(ScriptTiming::Deferred),
    );

    let completion = ["-command", "done", "-channel", "input"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &completion, 1, None),
        Some(ScriptTiming::Deferred),
    );
}

#[test]
fn instance_timing_requires_the_selected_provider_and_method() {
    use crate::hover::ScriptTiming;

    let provided = callback_metadata_registry();
    let arguments = ["dump", "-command", "visit", "1.0"];
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &arguments, 2, None),
        Some(ScriptTiming::SameInvocation),
    );
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert_eq!(
        unprovided.instance_script_timing("text", ".receiver", &arguments, 2, None),
        None,
    );
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &["unknown", "visit"], 1, None),
        None,
    );
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &arguments, 1, None),
        None,
        "an option name is not executable callback text",
    );
}

#[test]
fn constructor_option_callbacks_keep_their_original_argument_positions() {
    use crate::hover::ScriptTiming;
    let registry = callback_metadata_registry();
    let constructor = [".canvas", "-xscrollcommand", "visit"];
    assert_eq!(
        registry.script_timing("canvas", &constructor, 2, None),
        Some(ScriptTiming::Deferred),
    );
    for index in [0, 1] {
        assert_eq!(
            registry.script_timing("canvas", &constructor, index, None),
            None
        );
    }
    assert_eq!(
        registry.script_timing("canvas", &[".canvas", "-xscrollcommand"], 2, None),
        None,
    );
    let method = ["configure", "-xscrollcommand", "visit"];
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &method, 2, None),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &method, 1, None),
        None,
    );
    for query in [vec!["configure"], vec!["configure", "-xscrollcommand"]] {
        let invocation = registry
            .resolve_instance_invocation("canvas", ".canvas", &query, None)
            .expect("the provided classic widget retains its configure query");
        assert!(!invocation.semantics.mutator);
        assert!(invocation.semantics.traits.contains(Traits::PURE));
        assert_eq!(
            registry.instance_script_timing("canvas", ".canvas", &query, 1, None),
            None,
            "query forms do not execute the stored option value",
        );
    }
    let setter = registry
        .resolve_instance_invocation("canvas", ".canvas", &method, None)
        .expect("the provided classic widget retains its configure setter");
    assert!(setter.semantics.mutator);
    assert!(
        setter
            .semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
    );
    let cget = ["cget", "-xscrollcommand"];
    let invocation = registry
        .resolve_instance_invocation("canvas", ".canvas", &cget, None)
        .expect("the provided classic widget retains its cget method");
    assert!(!invocation.semantics.mutator);
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &cget, 1, None),
        None,
    );
}

#[test]
fn known_stored_tk_command_prefixes_are_deferred() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let stored = [
        ("canvas", "-xscrollcommand"),
        ("canvas", "-yscrollcommand"),
        ("entry", "-xscrollcommand"),
        ("listbox", "-xscrollcommand"),
        ("listbox", "-yscrollcommand"),
        ("menu", "-tearoffcommand"),
        ("scale", "-command"),
        ("scrollbar", "-command"),
        ("spinbox", "-xscrollcommand"),
        ("text", "-xscrollcommand"),
        ("text", "-yscrollcommand"),
        ("ttk::combobox", "-xscrollcommand"),
        ("ttk::entry", "-xscrollcommand"),
        ("ttk::scale", "-command"),
        ("ttk::scrollbar", "-command"),
        ("ttk::spinbox", "-xscrollcommand"),
        ("ttk::treeview", "-xscrollcommand"),
        ("ttk::treeview", "-yscrollcommand"),
        ("tk_chooseDirectory", "-command"),
        ("tk_getOpenFile", "-command"),
        ("tk_getSaveFile", "-command"),
        ("tk_messageBox", "-command"),
    ];
    for (command, option) in stored {
        let spec = reg
            .get_for_surface(command, reg.own_surface_query())
            .unwrap();
        let arguments = if spec.constructor_prefix_words().is_some() {
            vec![".w", option, "callback"]
        } else {
            vec![option, "callback"]
        };
        assert_eq!(
            reg.script_timing(command, &arguments, arguments.len() - 1, None),
            Some(ScriptTiming::Deferred),
            "{command} {option} stores its command prefix",
        );
    }

    assert_eq!(
        reg.instance_script_timing(
            "text",
            ".receiver",
            &["dump", "-command", "visit", "1.0"],
            2,
            None,
        ),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.instance_script_timing(
            "ttk::treeview",
            ".receiver",
            &["sort", "root", "-command", "compare"],
            3,
            None,
        ),
        Some(ScriptTiming::SameInvocation),
    );
}

#[test]
fn option_value_fixed_arity_emits_n_indices() {
    use crate::hover::OptionValue;
    let options = [opt("-rect", OptionValue::fixed(4, ArgRole::Value, "coord"))];
    let args = ["-rect", "1", "2", "3", "4", "tail"];
    assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3, 4]);
}

/// An authored hook owns its own termination policy.
fn rest_value(args: &[&str], start: usize) -> crate::hover::OptionValueOutcome {
    crate::hover::OptionValueOutcome {
        words: args[start..]
            .iter()
            .position(|word| *word == "--")
            .unwrap_or(args.len() - start),
        invalid: None,
    }
}

#[test]
fn option_value_rest_arity_stops_at_terminator() {
    let rest = crate::hover::OptionSpec {
        name: "-rest",
        value: crate::hover::OptionValue::Takes(crate::hover::OptionArg {
            arity: crate::hover::OptionArity::Hook(rest_value),
            ..crate::hover::OptionArg::DEFAULT
        }),
        ..crate::hover::OptionSpec::DEFAULT
    };
    let options = [rest];
    let args = ["-rest", "a", "b", "c", "--", "d"];
    // Consumes a, b, c up to `--`, not d.
    assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3]);
}

#[test]
fn option_value_terminator_stops_scan() {
    use crate::hover::OptionValue;
    let options = [opt("-command", OptionValue::script())];
    let args = ["--", "-command", "{x}"];
    assert_eq!(indices(&options, &args, ArgRole::Body).len(), 0);
}

#[test]
fn option_value_two_way_var_name_emits_for_both_roles() {
    use crate::hover::OptionValue;
    let options = [opt("-textvariable", OptionValue::var_name())];
    let args = ["-textvariable", "myvar"];
    assert_eq!(indices(&options, &args, ArgRole::VarWrite), vec![1]);
    assert_eq!(indices(&options, &args, ArgRole::VarRead), vec![1]);
}

#[test]
fn option_value_alias_matches_and_dynamic_flag_skipped() {
    use crate::hover::OptionValue;
    let options = [opt_with_alias("-command", &["-cmd"], OptionValue::script())];
    // Alias resolves to the same value role.
    assert_eq!(indices(&options, &["-cmd", "{x}"], ArgRole::Body), vec![1]);
    // A `$var` in flag position isn't an option name → treated as a
    // positional, so the real `-command` after it still resolves.
    assert_eq!(
        indices(&options, &["$opt", "v", "-command", "{x}"], ArgRole::Body),
        vec![3]
    );
}

#[test]
fn options_do_not_leak_into_positional_role_queries() {
    // A real command with value-taking options but no role annotations
    // must not surface any option value under Body/VarWrite (inert until
    // annotated).
    let reg = CommandRegistry::build_default();
    let b = reg.arg_indices_for_role("entry", &[".e", "-width", "10"], ArgRole::Body);
    assert!(b.is_empty(), "{b:?}");
}

#[test]
fn command_prefix_option_is_captured_but_never_a_body() {
    // `lsort -command cmp {a b}` — the `-command` value is a CommandPrefix,
    // so it is captured under that role but never returned for a
    // Body query, i.e. a bareword prefix is not recursed as a script.
    let reg = CommandRegistry::build_default();
    let args = ["-command", "cmp", "{a b}"];
    assert!(
        reg.arg_indices_for_role("lsort", &args, ArgRole::Body)
            .is_empty(),
        "a command prefix must not be recursed as a body",
    );
    assert_eq!(
        reg.arg_indices_for_role("lsort", &args, ArgRole::CommandPrefix),
        vec![1],
        "the -command value should carry the CommandPrefix role",
    );
}

#[test]
fn command_prefixes_carry_verified_arities() {
    // Ground-truthed vs real tclsh (stable 8.6→9.0). `command_prefixes`
    // is the single source of truth for both the position and the
    // appended arity.
    let reg = CommandRegistry::build_default();
    // `lsort -command cmp {a b}` → cmp invoked as `cmp x y` (2 appended).
    assert_eq!(
        reg.command_prefixes("lsort", &["-command", "cmp", "{a b}"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // `socket -server accept 9000` → accept invoked as `accept ch a p` (3).
    assert_eq!(
        reg.command_prefixes("socket", &["-server", "accept", "9000"]),
        vec![(1, AppendedArity::Exactly(3))],
    );
    // Positional: `tcltest::customMatch mode cmd`
    // → `cmd expected actual` (2).
    assert_eq!(
        reg.command_prefixes("tcltest::customMatch", &["exact", "cmp"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // Dynamic resolver: `selection handle window cmd` → the
    // last arg, invoked as `cmd offset maxChars` (2).
    assert_eq!(
        reg.command_prefixes("selection", &["handle", ".w", "getData"]),
        vec![(2, AppendedArity::Exactly(2))],
    );
}

#[test]
fn command_prefixes_cover_core_callback_commands() {
    // Ground-truthed vs real tclsh 8.6/9.0 (stable). Locks in the
    // coverage of trace / interp / tcllib callbacks.
    let reg = CommandRegistry::build_default();
    // `trace add variable v w cb` → cb(name1 name2 op) = 3.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "variable", "v", "write", "cb"]),
        vec![(4, AppendedArity::Exactly(3))],
    );
    // `trace add command c ops cb` → cb(old new op) = 3.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "command", "c", "rename", "cb"]),
        vec![(4, AppendedArity::Exactly(3))],
    );
    // Execution operations select exact callback contracts.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "enter", "cb"]),
        vec![(4, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "leave", "cb"]),
        vec![(4, AppendedArity::Exactly(4))],
    );
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "enter leave", "cb"]),
        vec![(
            4,
            AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 4]))
        )],
    );
    // Deprecated `trace variable v ops cb` → cb at index 3, 3 appended args.
    assert_eq!(
        reg.command_prefixes("trace", &["variable", "v", "write", "cb"]),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `interp alias {} a {} target x` (create form) → target at index 3,
    // variadic. The 2-arg query form has no target.
    assert_eq!(
        reg.command_prefixes("interp", &["alias", "{}", "a", "{}", "target"]),
        vec![(4, AppendedArity::Unknown)],
    );
    assert!(
        reg.command_prefixes("interp", &["alias", "{}", "a"])
            .is_empty(),
        "the interp-alias query form has no command prefix",
    );
    // tcllib: `struct::list filter seq cb` (1), `map seq cb` (1),
    // `fold seq init cb` (2).
    assert_eq!(
        reg.command_prefixes("struct::list", &["filter", "$s", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("struct::list", &["fold", "$s", "0", "cb"]),
        vec![(3, AppendedArity::Exactly(2))],
    );
}

#[test]
fn command_prefixes_cover_deferred_core_commands() {
    // Version-gated / resolver-driven callback tails wired in the deferred
    // pass. Ground-truthed vs tclsh 9.0 (all are 8.7/9.0-era surfaces).
    let reg = CommandRegistry::build_default();

    // `regsub -command re str cmdPrefix ?var?` (TIP 463, 9.0): with
    // `-command` the subSpec slot is a prefix called per match with the
    // whole match + capture groups appended (variadic ⇒ AtLeast(1)). The
    // slot index tracks the leading-switch shift.
    assert_eq!(
        reg.command_prefixes("regsub", &["-command", "re", "s", "cb"]),
        vec![(3, AppendedArity::AtLeast(1))],
    );
    assert_eq!(
        reg.command_prefixes("regsub", &["-all", "-command", "re", "s", "cb"]),
        vec![(4, AppendedArity::AtLeast(1))],
    );
    // `-c` is NOT an abbreviation of `-command`: regsub's switch table
    // resolves with Tcl_GetIndexFromObj(..., TCL_EXACT, ...), confirmed
    // live (tclsh 8.6.14: `regsub -c {a} aaa X y` -> `bad option "-c"`)
    // and against the real `Tcl_RegsubObjCmd` C source. Only the exact
    // spelling `-command` enables command-prefix mode.
    assert_eq!(
        reg.command_prefixes("regsub", &["-c", "re", "s", "cb"]),
        Vec::new(),
    );
    // Without `-command`, subSpec is a replacement template, not a prefix.
    assert!(
        reg.command_prefixes("regsub", &["re", "s", "template"])
            .is_empty(),
        "plain regsub subSpec is not a command prefix",
    );
    // `--` terminates switches, so a following `-command`-looking word is a
    // pattern, not the flag.
    assert!(
        reg.command_prefixes("regsub", &["--", "-command", "s", "template"])
            .is_empty(),
        "`--` disables -command detection",
    );

    // `namespace unknown handler` → handler(cmd ?arg...?) = AtLeast(1). The
    // zero-arg query form carries no prefix.
    assert_eq!(
        reg.command_prefixes("namespace", &["unknown", "handler"]),
        vec![(1, AppendedArity::AtLeast(1))],
    );
    assert!(
        reg.command_prefixes("namespace", &["unknown"]).is_empty(),
        "the namespace-unknown query form has no command prefix",
    );

    // `package unknown handler` → handler(name requirement ?requirement
    // ...?) = AtLeast(2): verified empirically on tclsh 8.6.14 that Tcl
    // always appends the package name *plus* at least one requirement
    // word, synthesising a "0-" placeholder when `package require` was
    // given none itself — never just the bare name, so AtLeast(2), not
    // AtLeast(1) (see the fuller note on `package_.rs`'s `unknown`
    // subcommand).
    assert_eq!(
        reg.command_prefixes("package", &["unknown", "handler"]),
        vec![(1, AppendedArity::AtLeast(2))],
    );
    assert!(
        reg.command_prefixes("package", &["unknown"]).is_empty(),
        "the package-unknown query form has no command prefix",
    );

    // `coroinject coroName command ?arg...?` — per the Tcl 9.0/9.1
    // coroutine(n) manpage, exactly two more words are appended when the
    // injected command runs: the name of the command that suspended the
    // coroutine (yield or yieldto) and its current resumption value.
    assert_eq!(
        reg.command_prefixes("coroinject", &["myCoro", "cb", "x"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // `coroprobe coroName command ?arg...?` runs command immediately
    // inside the suspended coroutine (not deferred through a yield/
    // yieldto resumption), so no fixed extra-argument count is
    // documented -- it stays a reference-only prefix (Unknown).
    assert_eq!(
        reg.command_prefixes("coroprobe", &["myCoro", "cb"]),
        vec![(1, AppendedArity::Unknown)],
    );

    // `chan create mode cmdPrefix` / `chan push channelId cmdPrefix` — the
    // reflected-channel/transform handler is invoked as `cmdPrefix
    // subcommand handle ?args...?` ⇒ AtLeast(2).
    assert_eq!(
        reg.command_prefixes("chan", &["create", "rw", "handler"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );
    assert_eq!(
        reg.command_prefixes("chan", &["push", "$ch", "xform"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );
}

#[test]
fn command_prefixes_cover_tcllib_callbacks() {
    // tcllib callback tails wired in the deferred pass.  Fixed arities are
    // ground-truthed against the package man pages; ambiguous/variadic tails
    // are Unknown (reference-only).
    let reg = CommandRegistry::build_default();

    // `struct::list split seq cmdprefix` — the twin of filter: cmdprefix(el).
    assert_eq!(
        reg.command_prefixes("struct::list", &["split", "$s", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    // `fileutil::find basedir filtercmd` — filtercmd(name).  Shorter forms
    // carry no prefix (idx≥argc dropped).
    assert_eq!(
        reg.command_prefixes("fileutil::find", &["/base", "flt"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert!(
        reg.command_prefixes("fileutil::find", &["/base"])
            .is_empty(),
        "the basedir-only fileutil::find form has no filter prefix",
    );

    // generator functional ops: reference-only (multi-value yield ⇒ Unknown).
    assert_eq!(
        reg.command_prefixes("generator", &["map", "fn", "$g"]),
        vec![(1, AppendedArity::Unknown)],
    );
    assert_eq!(
        reg.command_prefixes("generator", &["filter", "pred", "$g"]),
        vec![(1, AppendedArity::Unknown)],
    );

    // math::calculus func callbacks (man-page-pinned fixed arities).
    assert_eq!(
        reg.command_prefixes("math::calculus::integral", &["0", "1", "100", "f"]),
        vec![(3, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("math::calculus::integral3D", &["xi", "yi", "zi", "f"]),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `newtonRaphson func deriv initval` — two prefixes, each f(x).
    assert_eq!(
        reg.command_prefixes("math::calculus::newtonRaphson", &["f", "d", "0.5"]),
        vec![
            (0, AppendedArity::Exactly(1)),
            (1, AppendedArity::Exactly(1)),
        ],
    );
    // `math::probopt::pso function bounds ?args?` — objective(coordVec).
    assert_eq!(
        reg.command_prefixes("math::probopt::pso", &["obj", "bnds", "-iter", "50"]),
        vec![(0, AppendedArity::Exactly(1))],
    );

    // log message-writer callbacks — `cmd level text` (Exactly(2)).
    assert_eq!(
        reg.command_prefixes("log::lvCmd", &["debug", "writer"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("log::lvCmdForall", &["writer"]),
        vec![(0, AppendedArity::Exactly(2))],
    );
    // `uevent::bind tag event command` — command(tag event ?details?).
    assert_eq!(
        reg.command_prefixes("uevent::bind", &["tag", "ev", "cb"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );

    // `hook bind subject hook observer binding` — the 4-word set form names
    // a command prefix (Unknown appended: the count is whatever the matching
    // `hook call` passes); the shorter query forms name none.
    assert_eq!(
        reg.command_prefixes("hook", &["bind", "sub", "hk", "obs", "cb"]),
        vec![(4, AppendedArity::Unknown)],
    );
    assert!(
        reg.command_prefixes("hook", &["bind", "sub", "hk", "obs"])
            .is_empty(),
        "the 3-arg `hook bind` query form has no callback prefix",
    );

    // `processman::onexit id cmd` — `cmd` is a deferred *script* (`eval $cmd`,
    // 0 appended), NOT a command prefix: it must declare none.
    assert!(
        reg.command_prefixes("processman::onexit", &["$pid", "cb"])
            .is_empty(),
        "processman::onexit cmd is a script body, not a command prefix",
    );
    assert_eq!(
        reg.get("processman::onexit")
            .and_then(|s| s.arg_roles.iter().find(|(i, _)| *i == 1))
            .map(|(_, r)| *r),
        Some(ArgRole::Body),
        "processman::onexit cmd (index 1) must carry the Body script role",
    );
}

#[test]
fn command_prefixes_cover_option_value_callbacks() {
    // Option-value callbacks — the prefix is the value of a named `-flag`,
    // resolved through each command's `OptionSpec` array.  Arities are
    // ground-truthed against tcllib source (verified by re-running the exact
    // `uplevel`/`eval`/`{*}` idioms in tclsh — `uplevel`/`eval` re-split a
    // `[list …]` word, `[list {*}pfx {*}args]` keeps one word per element).
    let reg = CommandRegistry::build_default();

    // `mime::getbody token -command cb` — async body callback.  uplevel
    // re-splits `[list end]` → 1 word, `[list data $c]` → 2 ⇒ AtLeast(1).
    assert_eq!(
        reg.command_prefixes("mime::getbody", &["tok", "-command", "cb"]),
        vec![(2, AppendedArity::AtLeast(1))],
    );
    // `-decode` before `-command` is a value-less flag: the scan skips it
    // without swallowing the callback.
    assert_eq!(
        reg.command_prefixes("mime::getbody", &["tok", "-decode", "-command", "cb"]),
        vec![(3, AppendedArity::AtLeast(1))],
    );

    // `smtp::sendmessage tok -tlspolicy pol` — `eval $pol [list $code]
    // [list $diag]` ⇒ Exactly(2).  A preceding value-taking option's value
    // is skipped, not mistaken for the callback.
    assert_eq!(
        reg.command_prefixes("smtp::sendmessage", &["tok", "-tlspolicy", "pol"]),
        vec![(2, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes(
            "smtp::sendmessage",
            &["tok", "-servers", "mail.x", "-tlspolicy", "pol"],
        ),
        vec![(4, AppendedArity::Exactly(2))],
    );

    // `comm::comm send -command cb id cmd` — 7 `-key value` reply pairs ⇒
    // Exactly(14).  Subcommand-relative option scan offsets by 1.
    assert_eq!(
        reg.command_prefixes("comm::comm", &["send", "-command", "cb", "id", "cmd"]),
        vec![(2, AppendedArity::Exactly(14))],
    );

    // `bibtex::parse` — every callback prepends the parser token, then its
    // own payload words: `-command`/`-*command` ⇒ Exactly(2), except
    // `-recordcommand` ⇒ Exactly(4) (token type key recdata).
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-recordcommand", "cb", "text"]),
        vec![(1, AppendedArity::Exactly(4))],
    );
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-command", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-progresscommand", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );

    // `tcl::chan::halfpipe` — clean `[list {*}pfx {*}args]` idiom: write
    // appends (chan bytes) ⇒ 2, empty/close append (chan) ⇒ 1.  No
    // `-read-command` exists.
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-write-command", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-close-command", "cb"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-empty-command", "cb"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-read-command", "cb"])
            .is_empty(),
        "halfpipe has no -read-command",
    );
}

#[test]
fn instance_method_command_prefixes_cover_struct_graph_and_tree() {
    // Object-instance method callbacks — the prefix is on a method of a
    // created object command (`$g walk … -command cb`, `$t walkproc … cb`),
    // resolved through the class's ObjectClassSpec.  Indices are relative to
    // the words after the method name.
    let reg = CommandRegistry::build_default();

    // struct::graph `walk node … -command cb` — option-value prefix,
    // Exactly(3) (action graphName node).
    assert_eq!(
        reg.instance_method_command_prefixes(
            "struct::graph",
            "walk",
            &["root", "-order", "pre", "-command", "cb"],
        ),
        vec![(4, AppendedArity::Exactly(3))],
    );
    assert_eq!(
        reg.instance_method_command_prefixes("struct::graph", "walk", &["root", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(3))],
    );

    // struct::tree `walkproc node … cmdprefix` — trailing positional prefix
    // (resolver), Exactly(3) (tree node action).  The prefix is the final
    // word regardless of intervening `-order`/`-type` options.
    assert_eq!(
        reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root", "cb"]),
        vec![(1, AppendedArity::Exactly(3))],
    );
    assert_eq!(
        reg.instance_method_command_prefixes(
            "struct::tree",
            "walkproc",
            &["root", "-type", "dfs", "cb"],
        ),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `walkproc node` with no prefix word yet names none.
    assert!(
        reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root"])
            .is_empty(),
        "a walkproc with only the node names no prefix",
    );
    // An unmodelled method / class resolves to nothing.
    assert!(
        reg.instance_method_command_prefixes("struct::graph", "get", &["x"])
            .is_empty(),
        "an unmodelled instance method has no command prefix",
    );
}

#[test]
fn tk_command_options_classified_prefix_vs_script() {
    // The Tk `script()→command_prefix` conversion (separate commit, highest
    // risk).  Locks in the classification: appended-arg callbacks are
    // prefixes (references / call-graph / W123 / arity); verbatim scripts and
    // percent-substitution callbacks stay `script()` (Body, not recorded as a
    // reference).  Ground truth: Tk 8.6/9.0 man pages.
    let reg = CommandRegistry::build_default();

    // PREFIXES — scroll callbacks append `first last` (2).
    for widget in ["listbox", "text", "canvas", "entry", "spinbox"] {
        assert_eq!(
            reg.command_prefixes(widget, &[".w", "-xscrollcommand", "cb"]),
            vec![(2, AppendedArity::Exactly(2))],
            "{widget} -xscrollcommand must be a prefix appending 2",
        );
    }
    // scale / ttk::scale append the new value (1).
    assert_eq!(
        reg.command_prefixes("scale", &[".s", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("ttk::scale", &[".s", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    // scrollbar -command: `moveto frac` (2) or `scroll n units` (3).
    assert_eq!(
        reg.command_prefixes("scrollbar", &[".sb", "-command", "cb"]),
        vec![(
            2,
            AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 3]))
        )],
    );
    // menu -tearoffcommand appends the parent + torn-off menu paths (2).
    assert_eq!(
        reg.command_prefixes("menu", &[".m", "-tearoffcommand", "cb"]),
        vec![(2, AppendedArity::Exactly(2))],
    );

    // NOT prefixes — verbatim action scripts and percent-substitution
    // callbacks are `script()`, never recorded as a command reference.
    for (widget, opt) in [
        ("button", "-command"),
        ("checkbutton", "-command"),
        ("radiobutton", "-command"),
        ("menu", "-command"),
        ("menu", "-postcommand"),
        ("ttk::combobox", "-postcommand"),
        ("spinbox", "-command"),         // percent-substitution (%W %s %d)
        ("spinbox", "-validatecommand"), // percent-substitution
        ("entry", "-validatecommand"),   // percent-substitution
        ("entry", "-invalidcommand"),    // percent-substitution
    ] {
        assert!(
            reg.command_prefixes(widget, &[".w", opt, "cb"]).is_empty(),
            "{widget} {opt} must stay a script, not a command prefix",
        );
    }
}

#[test]
fn commands_naming_a_cmdprefix_declare_a_command_prefix() {
    // Drift guard: any command whose synopsis literally names a `cmdprefix`
    // argument must declare a `CommandPrefix` (static table, resolver, or
    // command-prefix option) so callbacks light up references / call-graph /
    // W123 / arity.  The allowlist holds genuinely-deferred callbacks that
    // still need modelling; it is empty now that the option-value
    // callbacks (`mime::getbody -command`, …) carry `OptionSpec` arrays.
    const DEFERRED_OPTION_PREFIX: &[&str] = &[];
    let reg = CommandRegistry::build_default();
    let mut gaps = Vec::new();
    for name in reg.command_names() {
        if DEFERRED_OPTION_PREFIX.contains(&name) {
            continue;
        }
        let spec = reg.get(name).expect("registered");
        let mut synopses: Vec<&str> = Vec::new();
        if let Some(h) = &spec.hover {
            synopses.extend(h.synopsis.iter().copied());
        }
        synopses.extend(spec.forms.iter().map(|f| f.synopsis));
        synopses.extend(spec.subcommands.iter().map(|s| s.synopsis));
        for syn in synopses {
            if !syn.to_ascii_lowercase().contains("cmdprefix") {
                continue;
            }
            // Probe with the synopsis words after the command name; a
            // declared prefix yields a non-empty result.  Strip the `?…?`
            // optionality markers so an option-value prefix
            // (`?-command cmdprefix?`) presents its bare `-command` /
            // `cmdprefix` words to the option scanner.
            let args: Vec<&str> = syn
                .split_whitespace()
                .skip(1)
                .map(|w| w.trim_matches('?'))
                .collect();
            if reg.command_prefixes(name, &args).is_empty() {
                gaps.push(format!("{name}: {syn}"));
            }
        }
    }
    gaps.sort();
    assert!(
        gaps.is_empty(),
        "commands whose synopsis names a cmdprefix but declare no CommandPrefix:\n{}",
        gaps.join("\n"),
    );
}

#[test]
fn arg_indices_for_role_command_prefix_matches_command_prefixes() {
    // The delegation invariant: `arg_indices_for_role(CommandPrefix)` is
    // exactly the positions `command_prefixes` reports — so highlighting,
    // param-traits, and the call-reference extractor never drift.
    let reg = CommandRegistry::build_default();
    for (name, args) in [
        ("lsort", &["-command", "cmp", "{a b}"][..]),
        ("socket", &["-server", "accept", "9000"][..]),
        ("tcltest::customMatch", &["exact", "cmp"][..]),
        ("selection", &["handle", ".w", "getData"][..]),
        ("regsub", &["-command", "re", "s", "cb"][..]),
        ("namespace", &["unknown", "handler"][..]),
        ("package", &["unknown", "handler"][..]),
        ("coroinject", &["myCoro", "cb", "x"][..]),
    ] {
        let via_role = reg.arg_indices_for_role(name, args, ArgRole::CommandPrefix);
        let via_prefixes: Vec<usize> = reg
            .command_prefixes(name, args)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        assert_eq!(via_role, via_prefixes, "delegation mismatch for {name}");
    }
}

#[test]
fn namespace_name_option_carries_name_role() {
    // `interp invokehidden path -namespace ns cmd` — the `-namespace` value is a
    // symbolic (namespace) name: captured declaratively for a
    // Name query, never for Body/VarWrite (not recursed, not a var def).
    let reg = CommandRegistry::build_default();
    let args = ["invokehidden", "child", "-namespace", "ns", "cmd"];
    assert_eq!(
        reg.arg_indices_for_role("interp", &args, ArgRole::Name),
        vec![3],
        "the -namespace value should carry the Name role",
    );
    assert!(
        reg.arg_indices_for_role("interp", &args, ArgRole::Body)
            .is_empty()
            && reg
                .arg_indices_for_role("interp", &args, ArgRole::VarWrite)
                .is_empty(),
        "a name value must not be recursed or treated as a variable",
    );
}

#[test]
fn bind_script_form_recurses_only_the_trailing_script() {
    // `bind $w <KeyPress> {…}` binds a script: the third
    // argument is a deferred event-handler body and must be recursed for
    // highlighting.  The `bind tag` / `bind tag sequence` query forms carry
    // no script and must not surface a Body.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role("bind", &["$w", "<KeyPress>", "{…}"], ArgRole::Body),
        vec![2],
        "the trailing script of the three-argument form is a body",
    );
    // The `+script` append form is still the trailing argument.
    assert_eq!(
        reg.arg_indices_for_role("bind", &[".b", "<Enter>", "+{puts hi}"], ArgRole::Body),
        vec![2],
    );
    assert!(
        reg.arg_indices_for_role("bind", &["$w"], ArgRole::Body)
            .is_empty(),
        "the single-tag query form has no script",
    );
    assert!(
        reg.arg_indices_for_role("bind", &["$w", "<KeyPress>"], ArgRole::Body)
            .is_empty(),
        "the tag+sequence query form has no script",
    );
}

#[test]
fn wm_protocol_handler_is_a_body() {
    // `wm protocol . WM_DELETE_WINDOW {script}` registers a deferred
    // handler script as its third argument; the query forms carry none.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role(
            "wm",
            &["protocol", ".", "WM_DELETE_WINDOW", "{exit}"],
            ArgRole::Body,
        ),
        // subcommand path offsets by +1 for the `protocol` word.
        vec![3],
        "the wm protocol handler command is a script body",
    );
    assert!(
        reg.arg_indices_for_role("wm", &["protocol", ".", "WM_DELETE_WINDOW"], ArgRole::Body)
            .is_empty(),
        "the two-argument `wm protocol window name` query form has no script",
    );
    assert!(
        reg.arg_indices_for_role("wm", &["protocol", "."], ArgRole::Body)
            .is_empty(),
        "the one-argument `wm protocol window` query form has no script",
    );
}

#[test]
fn canvas_bind_subcommand_script_is_a_body() {
    let reg = callback_metadata_registry();
    for (arguments, expected) in [
        (&["bind", "item", "<Button>", "{p}"][..], vec![3]),
        (&["bind", "item", "<Button>"][..], Vec::new()),
    ] {
        let invocation = reg
            .resolve_instance_invocation("canvas", ".canvas", arguments, None)
            .expect("provided canvas instance method");
        let roles = invocation
            .argument_roles()
            .0
            .into_iter()
            .filter_map(|(at, role)| {
                (role == ArgRole::Body)
                    .then_some(usize::from(at) + invocation.semantics.argument_offset)
            })
            .collect::<Vec<_>>();
        assert_eq!(roles, expected, "{arguments:?}");
    }
}

#[test]
fn ttk_widgets_require_tk_8_5() {
    // ttk (themed Tk) widgets were introduced in Tk 8.5, so they must be
    // gated out when only an older Tk is guaranteed by `package require`.
    let reg = CommandRegistry::build_default();
    for name in [
        "ttk::button",
        "ttk::treeview",
        "ttk::notebook",
        "ttk::style",
    ] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            !spec.available_for_version(Some("8.4")),
            "{name} must not be available under Tk 8.4",
        );
        assert!(
            spec.available_for_version(Some("8.5")),
            "{name} must be available under Tk 8.5",
        );
        assert!(
            spec.available_for_version(None),
            "{name} must be permissive when no Tk version is pinned",
        );
    }
}

#[test]
fn text_tag_bind_script_is_a_body() {
    // `pathName tag bind tagName sequence script` binds a deferred
    // event-handler script as its trailing word.
    let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
    let roles = |arguments: &[&str]| {
        let invocation = reg
            .resolve_instance_invocation("text", ".t", arguments, reg.own_surface_query())
            .expect("provided text instance method");
        invocation
            .argument_roles()
            .0
            .into_iter()
            .filter_map(|(index, role)| {
                (role == ArgRole::Body)
                    .then_some(usize::from(index) + invocation.semantics.argument_offset)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(roles(&["tag", "bind", "sel", "<Key>", "{p}"]), vec![4]);
    assert_eq!(roles(&["tag", "add", "sel", "1.0", "end"]).len(), 0);
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert!(
        unprovided
            .resolve_instance_invocation(
                "text",
                ".t",
                &["tag", "bind", "sel", "<Key>", "{p}"],
                unprovided.own_surface_query()
            )
            .is_none()
    );
}

#[test]
fn selection_handle_command_is_a_command_prefix() {
    // `selection handle window command` — the trailing command is a
    // prefix Tk appends offset/maxChars to, not a recursed script body.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role(
            "selection",
            &["handle", ".w", "getData"],
            ArgRole::CommandPrefix
        ),
        vec![2],
        "the selection handle command prefix is captured",
    );
    assert!(
        reg.arg_indices_for_role("selection", &["handle", ".w", "getData"], ArgRole::Body)
            .is_empty(),
        "a command prefix is not recursed as a script body",
    );
    // With leading option/value pairs the command is still the last arg.
    assert_eq!(
        reg.arg_indices_for_role(
            "selection",
            &["handle", "-format", "STRING", ".w", "getData"],
            ArgRole::CommandPrefix,
        ),
        vec![4],
    );
}

#[test]
fn writes_first_arg_variable_membership() {
    let reg = CommandRegistry::build_default();
    // TP: the five first-arg writers.
    for cmd in ["set", "append", "lappend", "incr", "lset"] {
        assert!(reg.writes_first_arg_variable(cmd), "{cmd} writes arg 0");
    }
    // FP guards: `unset` destroys (not a write); value-taking and
    // unknown commands are out.
    for cmd in ["unset", "puts", "llength", "foreach", "nosuchcmd"] {
        assert!(!reg.writes_first_arg_variable(cmd), "{cmd} must not match");
    }
}

#[test]
fn rmw_first_arg_variable_membership() {
    let reg = CommandRegistry::build_default();
    // TP: read-modify-write commands fold the current value in.
    for cmd in ["append", "lappend", "incr", "lset"] {
        assert!(reg.rmw_first_arg_variable(cmd), "{cmd} is RMW");
    }
    // FP guards: a whole-value `set` is rename-safe; `unset` destroys.
    for cmd in ["set", "unset", "puts", "nosuchcmd"] {
        assert!(!reg.rmw_first_arg_variable(cmd), "{cmd} must not match");
    }
}

#[test]
fn commands_with_trait_query() {
    let reg = CommandRegistry::build_default();
    let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
    assert!(control_flow.contains(&"for"));
    assert!(control_flow.contains(&"if"));
    assert!(control_flow.contains(&"while"));
    assert!(!control_flow.contains(&"puts"));
}

#[test]
fn native_root_factory_descriptor_uses_actual_axes() {
    let older_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let mut registry = CommandRegistry::build_default();
    registry.set_profile(older_profile);
    for (version, admitted) in [
        (tcl_dialect::TclVersion::V8_4, false),
        (tcl_dialect::TclVersion::V8_6, true),
        (tcl_dialect::TclVersion::V9_0, true),
        (tcl_dialect::TclVersion::V9_1, true),
    ] {
        assert_eq!(
            registry
                .native_default_construction_grammar(
                    "oo::class",
                    crate::InvocationDialect::for_version(version),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_some(),
            admitted,
            "the editor's older profile cannot replace the actual factory release",
        );
    }
    assert!(registry.default_construction_grammar("oo::class").is_none());
    let physically_projected = CommandRegistry::build_default().project_for_profile(older_profile);
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        assert!(
            physically_projected
                .native_default_construction_grammar(
                    "oo::class",
                    crate::InvocationDialect::for_version(version),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_none(),
            "actual axes cannot restore metadata removed by physical projection"
        );
    }
    let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert!(
        registry
            .native_default_construction_grammar(
                "oo::class",
                jim,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .is_none()
    );
}

fn assert_effective_semantics_match_uncached_queries(registry: &CommandRegistry) {
    let index = registry.effective_semantics();
    let mut expected_bindings: BTreeSet<String> = registry
        .command_names()
        .map(crate::rooted_command_descriptor_label)
        .collect();
    // Authored private implementation slots are bindings even when they
    // have no public command catalogue row.
    if let Some(grammar) = registry.default_construction_grammar("oo::class") {
        for member in grammar.members {
            for receiver in [
                crate::definer::DefinitionReceiver::Instance,
                crate::definer::DefinitionReceiver::Class,
            ] {
                if let Some(lookup) = grammar.definition_member_lookup_for_receiver(
                    member,
                    receiver,
                    registry.own_surface_query(),
                ) {
                    expected_bindings.insert(lookup.implementation);
                }
            }
        }
    }
    if let Some(profile) = registry.profile() {
        expected_bindings.extend(
            registry
                .stock_native_implementation_slots(crate::InvocationDialect::of_profile(profile))
                .into_iter()
                .map(|lookup| lookup.slot.to_owned()),
        );
    }
    if let Some(profile) = registry.profile()
        && let Some(support) = registry
            .native_class_factory_recipe(
                "oo::configurable",
                crate::InvocationDialect::of_profile(profile),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
    {
        expected_bindings.extend(
            support
                .binding_names()
                .iter()
                .map(|name| (*name).to_owned()),
        );
    }
    let expected_handlers: BTreeSet<String> = registry
        .commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER)
        .into_iter()
        .map(crate::rooted_command_descriptor_label)
        .collect();
    assert_eq!(index.binding_names(), &expected_bindings);
    assert_eq!(index.unresolved_command_handlers(), &expected_handlers);

    for name in registry.command_names() {
        let expected = match registry.own_surface_query() {
            Some(query) => registry.get_for_surface(name, Some(query)),
            None => registry.get(name),
        };
        let actual = index.command(name);
        assert_eq!(
            actual.map(EffectiveCommandSemantics::traits),
            expected.map(|spec| spec.traits)
        );
        assert_eq!(
            actual.and_then(EffectiveCommandSemantics::lowering_hook),
            expected.and_then(|spec| spec.lowering_hook),
            "lowering hook mismatch for {name}"
        );
    }
}

#[test]
fn effective_semantics_cache_matches_default_and_exact_release_queries() {
    assert_effective_semantics_match_uncached_queries(&CommandRegistry::build_default());
    for dialect in ["tcl8.4", "tcl9.0"] {
        assert_effective_semantics_match_uncached_queries(
            crate::model::ingress::static_context_for(dialect).commands(),
        );
    }
}

#[test]
fn snapshot_and_ingress_share_the_lazy_semantic_index() {
    let registry = CommandRegistry::build_default();
    let snapshot = registry.snapshot();
    assert!(Arc::ptr_eq(
        &registry.effective_semantics(),
        &snapshot.registry().effective_semantics(),
    ));
}

#[test]
fn effective_semantics_cache_is_reused_and_invalidated_by_overlay() {
    let mut registry = CommandRegistry::build_default();
    let before = registry.effective_semantics();
    assert!(Arc::ptr_eq(&before, &registry.effective_semantics()));

    // `insert` is the same authored-overlay seam used by the SpecTcl
    // installer after it has specialised a pack command.
    let mut overlay = registry.get("puts").expect("puts spec").clone();
    overlay.traits |= Traits::UNRESOLVED_COMMAND_HANDLER | Traits::CATCHABLE_THROW;
    registry.insert(overlay);

    let after = registry.effective_semantics();
    assert!(!Arc::ptr_eq(&before, &after));
    assert!(after.unresolved_command_handlers().contains("::puts"));
    assert!(
        after
            .command("puts")
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    );
    assert_effective_semantics_match_uncached_queries(&registry);
}

#[test]
fn effective_semantics_cache_is_invalidated_by_profile_stamp() {
    let mut registry = CommandRegistry::build_default();
    let before = registry.effective_semantics();
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").expect("tcl8.4 profile");
    registry.set_profile(profile);

    let after = registry.effective_semantics();
    assert!(!Arc::ptr_eq(&before, &after));
    assert_eq!(after.command("throw"), None);
    assert_effective_semantics_match_uncached_queries(&registry);
}

#[test]
fn byte_compiled_covers_the_core_builtins() {
    let reg = CommandRegistry::build_default();
    // Every registered command the minifier must never alias.
    // The former built-in skip list (minus the
    // non-command keywords `else` / `elseif` and the unregistered
    // `pwd`).
    let expected = [
        "set",
        "unset",
        "proc",
        "if",
        "while",
        "for",
        "foreach",
        "switch",
        "return",
        "break",
        "continue",
        "expr",
        "catch",
        "try",
        "throw",
        "package",
        "namespace",
        "upvar",
        "uplevel",
        "variable",
        "global",
        "append",
        "lappend",
        "incr",
        "info",
        "string",
        "list",
        "llength",
        "lindex",
        "lrange",
        "lsort",
        "lsearch",
        "lreplace",
        "linsert",
        "dict",
        "array",
        "regexp",
        "regsub",
        "scan",
        "format",
        "open",
        "close",
        "read",
        "gets",
        "eof",
        "flush",
        "seek",
        "tell",
        "fconfigure",
        "fcopy",
        "fileevent",
        "socket",
        "after",
        "update",
        "vwait",
        "rename",
        "source",
        "eval",
        "apply",
        "tailcall",
        "error",
        "cd",
        "file",
        "glob",
        "clock",
        "binary",
        "encoding",
        "interp",
        "load",
        "exit",
        "pid",
        "exec",
        "chan",
        "puts",
    ];
    for name in expected {
        assert!(
            reg.is_byte_compiled(name),
            "{name} should carry Traits::BYTE_COMPILED"
        );
    }
    // A user-proc-like name and a command outside the curated
    // skip set must not carry the trait.
    assert!(!reg.is_byte_compiled("my_helper_proc"));
    assert!(!reg.is_byte_compiled("split"));
}

#[test]
fn not_proc_factory_covers_registered_skip_heads() {
    let reg = CommandRegistry::build_default();
    // Registered heads from the former `_FACTORY_SKIP_HEADS` list
    // (the four non-command heads method / classmethod /
    // itcl::class / ::itcl::class are handled by the scanner's
    // residual set, not the registry).
    let expected = [
        "proc",
        "namespace",
        "if",
        "switch",
        "while",
        "for",
        "foreach",
        "try",
        "catch",
        "eval",
        "apply",
        "expr",
        "uplevel",
        "upvar",
        "variable",
        "set",
        "lappend",
        "dict",
        "array",
        "string",
        "list",
        "lindex",
        "package",
        "source",
        "interp",
        "oo::class",
        "oo::define",
        "oo::objdefine",
    ];
    for name in expected {
        assert!(
            reg.is_not_proc_factory(name),
            "{name} should carry Traits::NOT_PROC_FACTORY"
        );
    }
    // A real factory-wrapper head must not be skipped.
    assert!(!reg.is_not_proc_factory("my_factory"));
}

#[test]
fn frameless_runtime_covers_the_audited_allow_list() {
    let reg = CommandRegistry::build_default();
    let got: std::collections::HashSet<&str> = reg
        .commands_with_trait(Traits::FRAMELESS_RUNTIME)
        .into_iter()
        .collect();
    let expected: std::collections::HashSet<&str> = [
        "list",
        "lindex",
        "lrange",
        "linsert",
        "llength",
        "lsort",
        "lsearch",
        "lappend",
        "lreverse",
        "lreplace",
        // Tcl 9.0+: shares `lreplace`'s core (see
        // `tcl-vm/src/cmd_list.rs::cmd_ledit`); a flat single-variable
        // read-modify-write with no eval fallback and no sublist-index
        // descent (unlike `lset`, which deliberately stays off this
        // list) — see `commands/tcl/ledit.rs`.
        "ledit",
        "lrepeat",
        "lassign",
        // Tcl 9.0+: a flat native dispatch over `tcl-cmd-core::lseq`
        // (`tcl-vm/src/cmd_lseq.rs::cmd_lseq`) that evaluates nothing —
        // see `commands/tcl/lseq.rs`.
        "lseq",
        "concat",
        "split",
        "join",
        "string",
        "format",
        "expr",
        "global",
        "variable",
        "upvar",
        "namespace",
        "set",
        "incr",
        "append",
        "unset",
        "puts",
        "return",
        "error",
        "continue",
        "break",
        // `throw type message` (Tcl 8.6+) — `cmd_throw`
        // (`tcl-vm/src/cmd_try.rs`) takes its two already-substituted
        // arguments, validates `type` as a non-empty Tcl list, and
        // builds its return-options dict directly with no eval
        // fallback of any kind (it never touches `vm`), structurally
        // identical to `cmd_error` in this respect. See
        // `commands/tcl/throw_.rs`.
        "throw",
        // `::tcl::unsupported::corotype coroName` (and its
        // namespace-relative `tcl::unsupported::corotype` spelling) — a
        // flat lookup into the coroutine table
        // (`tcl-vm/src/cmd_coro.rs::cmd_corotype`): no eval fallback, no
        // `Frame` of its own. Both spellings share the same `make_spec`
        // and so the same traits. See
        // `commands/tcl/tcl_unsupported_corotype.rs`.
        "::tcl::unsupported::corotype",
        "tcl::unsupported::corotype",
        // `::tcl::unsupported::representation value` — describes the
        // value it was handed and runs nothing. See
        // `commands/tcl/tcl_unsupported_representation.rs`.
        "::tcl::unsupported::representation",
        "tcl::unsupported::representation",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        got, expected,
        "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
    );
}

#[test]
fn dialect_filter() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(spec.is_some());
    // dict is tcl8.5+ so should NOT be available in 8.4
    let spec84 = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
    assert!(spec84.is_none());
}

#[test]
fn subcommand_arg_roles() {
    let reg = CommandRegistry::build_default();
    let bodies = reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
    // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
    assert!(bodies.contains(&3));
}

#[test]
fn variable_write_commands() {
    let reg = CommandRegistry::build_default();
    let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
    assert_eq!(set_vars, vec![0]);
}

#[test]
fn role_queries_share_selected_set_forms_with_owned_facts() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    for (args, role, expected) in [
        (vec!["x"], ArgRole::VarRead, vec![0]),
        (vec!["x"], ArgRole::VarWrite, vec![]),
        (vec!["x", "value"], ArgRole::VarRead, vec![]),
        (vec!["x", "value"], ArgRole::VarWrite, vec![0]),
    ] {
        let facts = registry
            .resolve_invocation("set", &args, registry.own_surface_query())
            .expect("native set")
            .facts();
        let positions: Vec<_> = facts
            .arg_roles
            .iter()
            .filter(|(_, found)| *found == role)
            .map(|(index, _)| facts.argument_offset + usize::from(*index))
            .collect();
        assert_eq!(positions, expected);
        assert_eq!(
            registry.arg_indices_for_role("::set", &args, role),
            expected
        );
    }
}

#[test]
fn source_role_queries_select_forms_without_requiring_operand_values() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    let read = [InvocationWord::Dynamic];
    let write = [InvocationWord::Dynamic, InvocationWord::Dynamic];
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&read),
            ArgRole::VarRead
        ),
        Some(vec![0])
    );
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&write),
            ArgRole::VarWrite
        ),
        Some(vec![0])
    );
    let expanded = [InvocationWord::Expanded];
    assert_eq!(
        registry.arg_indices_for_role_words(
            "set",
            InvocationArguments::structured(&expanded),
            ArgRole::VarWrite
        ),
        None
    );
}

#[test]
fn source_procedure_roles_do_not_require_native_definition_acceptance() {
    // naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    let registry = CommandRegistry::build_default();
    let values = [
        InvocationWord::Dynamic,
        InvocationWord::KnownBytes(b"\xff"),
        InvocationWord::Dynamic,
    ];
    let arguments = InvocationArguments::structured(&values);
    let selected = registry
        .resolve_structured_invocation(
            InvocationWords::from_arguments(InvocationWord::Literal("proc"), arguments),
            None,
        )
        .resolved()
        .unwrap();
    assert!(!selected.facts().arg_roles_complete);
    assert_eq!(selected.facts().arg_roles.len(), 0);
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            arguments,
            &[ArgRole::Name, ArgRole::ParamList, ArgRole::Body],
        ),
        Some(vec![
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body)
        ])
    );
    assert_eq!(
        registry.arg_indices_for_role("proc", &["p", "a b", "return"], ArgRole::Body,),
        vec![2]
    );
    let expanded = [InvocationWord::Expanded];
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            InvocationArguments::structured(&expanded),
            &[ArgRole::Body],
        ),
        None
    );
    let opaque = [
        InvocationWord::Dynamic,
        InvocationWord::Opaque,
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        registry.arg_role_assignments_words(
            "proc",
            InvocationArguments::structured(&opaque),
            &[ArgRole::Body],
        ),
        None
    );
}

#[test]
fn source_role_queries_retain_explicit_optional_frame_grammar() {
    let registry = CommandRegistry::build_default();
    let args = [
        InvocationWord::Literal("+0"),
        InvocationWord::Literal("remote"),
        InvocationWord::Literal("local"),
    ];
    let legacy = InvocationArguments::structured(&args).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
    );
    let modern = InvocationArguments::structured(&args).with_dialect(
        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        registry.arg_indices_for_role_words("upvar", legacy, ArgRole::VarWrite),
        Some(vec![])
    );
    assert_eq!(
        registry.arg_indices_for_role_words("upvar", modern, ArgRole::VarWrite),
        Some(vec![2])
    );
}

/// `unset x y z` names *every* argument as a variable, not
/// just the first, so all of them highlight as variables.
#[test]
fn unset_marks_every_name() {
    let reg = CommandRegistry::build_default();
    let vars = reg.arg_indices_for_role("unset", &["x", "y", "z"], ArgRole::VarWrite);
    assert_eq!(vars, vec![0, 1, 2]);
}

/// `arg_indices_evaluated_in_frame` is the call-site half of
/// `ArgRole::braced_word_evaluated_in_frame`: it names the positions whose
/// brace-quoted word the callee still substitutes against the *caller's*
/// variables. tclsh 9.0.4: `set x 5; expr {$x} + 1` → `6`, while
/// `lindex {$x} 0` → the two characters `$x` and
/// `apply {{} {puts $x}}` errors with `can't read "x"` (a fresh frame).
#[test]
fn arg_indices_evaluated_in_frame_names_only_caller_frame_code_words() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("expr", &["$x", "+", "1"]),
        vec![0],
    );
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("lindex", &["$x", "0"])
            .len(),
        0
    );
    assert_eq!(
        reg.arg_indices_evaluated_in_frame("apply", &["{{} {puts $x}}"])
            .len(),
        0
    );
}

/// `unset -nocomplain -- a b` skips the leading options and names only the
/// real variables (`a`, `b`), mirroring `lower_unset`.
#[test]
fn unset_skips_leading_options() {
    let reg = CommandRegistry::build_default();
    let vars =
        reg.arg_indices_for_role("unset", &["-nocomplain", "--", "a", "b"], ArgRole::VarWrite);
    assert_eq!(vars, vec![2, 3]);
}

/// `unset` recognises only `-nocomplain` / `--` as options — a dash-prefixed
/// word like `-foo` is a real variable name (verified against tclsh), so it
/// keeps its `VarWrite` role.
#[test]
fn unset_dash_name_is_a_variable() {
    let reg = CommandRegistry::build_default();
    // `unset -foo bar` — both are variables (no `--` needed to reach them).
    assert_eq!(
        reg.arg_indices_for_role("unset", &["-foo", "bar"], ArgRole::VarWrite),
        vec![0, 1]
    );
    // `unset -nocomplain -foo` — `-nocomplain` is skipped, `-foo` is a name.
    assert_eq!(
        reg.arg_indices_for_role("unset", &["-nocomplain", "-foo"], ArgRole::VarWrite),
        vec![1]
    );
}

#[test]
fn arg_indices_for_role_trace_add_variable_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["add", "variable", "x", "write", "body"],
        ArgRole::VarWrite,
    );
    // +1 for subcommand offset.  arg "x" is at idx 2 in the
    // full args list (sub args[1] + 1).
    assert!(writes.contains(&2), "VarWrite writes={writes:?}");
}

/// `trace add execution` does NOT declare `VarWrite`
/// (the second arg is a command name, not a variable).
#[test]
fn arg_indices_for_role_trace_add_execution_no_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["add", "execution", "foo", "enter", "body"],
        ArgRole::VarWrite,
    );
    assert!(writes.is_empty(), "VarWrite writes={writes:?}");
}

/// `trace add command`/`execution` trace a command *by name*, so the name
/// argument is a `CommandName` reference (navigation follows it), while a
/// `variable` trace's name is not.
#[test]
fn arg_indices_for_role_trace_add_command_name_reference() {
    let reg = CommandRegistry::build_default();
    for kind in ["command", "execution"] {
        let names = reg.arg_indices_for_role(
            "trace",
            &["add", kind, "foo", "enter", "body"],
            ArgRole::CommandName,
        );
        // +1 subcommand offset: `foo` is at full-args index 2.
        assert!(names.contains(&2), "{kind}: CommandName names={names:?}");
    }
    let var = reg.arg_indices_for_role(
        "trace",
        &["add", "variable", "x", "write", "body"],
        ArgRole::CommandName,
    );
    assert!(var.is_empty(), "a variable trace names no command: {var:?}");
}

/// `trace remove variable` behaves like `trace add variable`:
/// both alias spellings flow through the same `VarWrite` query.
#[test]
fn arg_indices_for_role_trace_remove_variable_var_write() {
    let reg = CommandRegistry::build_default();
    let writes = reg.arg_indices_for_role(
        "trace",
        &["remove", "variable", "y", "write", "body"],
        ArgRole::VarWrite,
    );
    assert!(writes.contains(&2), "VarWrite writes={writes:?}");
}

/// `global` / `variable` / `upvar` carry
/// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
/// per-arg list belongs to `var_scoping`, not the role-driven
/// `VarWrite` query.
#[test]
fn creates_dynamic_barrier_trait_marks_scope_aliases() {
    let reg = CommandRegistry::build_default();
    for cmd in &["global", "variable", "upvar"] {
        assert!(
            reg.get(cmd)
                .unwrap()
                .traits
                .contains(Traits::CREATES_DYNAMIC_BARRIER),
            "{cmd} should carry CREATES_DYNAMIC_BARRIER",
        );
    }
    // `set` does NOT carry the trait — its VarWrite at arg 0 is
    // a single-target def, not a vararg list.
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::CREATES_DYNAMIC_BARRIER)
    );
}

/// `dict with` / `dict update` arg 0 (the dict variable) plays
/// both `VarRead` and `VarWrite` roles. This is emitted via
/// duplicate `(idx, role)` rows in the resolver (the multi-role
/// observable behaviour is what consumers query).
/// `DEFERS_BODY` marks the commands that **store** a body rather than
/// run it, and it is the fail-safe direction that matters: a consumer
/// asking "can an unreadable body word cost me the proof that this call
/// completes?" must get `yes` for everything that has not declared
/// otherwise.
///
/// `BodyKind` cannot answer this — `proc` and `uplevel` are both
/// `Structural`, because that descriptor answers *which frame*, not
/// *when* — and neither can the absence of `control_arm_semantics`, which
/// `uplevel` / `oo::define` share with `proc`. `apply`'s lambda position
/// is a typed frame boundary; it still declares no `DEFERS_BODY`,
/// which is what this test checks.
#[test]
fn defers_body_marks_only_stored_bodies() {
    use crate::body_kind::BodyKind;
    let mut reg = CommandRegistry::build_default();
    assert!(
        reg.get("proc")
            .unwrap()
            .traits
            .contains(Traits::DEFERS_BODY),
        "`proc` stores its body for a later call"
    );
    // Every command that runs its script as part of the same call — all
    // of which lack control-arm semantics exactly as `proc` does.
    for name in ["uplevel", "apply", "oo::define", "oo::objdefine", "eval"] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} is known"));
        assert!(
            !spec.traits.contains(Traits::DEFERS_BODY),
            "{name} executes its script argument at this call"
        );
    }
    // The trap the flag exists to close: same `BodyKind`, opposite timing.
    assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
    assert_eq!(reg.get("uplevel").unwrap().body_kind, BodyKind::Structural);
    // An iRules handler registration stores its body the same way.
    reg.load_irules();
    assert!(
        reg.get("when")
            .unwrap()
            .traits
            .contains(Traits::DEFERS_BODY)
    );
    // The flag is opt-in, so the silent default is the abstaining one.
    assert!(
        !CommandSpec::DEFAULT.traits.contains(Traits::DEFERS_BODY),
        "an undeclared command must never read as dormant"
    );
    assert!(
        !SubCommand::DEFAULT.traits.contains(Traits::DEFERS_BODY),
        "an undeclared subcommand must never read as dormant"
    );
}

/// `apply`'s lambda position is a typed **frame boundary**: the script
/// runs as part of this call, in a fresh procedure frame.
///
/// The declaration is what lets a consumer walking a statement stream
/// treat an unreadable `apply $lambda` as the missing answer it is — the
/// role alone never reached the question, so the walk claimed past a
/// lambda that can raise.
///
/// Gated on the role, not on the argument number: only the
/// [`ArgRole::LambdaLiteral`] word is the script. `apply`'s trailing
/// `arg1 arg2 …` are ordinary values bound to the lambda's parameters.
#[test]
fn apply_types_its_lambda_position_as_a_frame_boundary() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.control_arm_semantics("apply", &["{{} {puts hi}}"], 0),
        Some(ControlArmSemantics::FrameBoundary),
        "`apply`'s lambda runs now, in a fresh frame"
    );
    for index in 1..4 {
        assert_eq!(
            reg.control_arm_semantics("apply", &["{{a b} {puts hi}}", "1", "2"], index),
            None,
            "argument {index} is a value bound to a parameter, not a script"
        );
    }
    // The role is what the declaration keys on, so it survives a rename of
    // the position and answers for any future command sharing the shape.
    assert_eq!(
        reg.arg_indices_for_role("apply", &["{{} {puts hi}}"], ArgRole::LambdaLiteral),
        vec![0]
    );
}

/// Every shipped command carrying [`ArgRole::LambdaLiteral`] types that
/// position, so the analyser's "read the body only where the registry says
/// what running it means" gate is never the thing deciding a shipped
/// command's fate.
///
/// Pinned rather than assumed: `apply` is the only such
/// command today, so dropping that gate would change nothing here — but a
/// pack, or a future built-in, can carry the role without typing it, and
/// then the gate is what keeps an untyped script that runs *now* from
/// being walked into and claimed. This test says out loud that the shipped
/// set does not exercise it.
#[test]
fn every_shipped_lambda_position_is_typed() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
    let mut seen = 0usize;
    for name in names {
        for argc in 1..=4 {
            let args = vec!["{{} {puts hi}}"; argc];
            for index in reg.arg_indices_for_role(&name, &args, ArgRole::LambdaLiteral) {
                seen += 1;
                assert!(
                    reg.control_arm_semantics(&name, &args, index).is_some(),
                    "{name}/{argc} carries a lambda at {index} without typing what running \
                         it means"
                );
            }
        }
    }
    assert!(seen > 0, "the sweep must not be vacuous");
}

/// Every command that carries an untyped [`ArgRole::Body`] position and
/// runs it *now*, pinned by name.
///
/// The analyser reads the absence of `DEFERS_BODY` as "this body runs as
/// part of this invocation" and lets such a body stop
/// its fall-through walk. That makes a *missing* `DEFERS_BODY` a silent
/// wrong answer rather than a missing one, so the inventory is pinned:
/// adding a Body-role command fails this test until its author puts it on
/// one of the two lists below, which is the moment to ask the question.
///
/// Store-only forms do not appear here — they carry `DEFERS_BODY` and the
/// sweep skips them. Commands the registry *types* do not appear either:
/// a typed arm is answered by [`ControlArmSemantics`], not by this trait.
///
/// Every entry was measured against tclsh 8.6.16 and 9.0.4, which agreed
/// byte-for-byte, with the shape `proc p {} { FORM {error stop}; set
/// ::reached 1 }`: `::reached` unset means the body ran here.
const BODY_RUNS_NOW: &[&str] = &[
    // Core Tcl: script evaluators and definition scripts.
    "::tcl::dict::for",
    "::tcl::dict::map",
    "::tcl::dict::update",
    "::tcl::dict::with",
    "eval",
    "oo::define",
    "oo::objdefine",
    "time",
    "timerate",
    "uplevel",
    // Runs its body now, but *absorbs* its completion — the harness
    // catches a failing test body and reports it. That is a completion
    // boundary, which is a typed answer this trait cannot give, so it
    // sits here and the walk over-abstains on it. Tracked separately.
    "tcltest::test",
    // tcllib: iterators and definition scripts. `clay::define` measured
    // with tcllib 2.0 clay on tclsh 8.6.18 and 9.0.4.
    "clay::define",
    "control::do",
    "fileutil::foreachLine",
    "lfilter",
    "snit::compile",
    "snit::type",
    "snit::widget",
    "snit::widgetadaptor",
    "stooop::class",
    "uri::register",
    // itcl: `class` runs its definition script; `body` and `configbody`
    // install a body against an already-declared member and look like
    // store-only forms, but no itcl is installable in this environment,
    // so they are recorded as unmeasured rather than marked on a guess.
    "itcl::body",
    "itcl::class",
    "itcl::configbody",
    // iRules: these run the script now, in a different flow context.
    "clientside",
    "peer",
    "serverside",
];

/// The gate for [`BODY_RUNS_NOW`]: no command may carry an untyped
/// `ArgRole::Body` position without an explicit stance on whether that
/// body runs now or is stored.
#[test]
fn every_untyped_body_role_declares_whether_its_body_runs_now() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
    let mut unclassified: Vec<String> = Vec::new();
    let mut seen_runs_now = 0usize;
    for name in &names {
        // Role resolution is argv-shaped, so probe the argument counts a
        // grammar could take, and the leading words that select the
        // script-bearing subforms of the ensembles in this sweep. Both
        // questions are asked per shape, exactly as the analyser asks
        // them: `DEFERS_BODY` can live on a subcommand rather than the
        // command (`package ifneeded`), so a spec-level test would miss it.
        let mut untyped_body = false;
        for argc in 1..=6usize {
            for lead in ["x", "idle", "ifneeded", "0"] {
                let mut args = vec!["x"; argc];
                args[0] = lead;
                // One name can carry more than one spec, resolved by
                // dialect — `after` has a Tcl spec and an iRules one, and
                // a sweep that looked at only one would let the other lose
                // its trait unnoticed. Iterating the specs (rather than a
                // list of points) also keeps every probe at a point the
                // command actually resolves at: an unresolvable call
                // answers with an empty trait set, which is indistinguish-
                // able from a spec whose only trait was `DEFERS_BODY`
                // until it was dropped — precisely the regression this
                // gate exists to catch.
                for spec in reg.specs(name) {
                    // Probe at a point the spec actually resolves under:
                    // its own first row, or any Tcl release when it names
                    // no provider.
                    let point = spec.surface.and_then(|rows| rows.first()).map_or(
                        SurfaceQuery::any_release(Family::Tcl),
                        |row| match row.provider {
                            SpecProvider::Core(family) => SurfaceQuery::any_release(family),
                            SpecProvider::Package(_) => SurfaceQuery::any_release(Family::Tcl),
                        },
                    );
                    let traits = reg.invocation_traits(name, &args, Some(point));
                    if spec.traits.contains(Traits::DEFERS_BODY)
                            || traits.contains(Traits::DEFERS_BODY)
                            // A `CONTROL_FLOW` command is the registry's own
                            // business: the analyser refuses to walk one whose
                            // arm it cannot type rather than reading the body,
                            // so a malformed probe shape of `if` / `try` is
                            // not a missing stance.
                            || traits.contains(Traits::CONTROL_FLOW)
                    {
                        continue;
                    }
                    let Some(invocation) = reg.resolve_invocation(name, &args, Some(point)) else {
                        continue;
                    };
                    for index in invocation
                        .argument_roles()
                        .0
                        .into_iter()
                        .filter(|(_, role)| *role == ArgRole::Body)
                        .map(|(position, _)| {
                            invocation.semantics.argument_offset + usize::from(position)
                        })
                    {
                        if reg.control_arm_semantics(name, &args, index).is_none()
                            && !reg
                                .get_for_surface(name, Some(point))
                                .is_some_and(|selected| {
                                    has_authored_body_timing(selected, &invocation, &args, index)
                                })
                        {
                            untyped_body = true;
                        }
                    }
                }
            }
        }
        if !untyped_body {
            continue;
        }
        if BODY_RUNS_NOW.contains(&name.as_str()) {
            seen_runs_now += 1;
        } else {
            unclassified.push(name.clone());
        }
    }
    assert!(
        seen_runs_now > 0,
        "the sweep must not be vacuous — no runs-now command was reached"
    );
    assert!(
        unclassified.is_empty(),
        "these commands carry an untyped ArgRole::Body and declare no \
             stance: supply exact argument-sensitive script timing, store the body (give the spec \
             Traits::DEFERS_BODY, with an oracle row) or they run it now (add \
             them to BODY_RUNS_NOW, with an oracle row). Leaving them out \
             makes the analyser's fall-through walk treat their body as one \
             that can stop control — see issue #1672. Unclassified: {unclassified:?}"
    );
}

fn has_authored_body_timing(
    spec: &CommandSpec,
    invocation: &ResolvedInvocation<'_, '_>,
    arguments: &[&str],
    index: usize,
) -> bool {
    let (resolver, offset) = invocation
        .subcommand
        .resolved()
        .and_then(|selected| spec.subcommand(selected.canonical_name))
        .map_or((spec.script_timing_resolver, 0), |sub| {
            (sub.script_timing_resolver, 1)
        });
    resolver.is_some_and(|resolve| {
        resolve(crate::InvocationArguments::literals(
            arguments.get(offset..).unwrap_or(&[]),
        ))
        .iter()
        .any(|(position, _)| offset + usize::from(*position) == index)
    })
}

/// `DEFERS_BODY` and typed control-arm semantics are answers to different
/// questions, and today no command gives both: everything the registry
/// types (`if`, `switch`, `namespace eval`, `catch`, `try`, the loops,
/// `apply`'s lambda)
/// runs the arm it types.
///
/// That is *why* a consumer cannot currently observe which of the two
/// checks fires first — dropping either leaves every shipped command
/// classified the same. It is a property of today's data, not of the
/// question, so it is pinned rather than assumed: a future command that
/// stored a body the registry also typed as an arm would make the order
/// load-bearing, and this is the assertion that says so out loud before
/// the analyser silently starts trusting a dormant body's arm.
#[test]
fn no_command_both_defers_a_body_and_types_it_as_a_control_arm() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let deferring: Vec<String> = reg
        .command_names()
        .filter(|name| {
            reg.get(name)
                .is_some_and(|spec| spec.traits.contains(Traits::DEFERS_BODY))
        })
        .map(str::to_owned)
        .collect();
    assert!(!deferring.is_empty(), "the sweep must not be vacuous");
    // Positive control: the probe below has to be able to *see* a typed
    // arm, or the sweep proves nothing. Role resolution is argv-shaped, so
    // an empty argument list makes every command answer `None` — including
    // `if`. Probe each deferring command across the argument counts its
    // grammar could take instead.
    assert_eq!(
        reg.control_arm_semantics("if", &["{$c}", "{body}"], 1),
        Some(ControlArmSemantics::Selected),
        "the probe must detect a typed arm, else the sweep is inert"
    );
    for name in deferring {
        for argc in 1..=6 {
            let args = vec!["x"; argc];
            for index in 0..argc {
                assert_eq!(
                    reg.control_arm_semantics(&name, &args, index),
                    None,
                    "{name}/{argc} declares DEFERS_BODY but types argument {index} as a \
                         control arm; `unreadable_body_is_material`'s check order now decides \
                         its behaviour"
                );
            }
        }
    }
}

/// Every spec defaults to `BodyKind::Plain` unless it
/// opts into `Structural`.
#[test]
fn body_kind_default_plain() {
    use crate::body_kind::BodyKind;
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
    assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
}

/// `proc` / `oo::class` / `oo::define` / `oo::objdefine`
/// stamp `Structural` so SSA skips their body args from the
/// enclosing block's data flow.
#[test]
fn body_kind_structural_marks() {
    use crate::body_kind::BodyKind;
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
    assert_eq!(
        reg.get("oo::class").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("oo::define").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("oo::objdefine").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("snit::method").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("snit::typemethod").unwrap().body_kind,
        BodyKind::Structural
    );
    assert_eq!(
        reg.get("uri::register").unwrap().body_kind,
        BodyKind::Structural
    );
}

/// iRules `when` event handler bodies are structural.
#[test]
fn body_kind_irules_when_structural() {
    use crate::body_kind::BodyKind;
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
}

/// `plain_body_arg_indices` surfaces the same-frame control-flow /
/// `eval` bodies a nested dispatch (e.g. `TclOO` `my method`) still
/// executes inside — every `Plain`-kind `ArgRole::Body` argument.
#[test]
fn plain_body_arg_indices_covers_same_frame_bodies() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.plain_body_arg_indices("if", &["1", "{body}"]), vec![1]);
    assert_eq!(
        reg.plain_body_arg_indices("while", &["1", "{body}"]),
        vec![1]
    );
    assert_eq!(
        reg.plain_body_arg_indices("foreach", &["v", "$list", "{body}"]),
        vec![2]
    );
    assert_eq!(reg.plain_body_arg_indices("eval", &["{body}"]), vec![0]);
    assert_eq!(
        reg.plain_body_arg_indices("catch", &["{body}", "res"]),
        vec![0]
    );
}

/// `Structural` bodies (`proc`, `oo::class create`, `uplevel`,
/// `namespace eval`) never come back from `plain_body_arg_indices` —
/// they run in a definition / different-frame context, so a dispatch
/// found inside is not still the caller's scope.
#[test]
fn plain_body_arg_indices_excludes_structural_bodies() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.plain_body_arg_indices("proc", &["name", "args", "{body}"]),
        Vec::<usize>::new()
    );
    assert_eq!(
        reg.plain_body_arg_indices("uplevel", &["1", "{body}"]),
        Vec::<usize>::new()
    );
    assert_eq!(
        reg.plain_body_arg_indices("namespace", &["eval", "ns", "{body}"]),
        Vec::<usize>::new()
    );
}

/// An unknown command name (a user proc, a `TclOO` method body's own
/// `unknownProc arg` call) has no registry spec at all — returns empty
/// rather than panicking.
#[test]
fn plain_body_arg_indices_unknown_command_is_empty() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.plain_body_arg_indices("myUserProc", &["a", "b"]),
        Vec::<usize>::new()
    );
}

/// `body_arg_implicit_args` defaults to 0 and is set on
/// `fileutil::updateInPlace` (which appends file contents to
/// the body's first command at runtime).
#[test]
fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
    let reg = CommandRegistry::build_default();
    assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
    assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
    assert_eq!(
        reg.get("fileutil::updateInPlace")
            .unwrap()
            .body_arg_implicit_args,
        1,
    );
}

#[test]
fn procedure_definition_words_place_the_three_words_of_tcl_proc() {
    assert_eq!(
        CommandRegistry::build_default()
            .procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
        None,
        "catalogue presence supplies no native formal-argument grammar"
    );
    let reg = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert_eq!(
        reg.procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
        Some(ProcedureWords {
            name: 0,
            params: 1,
            statics: None,
            body: 2,
        })
    );
}

#[test]
fn procedure_definition_words_answers_only_for_a_procedure_definer() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.procedure_definition_words("set", &["x", "1", "2"]),
        None
    );
    assert_eq!(reg.procedure_definition_words("proc", &["f"]), None);
    assert_eq!(reg.procedure_definition_words("no_such_head", &[]), None);
}

/// The providers of a name across the compiled-in universe: Expect's
/// `system` is offered by that package alone, `dict` by a core Tcl
/// window, a name nothing defines by nobody.
#[test]
fn providers_in_any_dialect_names_who_offers_a_command() {
    let reg = CommandRegistry::build_default();
    let system = reg
        .providers_in_any_dialect("system")
        .expect("system is Expect's");
    assert!(!system.unrestricted);
    assert_eq!(system.providers, [SpecProvider::Package("expect")]);

    let dict = reg.providers_in_any_dialect("dict").expect("dict is Tcl's");
    assert!(
        dict.providers.contains(&SpecProvider::Core(Family::Tcl)),
        "{dict:?}"
    );

    assert!(
        reg.providers_in_any_dialect("no_such_command_anywhere")
            .is_none()
    );
    assert_eq!(
        reg.providers_in_any_dialect("::dict"),
        reg.providers_in_any_dialect("dict"),
        "a rooted spelling names the same providers"
    );
}

#[test]
fn arg_indices_for_role_dict_with_multirole() {
    let reg = CommandRegistry::build_default();
    let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
    let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
    assert!(reads.contains(&1), "VarRead reads={reads:?}");
    assert!(writes.contains(&1), "VarWrite writes={writes:?}");
}

#[test]
fn arg_indices_for_role_dict_update_multirole() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let reg = CommandRegistry::build_default();
    let reads = reg.arg_indices_for_role(
        "dict",
        &["update", "$var", "k", "vname", "body"],
        ArgRole::VarRead,
    );
    let writes = reg.arg_indices_for_role(
        "dict",
        &["update", "$var", "k", "vname", "body"],
        ArgRole::VarWrite,
    );
    assert!(reads.contains(&1), "VarRead reads={reads:?}");
    assert!(writes.contains(&1), "VarWrite writes={writes:?}");
}

#[test]
fn dynamic_arg_role_resolution() {
    let reg = CommandRegistry::build_default();
    // if expr body elseif expr body else body
    let roles = reg.arg_indices_for_role(
        "if",
        &[
            "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
        ],
        ArgRole::Body,
    );
    // Bodies are at positions 2, 5, 7
    assert!(roles.contains(&2));
    assert!(roles.contains(&5));
    assert!(roles.contains(&7));
}

#[test]
fn load_irules_dialect() {
    let mut reg = CommandRegistry::build_default();
    assert!(reg.get("HTTP::header").is_none()); // not loaded yet
    reg.load_irules();
    assert!(reg.get("HTTP::header").is_some());
    assert!(reg.len() > 200); // should have 1000+ commands now
}

#[test]
fn irules_command_has_irules_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let spec = reg.get("HTTP::header").unwrap();
    assert_eq!(spec.surface, Some(SpecSurface::IRULES));
}

#[test]
fn irules_idempotent_load() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let count1 = reg.len();
    reg.load_irules(); // second load should be no-op
    assert_eq!(reg.len(), count1);
}

#[test]
fn default_includes_stdlib_and_tcllib() {
    let reg = CommandRegistry::build_default();
    // stdlib and tcllib are loaded by default
    assert!(reg.len() > 200);
}

#[test]
fn load_tk_dialect() {
    // Tk is part of the always-known base registry now, so it is present
    // by default and a later `load_surface(Tk)` is an idempotent no-op.
    let reg = CommandRegistry::build_default();
    assert!(
        reg.get("grid").is_some(),
        "Tk commands are loaded by default"
    );
    let base_count = reg.len();
    let mut reg2 = CommandRegistry::build_default();
    reg2.load_surface(SurfaceLayer::Package("Tk"));
    assert_eq!(
        reg2.len(),
        base_count,
        "loading the Tk surface is a no-op after the default load"
    );
}

#[test]
fn load_iapps_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Package("iapps"));
    assert!(reg.len() > 100);
}

#[test]
fn load_expect_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Package("expect"));
    assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
}

#[test]
fn resolve_call_unknown_command_returns_none() {
    let reg = CommandRegistry::build_default();
    assert!(reg.resolve_call("no_such_cmd", &[], None).is_none());
}

#[test]
fn resolve_invocation_retains_words_and_canonicalises_the_registry_name() {
    let reg = CommandRegistry::build_default();
    let args = ["create", "key", "value"];
    let resolved = reg
        .resolve_invocation(
            "::dict",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("global dict spelling resolves through the registry");

    assert_eq!(resolved.words.head_literal(), Some("::dict"));
    assert_eq!(
        resolved.words.arguments(),
        InvocationArguments::literals(&args)
    );
    assert_eq!(resolved.canonical_command, "dict");
    let sub = resolved
        .subcommand
        .resolved()
        .expect("dict create subcommand");
    assert_eq!(sub.spelling, "create");
    assert_eq!(sub.canonical_name, "create");
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Exact)
    );
    assert_eq!(resolved.semantics.argument_offset, 1);
}

#[test]
fn structured_resolution_keeps_dynamic_subcommands_indeterminate() {
    let reg = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("text"),
    ];
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Literal("string"), &arguments),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .expect("a literal command head is registry-known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Indeterminate {
            word_kind: crate::InvocationWordKind::Dynamic
        }
    ));
    assert!(resolved.form.is_none());
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::Invoke,
        "a dynamic subcommand cannot select specialised subcommand metadata"
    );
}

#[test]
fn expanded_argument_does_not_match_an_arity_form() {
    let reg = CommandRegistry::build_default();
    let arguments = [crate::InvocationWord::Expanded];
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Literal("incr"), &arguments),
            None,
        )
        .resolved()
        .expect("a literal command head is registry-known");

    assert!(resolved.form.is_none());
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr),
        "the command-level common operation remains visible without claiming a form"
    );
}

#[test]
fn variable_effect_footprints_keep_actual_availability_over_argument_dialect() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This is authored metadata, not a Native read/store or command result.
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "gated-write",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("set").unwrap().clone()
    });
    let store = std::sync::Arc::new(registry);
    let arguments = [
        crate::InvocationWord::Literal("$scalar(open"),
        crate::InvocationWord::Literal("VALUE"),
    ];
    let words = crate::InvocationWords::structured(
        crate::InvocationWord::Literal("gated-write"),
        &arguments,
    )
    .with_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
        let context = crate::model::ingress::static_context_for(environment)
            .with_command_store(std::sync::Arc::clone(&store));
        let projection = store.variable_write_projection_in_resolved_context(
            context.context(),
            words,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert_eq!(
            projection
                .literal_names
                .contains(&"$scalar(open".to_owned()),
            available,
            "{environment}"
        );
        assert_eq!(
            projection.opaque_variable_frame, !available,
            "{environment}"
        );
    }
}

#[test]
fn variable_write_projection_keeps_repeated_literal_targets() {
    let reg = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Literal("{(a)(b)}"),
        crate::InvocationWord::Literal("ab"),
        crate::InvocationWord::Literal("whole"),
        crate::InvocationWord::Literal("first"),
        crate::InvocationWord::Literal("second"),
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("regexp"),
        &arguments,
    ));

    assert_eq!(projection.literal_names, ["whole", "first", "second"]);
    assert!(!projection.opaque_variable_frame);
}

#[test]
fn variable_write_projection_roots_registry_global_option_targets() {
    let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
    let arguments = [
        crate::InvocationWord::Literal(".e"),
        crate::InvocationWord::Literal("-textvariable"),
        crate::InvocationWord::Literal("value"),
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("entry"),
        &arguments,
    ));

    assert_eq!(projection.literal_names, ["::value"]);
    assert!(!projection.opaque_variable_frame);
    assert!(
        CommandRegistry::build_default()
            .variable_write_projection(InvocationWords::structured(
                crate::InvocationWord::Literal("entry"),
                &arguments,
            ))
            .literal_names
            .is_empty(),
        "an unprovided catalogue widget cannot establish its option targets"
    );
}

#[test]
fn variable_write_projection_respects_registry_write_arity_floor() {
    let reg = CommandRegistry::build_default();
    for (head, arguments) in [
        (
            "::regexp",
            vec![
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
            ],
        ),
        (
            "::regsub",
            vec![
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Dynamic,
            ],
        ),
    ] {
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal(head),
            &arguments,
        ));
        assert_eq!(projection, VariableWriteProjection::default(), "{head}");
    }

    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("::regexp"),
        &arguments,
    ));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(projection.opaque_variable_frame);
}

#[test]
fn variable_write_projection_marks_dynamic_targets_and_argv_opaque() {
    let reg = CommandRegistry::build_default();
    for arguments in [
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("value"),
        ],
        vec![crate::InvocationWord::Expanded],
    ] {
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("set"),
            &arguments,
        ));
        assert_eq!(projection.literal_names.len(), 0);
        assert!(projection.opaque_variable_frame);
    }
}

#[test]
fn variable_write_projection_marks_computed_heads_opaque() {
    let reg = CommandRegistry::build_default();
    let no_arguments = [];
    for head in [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Expanded,
        crate::InvocationWord::Opaque,
    ] {
        let projection =
            reg.variable_write_projection(InvocationWords::structured(head, &no_arguments));
        assert_eq!(projection.literal_names.len(), 0);
        assert!(projection.opaque_variable_frame);
    }
}

#[test]
fn variable_write_projection_distinguishes_aliases_from_value_writes() {
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for (head, arguments) in [
            ("global", vec![crate::InvocationWord::Literal("item")]),
            (
                "upvar",
                vec![
                    crate::InvocationWord::Literal("1"),
                    crate::InvocationWord::Literal("outer"),
                    crate::InvocationWord::Literal("local"),
                ],
            ),
            (
                "trace",
                vec![
                    crate::InvocationWord::Literal("add"),
                    crate::InvocationWord::Literal("variable"),
                    crate::InvocationWord::Literal("item"),
                    crate::InvocationWord::Literal("write"),
                    crate::InvocationWord::Literal("callback"),
                ],
            ),
        ] {
            let projection = reg.variable_write_projection(
                InvocationWords::structured(crate::InvocationWord::Literal(head), &arguments)
                    .with_dialect(dialect),
            );
            assert_eq!(projection, VariableWriteProjection::default(), "{head}");
        }

        let arguments = [
            crate::InvocationWord::Literal("first"),
            crate::InvocationWord::Literal("value"),
            crate::InvocationWord::Literal("declared_only"),
        ];
        let projection = reg.variable_write_projection(
            InvocationWords::structured(crate::InvocationWord::Literal("variable"), &arguments)
                .with_dialect(dialect),
        );
        assert_eq!(projection.literal_names, ["first"]);
        assert!(!projection.opaque_variable_frame);
    }
    let unknown = [crate::InvocationWord::Literal("item")];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("global"),
        &unknown,
    ));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(
        projection.opaque_variable_frame,
        "missing alias naming policy remains unknown"
    );
}

#[test]
fn variable_write_projection_does_not_infer_roles_from_an_unrelated_resolver() {
    let reg = CommandRegistry::build_default();
    let arguments = [crate::InvocationWord::Dynamic];
    let projection = reg.variable_write_projection(InvocationWords::structured(
        crate::InvocationWord::Literal("puts"),
        &arguments,
    ));
    assert_eq!(projection, VariableWriteProjection::default());
}

/// An unbind reads its place's existence: the
/// read projection names a destroyer's targets, past `-nocomplain` and
/// `--`, where the write projection names none, and a substituted target
/// widens the frame.
#[test]
fn variable_read_projection_names_a_destroyers_targets() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Literal};
    let reg = CommandRegistry::build_default();
    for (arguments, expected) in [
        (vec![Literal("x")], vec!["x"]),
        (
            vec![
                Literal("-nocomplain"),
                Literal("--"),
                Literal("a"),
                Literal("b"),
            ],
            vec!["a", "b"],
        ),
    ] {
        let words = InvocationWords::structured(Literal("unset"), &arguments);
        assert_eq!(reg.variable_read_projection(words).literal_names, expected);
        assert_eq!(
            reg.variable_write_projection(words),
            VariableWriteProjection::default()
        );
    }
    let computed = [Dynamic];
    let projection =
        reg.variable_read_projection(InvocationWords::structured(Literal("unset"), &computed));
    assert_eq!(projection.literal_names.len(), 0);
    assert!(projection.opaque_variable_frame);
    let query = [Literal("exists"), Literal("x")];
    assert_eq!(
        reg.variable_read_projection(InvocationWords::structured(Literal("info"), &query))
            .literal_names,
        vec!["x"]
    );
}

#[test]
fn literal_adapter_matches_the_structured_literal_view() {
    let reg = CommandRegistry::build_default();
    let arguments = ["counter"];
    let adapter = reg
        .resolve_invocation("incr", &arguments, None)
        .expect("literal adapter resolves");
    let structured = reg
        .resolve_structured_invocation(InvocationWords::literals("incr", &arguments), None)
        .resolved()
        .expect("structured literal input resolves");

    assert_eq!(adapter.canonical_command, structured.canonical_command);
    assert_eq!(adapter.subcommand, structured.subcommand);
    assert_eq!(
        adapter.form.map(|form| form.name),
        structured.form.map(|form| form.name)
    );
    assert_eq!(adapter.semantics.operation, structured.semantics.operation);
}

#[test]
fn structured_resolution_reports_computed_and_unknown_command_heads() {
    let reg = CommandRegistry::build_default();
    let no_arguments: [crate::InvocationWord<'_>; 0] = [];

    let computed = reg.resolve_structured_invocation(
        InvocationWords::structured(crate::InvocationWord::Dynamic, &no_arguments),
        None,
    );
    assert_eq!(
        computed.unresolved(),
        Some(crate::InvocationResolutionUnresolved::ComputedHead {
            word_kind: crate::InvocationWordKind::Dynamic,
        })
    );

    let no_literal_arguments: [&str; 0] = [];
    let unknown = reg.resolve_structured_invocation(
        InvocationWords::literals("not-a-registry-command", &no_literal_arguments),
        None,
    );
    assert_eq!(
        unknown.unresolved(),
        Some(crate::InvocationResolutionUnresolved::UnknownLiteralHead {
            spelling: "not-a-registry-command",
        })
    );
}

#[test]
fn irules_placement_contract_owns_declaration_and_execution_contexts() {
    use crate::events::{IrulesCommandPlacement as Placement, IrulesExecutionContext as Ctx};

    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    for declaration in ["when", "proc", "timing", "priority"] {
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::TopLevel),
            Placement::Allowed,
            "{declaration} is an iRules top-level declaration"
        );
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::EventBody),
            Placement::RequiresTopLevel,
            "{declaration} cannot be nested in an event"
        );
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::ProcedureBody),
            Placement::RequiresTopLevel,
            "{declaration} cannot be nested in a proc"
        );
    }
    for executable in ["set", "HTTP::uri", "call", "user_proc"] {
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::TopLevel),
            Placement::RequiresEventOrProcedure,
            "{executable} cannot execute at iRules top level"
        );
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::EventBody),
            Placement::Allowed
        );
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::ProcedureBody),
            Placement::Allowed
        );
    }
}

#[test]
fn irules_declaration_owner_requires_braced_source_bodies() {
    use crate::events::IrulesTopLevelDeclaration as Declaration;

    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let events = crate::events::EventRegistry::build();
    let declaration = |name: &str, args: &[&str]| {
        let body_index = if name == "proc" {
            2
        } else {
            args.len().saturating_sub(1)
        };
        let tokens = args
            .iter()
            .enumerate()
            .map(|(index, _)| {
                tcl_lexer::Token::new(
                    if index == body_index {
                        tcl_lexer::TokenType::Str
                    } else {
                        tcl_lexer::TokenType::Esc
                    },
                    tcl_lexer::Span::empty(0),
                )
            })
            .collect::<Vec<_>>();
        let single = vec![true; args.len()];
        let closed = tokens
            .iter()
            .map(|token| token.kind == tcl_lexer::TokenType::Str)
            .collect::<Vec<_>>();
        registry.irules_top_level_declaration(
            name,
            crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                .expect("parallel source-word views"),
            &events,
        )
    };
    assert!(matches!(
        declaration("when", &["HTTP_REQUEST", "set x 1"]),
        Some(Declaration::Event { .. })
    ));
    assert!(matches!(
        declaration("::when", &["HTTP_REQUEST", "set x 1"]),
        Some(Declaration::Event { .. })
    ));
    assert!(declaration("when", &["BOGUS_EVENT", "pool x"]).is_none());
    for valid in [
        &["HTTP_REQUEST", "body"][..],
        &["HTTP_REQUEST", "priority", "100", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "body"][..],
        &[
            "HTTP_REQUEST",
            "priority",
            "100",
            "timing",
            "disable",
            "body",
        ][..],
        &["HTTP_REQUEST", "priority", "0", "body"][..],
        &["HTTP_REQUEST", "priority", "1000", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "priority", "100", "body"][..],
    ] {
        assert!(declaration("when", valid).is_some());
    }
    for invalid in [
        &["HTTP_REQUEST"][..],
        &["HTTP_REQUEST", "body", "trailing"][..],
        &["HTTP_REQUEST", "priority", "bad", "body"][..],
        &["HTTP_REQUEST", "priority", "-1", "body"][..],
        &["HTTP_REQUEST", "priority", "1001", "body"][..],
        &["HTTP_REQUEST", "timing", "maybe", "body"][..],
        &["HTTP_REQUEST", "priority", "1", "priority", "2", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "timing", "off", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "priority", "1001", "body"][..],
    ] {
        assert!(declaration("when", invalid).is_none());
    }
    assert!(matches!(
        declaration("proc", &["p", "", "return"]),
        Some(Declaration::Procedure { .. })
    ));
    for value in ["0", "500", "1000"] {
        assert_eq!(
            declaration("priority", &[value]),
            Some(Declaration::Priority {
                value: value.parse().unwrap()
            })
        );
    }
    for invalid in ["-1", "1001", "bad"] {
        assert!(declaration("priority", &[invalid]).is_none());
    }
    assert_eq!(
        declaration("timing", &["enable"]),
        Some(Declaration::Timing { enabled: true })
    );
    for malformed in [&["p", ""][..], &["p", "", "return", "extra"][..]] {
        assert!(declaration("proc", malformed).is_none());
    }
}

#[test]
fn irules_declaration_owner_rejects_bare_and_quoted_source_bodies() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    for (name, args, body_index) in [
        ("when", &["HTTP_REQUEST", "set x 1"][..], 1),
        ("proc", &["p", "", "return"][..], 2),
    ] {
        let mut tokens =
            vec![
                tcl_lexer::Token::new(tcl_lexer::TokenType::Esc, tcl_lexer::Span::empty(0),);
                args.len()
            ];
        let single = vec![true; args.len()];
        let closed = vec![false; args.len()];
        for (kind, in_quote) in [
            (tcl_lexer::TokenType::Esc, false),
            (tcl_lexer::TokenType::Esc, true),
            // Lexer recovery uses Str for `{body` at EOF; the separate
            // closed-brace fact must keep it out of declarations too.
            (tcl_lexer::TokenType::Str, false),
        ] {
            tokens[body_index] = tcl_lexer::Token {
                kind,
                in_quote,
                ..tokens[body_index]
            };
            let arguments =
                crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                    .expect("parallel source-word views");
            assert!(
                registry
                    .irules_top_level_declaration_shape(name, arguments)
                    .is_none(),
                "{name} must reject a non-closed-braced body"
            );
        }
    }
}

#[test]
fn literal_form_selectors_choose_the_longest_static_match() {
    const FORMS: &[CommandForm] = &[
        CommandForm {
            name: "tag-one",
            arity: Arity::exact(1),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "tag-fallback",
            arity: Arity::exact(3),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "tag-bind",
            arity: Arity::exact(3),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag", "bind"])),
            ..CommandForm::DEFAULT
        },
    ];
    const SPEC: CommandSpec = CommandSpec {
        name: "literal-form-fixture",
        arity: Arity::any(),
        command_forms: FORMS,
        ..CommandSpec::DEFAULT
    };

    let mut registry = CommandRegistry::build_default();
    registry.insert(SPEC);
    let form = |args: &[&str]| {
        registry
            .resolve_invocation("literal-form-fixture", args, None)
            .and_then(|resolved| resolved.form)
            .map(|form| form.name)
    };

    assert_eq!(form(&["tag"]), Some("tag-one"), "arity disambiguates");
    assert_eq!(
        form(&["tag", "bind", "script"]),
        Some("tag-bind"),
        "the longer selector overrides its completed prefix"
    );
    assert_eq!(
        form(&["ta", "bi", "script"]),
        Some("tag-bind"),
        "each selector word still accepts a unique abbreviation"
    );
    assert_eq!(
        form(&["tag", "other", "value"]),
        Some("tag-fallback"),
        "a static non-match proves that the shorter selector applies"
    );

    let dynamic = [
        InvocationWord::Literal("tag"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("script"),
    ];
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("literal-form-fixture"), &dynamic),
            None,
        )
        .resolved()
        .expect("fixture command resolves");
    assert!(
        resolved.form.is_none(),
        "a dynamic word that could extend a completed selector must abstain"
    );
}

#[test]
fn unresolved_subcommands_never_fall_back_to_a_top_level_form() {
    const TOP_LEVEL_FORM: CommandForm = CommandForm {
        name: "top-level-form",
        arity: Arity::exact(1),
        ..CommandForm::DEFAULT
    };
    const SUBCOMMANDS: &[SubCommand] = &[
        SubCommand {
            name: "alpha",
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "alpine",
            ..SubCommand::DEFAULT
        },
    ];
    const SPEC: CommandSpec = CommandSpec {
        name: "subcommand-form-fixture",
        arity: Arity::any(),
        subcommands: SUBCOMMANDS,
        command_forms: &[TOP_LEVEL_FORM],
        ..CommandSpec::DEFAULT
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(SPEC);
    let dynamic_arguments = [crate::InvocationWord::Dynamic];
    let dynamic = reg
        .resolve_structured_invocation(
            InvocationWords::structured(
                crate::InvocationWord::Literal("subcommand-form-fixture"),
                &dynamic_arguments,
            ),
            None,
        )
        .resolved()
        .expect("fixture command resolves");
    assert!(matches!(
        dynamic.subcommand,
        SubcommandResolution::Indeterminate { .. }
    ));
    assert!(dynamic.form.is_none());
    assert!(matches!(
        dynamic.facts().subcommand,
        crate::OwnedSubcommandResolution::Indeterminate {
            word_kind: crate::InvocationWordKind::Dynamic
        }
    ));

    let ambiguous = reg
        .resolve_invocation("subcommand-form-fixture", &["al"], None)
        .expect("fixture command resolves");
    assert!(matches!(
        ambiguous.subcommand,
        SubcommandResolution::Ambiguous { .. }
    ));
    assert!(ambiguous.form.is_none());
    assert!(matches!(
        ambiguous.facts().subcommand,
        crate::OwnedSubcommandResolution::Ambiguous { ref spelling } if spelling == "al"
    ));

    let unknown = reg
        .resolve_invocation("subcommand-form-fixture", &["missing"], None)
        .expect("fixture command resolves");
    assert!(matches!(
        unknown.subcommand,
        SubcommandResolution::Unknown { .. }
    ));
    assert!(unknown.form.is_none());
    assert!(matches!(
        unknown.facts().subcommand,
        crate::OwnedSubcommandResolution::Unknown { ref spelling } if spelling == "missing"
    ));
}

#[test]
fn resolve_invocation_applies_registry_unique_prefix_subcommand_resolution() {
    let reg = CommandRegistry::build_default();
    let args = ["le", "hello"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("string is registry-known");

    let sub = resolved
        .subcommand
        .resolved()
        .expect("string le is a valid unique prefix");
    assert_eq!(sub.spelling, "le");
    assert_eq!(sub.canonical_name, "length");
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::UniquePrefix)
    );
    assert_eq!(
        resolved.semantics.return_type,
        Some(crate::types::TclType::Int)
    );
}

#[test]
fn resolve_invocation_preserves_an_ambiguous_subcommand_outcome() {
    let reg = CommandRegistry::build_default();
    let args = ["t", "hello"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("the command head remains known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Ambiguous { spelling: "t" }
    ));
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Ambiguous)
    );
    assert!(resolved.form.is_none());
}

#[test]
fn resolve_invocation_preserves_an_unknown_subcommand_outcome() {
    let reg = CommandRegistry::build_default();
    let args = ["does-not-exist"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("the command head remains known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Unknown {
            spelling: "does-not-exist"
        }
    ));
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Unknown)
    );
}

#[test]
fn resolve_invocation_projects_forms_without_backend_hooks() {
    let reg = CommandRegistry::build_default();
    let args = ["counter"];
    let resolved = reg
        .resolve_invocation("incr", &args, None)
        .expect("incr is registry-known");

    let form = resolved.form.expect("implicit incr form");
    assert_eq!(form.name, "implicit");
    assert_eq!(form.arity, Arity::exact(1));
    assert_eq!(form.arg_roles, &[(0, ArgRole::VarWrite)]);
    assert_eq!(resolved.semantics.arg_roles, form.arg_roles);
    assert_eq!(
        resolved.semantics.return_type,
        Some(crate::types::TclType::Int)
    );
    assert_eq!(
        resolved.semantics.lowering_hook,
        Some(LoweringHookId::Incr),
        "the common lowering descriptor survives the target-neutral projection"
    );
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
    );
}

#[test]
fn resolve_invocation_identifies_channel_write_without_selecting_a_backend() {
    let reg = CommandRegistry::build_default();
    let args = ["hello"];
    let resolved = reg
        .resolve_invocation("puts", &args, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
        .expect("puts resolves");

    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ChannelWrite)
    );
    assert_eq!(
        reg.command_names_for_semantic_operation(crate::SemanticOperationId::Intrinsic(
            crate::IntrinsicId::ChannelWrite,
        ))
        .collect::<Vec<_>>(),
        vec!["puts"]
    );
}

#[test]
fn semantic_operation_resolution_prefers_form_then_subcommand_then_command() {
    const FORM: CommandForm = CommandForm {
        name: "form",
        arity: Arity::exact(1),
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Set,
        )),
        ..CommandForm::DEFAULT
    };
    const SUB: SubCommand = SubCommand {
        name: "sub",
        arity: Arity::at_least(0),
        subcommand_forms: &[FORM],
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Incr,
        )),
        ..SubCommand::DEFAULT
    };
    const SPEC: CommandSpec = CommandSpec {
        name: "semantic-operation-precedence",
        arity: Arity::at_least(0),
        subcommands: &[SUB],
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Return,
        )),
        ..CommandSpec::DEFAULT
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(SPEC);

    let command = reg
        .resolve_invocation("semantic-operation-precedence", &[], None)
        .expect("command form resolves");
    assert_eq!(
        command.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
    );

    let sub = reg
        .resolve_invocation("semantic-operation-precedence", &["sub"], None)
        .expect("subcommand form resolves");
    assert_eq!(
        sub.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
    );

    let form = reg
        .resolve_invocation("semantic-operation-precedence", &["sub", "argument"], None)
        .expect("subcommand form resolves");
    assert_eq!(
        form.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Set)
    );
}

#[test]
fn resolve_invocation_exposes_subcommand_effects_through_one_semantic_view() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let args = ["insert", "x-demo", "value"];
    let resolved = reg
        .resolve_invocation(
            "HTTP::header",
            &args,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .expect("HTTP::header insert resolves in the iRules dialect");

    assert!(resolved.semantics.traits.contains(Traits::PURE));
    assert_eq!(resolved.semantics.side_effects.lower_bound().len(), 1);
    let effect = resolved.semantics.side_effects.lower_bound()[0];
    assert_eq!(
        effect.target,
        crate::side_effects::SideEffectTarget::HttpHeader
    );
    assert!(effect.reads);
    assert!(effect.writes);
}

#[test]
fn resolved_call_and_resolved_invocation_share_selection() {
    let reg = CommandRegistry::build_default();
    let args = ["counter", "5"];
    let common = reg
        .resolve_invocation("incr", &args, None)
        .expect("incr resolves");
    let legacy = reg
        .resolve_call("incr", &args, None)
        .expect("legacy compatibility resolver remains available");

    assert_eq!(common.canonical_command, legacy.spec.name);
    assert_eq!(
        common.form.map(|form| form.name),
        legacy.form.map(|form| form.name)
    );
    assert_eq!(common.semantics.lowering_hook, legacy.lowering_hook);
    assert_eq!(common.semantics.arity, legacy.arity());
}

#[test]
fn resolve_call_top_level_command() {
    let reg = CommandRegistry::build_default();
    let resolved = reg.resolve_call("set", &["x", "1"], None).unwrap();
    assert_eq!(resolved.spec.name, "set");
    assert!(resolved.sub.is_none());
}

#[test]
fn resolve_call_subcommand() {
    let reg = CommandRegistry::build_default();
    let resolved = reg
        .resolve_call(
            "dict",
            &["create", "k", "v"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(resolved.spec.name, "dict");
    let sub = resolved.sub.expect("dict create resolves to a subcommand");
    assert_eq!(sub.name, "create");
}

#[test]
fn resolve_call_dialect_filter_blocks_tcl84_dict() {
    let reg = CommandRegistry::build_default();
    // dict is tcl8.5+; resolving against tcl8.4 must fail.
    assert!(
        reg.resolve_call(
            "dict",
            &["create"],
            Some(SurfaceQuery::core(Family::Tcl, "8.4"))
        )
        .is_none()
    );
}

#[test]
fn resolve_call_picks_arity_matched_command_form_for_incr() {
    let reg = CommandRegistry::build_default();
    // `incr counter` — arity 1 → matches the implicit form.
    let r1 = reg.resolve_call("incr", &["counter"], None).unwrap();
    let f1 = r1.form.expect("incr should match a CommandForm");
    assert_eq!(f1.name, "implicit");
    assert_eq!(f1.arity, crate::arity::Arity::exact(1));

    // `incr counter 5` — arity 2 → matches the explicit form.
    let r2 = reg.resolve_call("incr", &["counter", "5"], None).unwrap();
    let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
    assert_eq!(f2.name, "explicit");
    assert_eq!(f2.arity, crate::arity::Arity::exact(2));
}

#[test]
fn resolve_call_picks_arity_matched_command_form_for_lset() {
    let reg = CommandRegistry::build_default();
    // `lset lst value` — arity 2 → replace form.
    let replace = reg
        .resolve_call(
            "lset",
            &["lst", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(replace.form.unwrap().name, "replace");

    // `lset lst 0 value` — arity 3 → single_index form.
    let single = reg
        .resolve_call(
            "lset",
            &["lst", "0", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(single.form.unwrap().name, "single_index");

    // `lset lst 0 1 2 value` — arity 5 → flat_path form.
    let flat = reg
        .resolve_call(
            "lset",
            &["lst", "0", "1", "2", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(flat.form.unwrap().name, "flat_path");
}

// -- ``resolve_option_terminator`` (W304 driver)
//
// Exercises option-terminator resolution for each command.
// Each W304 fixture
// is rooted in one of these resolver outcomes; the resolver tests
// here pin the per-command shape, the analyser tests pin the
// tristate-severity / two-diagnostic / code-fix behaviour.

#[test]
fn resolve_option_terminator_returns_none_for_unknown_command() {
    let reg = CommandRegistry::build_default();
    assert!(
        reg.resolve_option_terminator("unknownthing", &[], None)
            .is_none()
    );
}

#[test]
fn resolve_option_terminator_returns_none_for_command_without_terminator() {
    let reg = CommandRegistry::build_default();
    // ``set`` does not declare a ``--`` terminator option.
    assert!(
        reg.resolve_option_terminator("set", &["x", "1"], None)
            .is_none()
    );
}

#[test]
fn resolve_option_terminator_form_level_for_regexp() {
    let reg = CommandRegistry::build_default();
    let profile = reg
        .resolve_option_terminator("regexp", &[], None)
        .expect("regexp declares -- at the form level");
    assert_eq!(profile.scan_start, 0);
    assert!(profile.subcommand.is_none());
    // ``-start`` takes a value; ``-nocase`` does not.
    // ``-start`` takes a value; ``-nocase`` does not.  The
    // resolver returns the borrowed options slice; callers
    // filter via ``OptionSpec::takes_value``.
    assert!(
        profile
            .options
            .iter()
            .any(|o| o.name == "-start" && o.takes_value())
    );
    assert!(
        profile
            .options
            .iter()
            .any(|o| o.name == "-nocase" && !o.takes_value())
    );
}

#[test]
fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
    let reg = CommandRegistry::build_default();
    let profile = reg
        .resolve_option_terminator("file", &["delete", "$path"], None)
        .expect("file delete declares -- at the subcommand level");
    assert_eq!(profile.scan_start, 1);
    assert_eq!(profile.subcommand, Some("delete"));
}

#[test]
fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
    let reg = CommandRegistry::build_default();
    // ``file mtime`` has no ``--`` terminator.
    let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], None);
    assert!(profile.is_none(), "got {profile:?}");
}

// -- ``is_canonical_list_command`` (W101 safe-idiom driver)

#[test]
fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
    let reg = CommandRegistry::build_default();
    assert!(reg.is_canonical_list_command("list"));
    assert!(reg.is_canonical_list_command("linsert"));
    assert!(reg.is_canonical_list_command("split"));
    assert!(reg.is_canonical_list_command("lreverse"));
    // ``concat`` returns LIST but is the explicit non-canonical
    // exclusion.
    assert!(!reg.is_canonical_list_command("concat"));
    // Non-list commands (e.g. ``set``) are filtered out.
    assert!(!reg.is_canonical_list_command("set"));
    // Unknown commands return false.
    assert!(!reg.is_canonical_list_command("unknownthing"));
}

#[test]
fn is_canonical_list_command_handles_compound_subcommand_keys() {
    let reg = CommandRegistry::build_default();
    // ``dict keys`` returns LIST.
    assert!(reg.is_canonical_list_command("dict keys"));
    // ``dict get`` returns String (or unspecified) — not canonical.
    assert!(!reg.is_canonical_list_command("dict froob"));
}

#[test]
fn irules_sink_commands_carry_structural_options() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();

    let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
    let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
    // Option set follows the reference standard:
    // -version/-content/-ifile/
    // -noserver/-reset. (`-status` is the positional status arg.)
    assert!(
        opts.contains(&"-version")
            && opts.contains(&"-content")
            && opts.contains(&"-noserver")
            && opts.contains(&"-reset"),
        "HTTP::respond options {opts:?} should include -version / -content / -noserver / -reset",
    );
    let noserver = respond
        .options
        .iter()
        .find(|o| o.name == "-noserver")
        .unwrap();
    assert!(!noserver.takes_value());
    let version = respond
        .options
        .iter()
        .find(|o| o.name == "-version")
        .unwrap();
    assert!(version.takes_value());

    let header = reg.get("HTTP::header").expect("HTTP::header loaded");
    let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
    assert!(
        header_opts.contains(&"-noupdate"),
        "HTTP::header options {header_opts:?} should include -noupdate",
    );
}

#[test]
fn xc_translatability_helpers_read_spec_flags() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();

    // `xc_translatable: Some(false)` → never translatable (consumed by the
    // `f5-xc` translator's XC300 branch).
    assert!(reg.is_xc_never_translatable("eval"));
    assert!(!reg.is_xc_translatable_override("eval"));

    // `xc_translatable: Some(true)` → translatable override despite an
    // otherwise-untranslatable namespace prefix (e.g. `IP::`, `ASM::`).
    assert!(reg.is_xc_translatable_override("IP::client_addr"));
    assert!(!reg.is_xc_never_translatable("IP::client_addr"));

    // Commands with no `xc_translatable` flag report neither.
    assert!(!reg.is_xc_never_translatable("set"));
    assert!(!reg.is_xc_translatable_override("set"));
    // An unknown command name is safely neither.
    assert!(!reg.is_xc_never_translatable("no_such_command_xyz"));
    assert!(!reg.is_xc_translatable_override("no_such_command_xyz"));
}

/// `unit_linkage` composes `spec.traits | sub.traits` and filters to the
/// linkage union, so the answer is subcommand-precise: `package provide`
/// publishes an API surface, `package require` pulls another unit in, and
/// `package names` does neither.
#[test]
fn unit_linkage_is_subcommand_precise() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    assert_eq!(
        reg.unit_linkage("package", &["provide", "mylib", "1.0"], empty),
        Traits::PROVIDES_PACKAGE
    );
    assert_eq!(
        reg.unit_linkage("package", &["ifneeded", "mylib", "1.0", "body"], empty),
        Traits::PROVIDES_PACKAGE
    );
    assert_eq!(
        reg.unit_linkage("package", &["require", "mylib"], empty),
        Traits::LOADS_EXTERNAL_UNIT
    );
    assert_eq!(
        reg.unit_linkage("package", &["names"], empty),
        Traits::empty()
    );
}

/// `invocation_traits` composes `spec.traits | sub.traits` for the
/// concrete call, which is the only way the eval-family bits on the
/// compound members are visible: `namespace eval` / `namespace inscope` /
/// `interp eval` carry `EVALUATES_CODE` on the **subcommand**, so a
/// parent-only `get(name).traits` test misses them.
#[test]
fn invocation_traits_compose_subcommand_traits() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    // TP — the eval family, bare and compound.
    for (name, args) in [
        ("eval", &["{set x 1}"][..]),
        ("uplevel", &["1", "{set x 1}"][..]),
        ("namespace", &["eval", "ns", "{set x 1}"][..]),
        ("namespace", &["inscope", "ns", "{set x 1}"][..]),
        ("interp", &["eval", "slave", "{set x 1}"][..]),
    ] {
        let traits = reg.invocation_traits(name, args, empty);
        assert!(
            traits.contains(Traits::EVALUATES_CODE),
            "{name} {args:?} must carry EVALUATES_CODE"
        );
        assert!(
            traits.contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{name} {args:?} must carry SCRIPT_CONCATENATES_ARGS"
        );
    }
    // TN — the parent spec alone carries neither bit, which is exactly
    // why composing is required.
    let parent = reg.get("namespace").expect("namespace spec").traits;
    assert!(!parent.contains(Traits::EVALUATES_CODE));
    // TN — a sibling subcommand that evaluates nothing stays clean.
    assert!(
        !reg.invocation_traits("namespace", &["delete", "ns"], empty)
            .contains(Traits::EVALUATES_CODE)
    );
    // TN — an unknown command carries no traits at all.
    assert_eq!(
        reg.invocation_traits("no_such_command_xyz", &["a"], empty),
        Traits::empty()
    );
    // The parent's own traits still compose in — a subcommand's traits
    // are additive, not a replacement.
    assert!(
        reg.invocation_traits("namespace", &["eval", "ns", "{}"], empty)
            .contains(Traits::LANGUAGE_KEYWORD),
        "the parent `namespace` spec's own bits survive composition"
    );
}

#[test]
fn typed_control_arms_are_registry_owned() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "{}"], 2),
        Some(ControlArmSemantics::Always)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "on", "error", "{m o}", "{}"], 4,),
        Some(ControlArmSemantics::Selected)
    );
    assert_eq!(
        reg.try_control_invocation("try", &["{}", "on", "error", "{m o}", "{}"], None,),
        Some(TryControlInvocation {
            body_index: 0,
            clauses: vec![TryControlClause {
                kind: TryClauseKind::On(TryCompletionSelector::Error),
                selector_index: Some(2),
                variable_list_index: Some(3),
                body_index: 4,
                fallthrough: false,
            }],
        })
    );
    assert_eq!(
        reg.control_arm_semantics("namespace", &["eval", "::ns", "{}"], 2),
        Some(ControlArmSemantics::FrameBoundary)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "{}", "orphan"], 0),
        None,
        "a malformed trailing clause invalidates even the main arm"
    );
    for args in [
        &["{}", "on", "bogus", "{}", "{}"][..],
        &["{}", "on", "ok", "a b c", "{}"][..],
        &["{}", "on", "ok", "{}", "-"][..],
    ] {
        assert_eq!(reg.control_arm_semantics("try", args, 0), None);
    }
    assert_eq!(
        reg.control_invocation_valid("try", &["{}", "on", "ok", "{}", "-"], None,),
        Some(false),
        "a terminal handler fallthrough has no following script"
    );
    assert_eq!(
        reg.control_arm_semantics("try", &["{}", "finally", "-"], 2),
        Some(ControlArmSemantics::Always),
        "a finally body is a script, so `-` has no fallthrough meaning there"
    );
    let chained_fallthrough = [
        "{}",
        "on",
        "error",
        "{}",
        "-",
        "on",
        "error",
        "{}",
        "{set r fell}",
    ];
    assert_eq!(
        reg.control_invocation_valid("try", &chained_fallthrough, None),
        Some(true)
    );
    assert_eq!(
        reg.control_arm_semantics("try", &chained_fallthrough, 8),
        Some(ControlArmSemantics::Selected)
    );
    assert_eq!(
        reg.control_invocation_valid("if", &["1", "then"], None),
        Some(false)
    );
    assert_eq!(
        reg.control_invocation_valid("if", &["1", "a", "b"], None),
        Some(true)
    );
    assert_eq!(
        reg.control_arm_semantics("if", &["1", "a", "b"], 2),
        Some(ControlArmSemantics::Selected)
    );
}

#[test]
fn try_completion_selectors_use_the_profiled_tcl_number_and_completion_owners() {
    // `try_control_invocation` receives resolved word values. In source,
    // `{ 1 }` becomes the `" 1 "` value below; a literal value of `{1}`
    // would instead come from nested braces and is not a completion code.
    let selector = |registry: &CommandRegistry, selector: &str| {
        let invocation = registry.try_control_invocation(
            "try",
            &["{body}", "on", selector, "{message}", "{handler}"],
            None,
        )?;
        let [clause] = invocation.clauses.as_slice() else {
            return None;
        };
        match clause.kind {
            TryClauseKind::On(selector) => Some(selector),
            TryClauseKind::Trap | TryClauseKind::Finally => None,
        }
    };

    let tcl9 = crate::model::ingress::static_context_for("tcl9.0").commands();
    for spelling in ["+1", "01", "0x1", " 1 "] {
        assert_eq!(
            selector(tcl9, spelling),
            Some(TryCompletionSelector::Error),
            "Tcl 9 completion selector {spelling:?}"
        );
    }
    assert_eq!(
        selector(tcl9, "{1}"),
        None,
        "the registry must not mistake nested literal braces for source-word delimiters"
    );

    // The number owner selects grammar from the registry's target
    // profile. A leading-zero spelling is octal through Tcl 8.6 and
    // decimal from Tcl 9.0.
    let tcl8 = crate::model::ingress::static_context_for("tcl8.6").commands();
    assert_eq!(
        selector(tcl8, "010"),
        Some(TryCompletionSelector::Numeric(8))
    );
    assert_eq!(
        selector(tcl9, "010"),
        Some(TryCompletionSelector::Numeric(10))
    );

    // Completion codes use Tcl's C-compatible signed-int domain: the
    // final unsigned range wraps, while values outside it are invalid.
    assert_eq!(
        selector(tcl9, "4294967295"),
        Some(TryCompletionSelector::Numeric(-1))
    );
    for out_of_range in ["-2147483649", "4294967296", "9223372036854775808"] {
        assert_eq!(
            selector(tcl9, out_of_range),
            None,
            "out-of-range completion selector {out_of_range:?}"
        );
    }
}

#[test]
fn explicit_control_surface_selects_completion_code_numeral_grammar() {
    let registry = CommandRegistry::build_default();
    let args = ["{body}", "on", "1_0", "{message}", "{handler}"];
    let tcl86 = SurfaceQuery::core(Family::Tcl, "8.6");
    let tcl90 = SurfaceQuery::core(Family::Tcl, "9.0");

    assert_eq!(
        registry.control_invocation_valid("try", &args, Some(tcl86)),
        Some(false),
        "Tcl 8.6 does not admit underscore-separated completion codes"
    );
    assert_eq!(
        registry.try_control_invocation("try", &args, Some(tcl86)),
        None
    );
    assert_eq!(
        registry.control_invocation_valid("try", &args, Some(tcl90)),
        Some(true),
        "Tcl 9 admits underscore-separated completion codes"
    );
    let invocation = registry
        .try_control_invocation("try", &args, Some(tcl90))
        .expect("Tcl 9 try invocation");
    assert!(matches!(
        invocation.clauses.as_slice(),
        [TryControlClause {
            kind: TryClauseKind::On(TryCompletionSelector::Numeric(10)),
            ..
        }]
    ));
}

#[test]
fn invocation_completion_is_registry_owned() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert_eq!(
        reg.invocation_completion("return", &["-code", "error", "$w"], None,),
        InvocationCompletion::Terminates
    );
    assert_eq!(
        reg.invocation_completion("return", &["$w"], None),
        InvocationCompletion::ReturnsResult(Some(0))
    );
    assert_eq!(
        reg.invocation_completion("return", &["-code"], None),
        InvocationCompletion::ReturnsResult(Some(0)),
        "a trailing option-shaped word is return's result, not a missing option value"
    );
    assert_eq!(
        reg.invocation_completion("return", &["--", "x"], None),
        InvocationCompletion::ReturnsResult(None),
        "`--` is no end of options: `return -- x` is the pair `-- x` and the empty result"
    );
    assert_eq!(
        reg.invocation_completion("return", &["-level", "0", "$w"], None,),
        InvocationCompletion::FallsThrough
    );
    for args in [
        &["-level", "0", "-code", "error", "$w"][..],
        &["-code", "error", "-level", "0", "$w"][..],
        &["-level", "2", "$w"][..],
        &["-level", "-1", "$w"][..],
    ] {
        assert_eq!(
            reg.invocation_completion("return", args, None),
            InvocationCompletion::Terminates,
            "{args:?}"
        );
    }
    assert_eq!(
        reg.invocation_completion("return", &["-level", "$dynamic", "$w"], None),
        InvocationCompletion::Unknown,
    );
    {
        let invalid = &["-level", "-1", "value"][..];
        assert_eq!(
            reg.invocation_completion("return", invalid, None),
            InvocationCompletion::Terminates,
            "native validation error ends the current path"
        );
    }
    // Native worker383 ordinary_pair: C85–C91 catch=2/result empty;
    // C84 catch=1. Custom option pairs are valid in this selected C9 context.
    assert_eq!(
        reg.invocation_completion("return", &["value", "extra"], None),
        InvocationCompletion::ReturnsResult(None)
    );
    // A registry that names no release answers only what every release
    // reads alike: 8.4 rejects `-level`, which 8.5 reads.
    assert_eq!(
        CommandRegistry::build_default().invocation_completion(
            "return",
            &["-level", "0", "$w"],
            None
        ),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        reg.invocation_completion("not-a-command", &[], None),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        reg.invocation_completion("set", &[], None),
        InvocationCompletion::Terminates
    );
}

#[test]
fn source_completion_preserves_dynamic_options_and_pending_return_depth() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    let result = [Literal("-level"), Literal("1"), Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&result),
            None
        ),
        InvocationCompletion::ReturnsResult(Some(2))
    );
    let deeper = [Literal("-level"), Literal("2"), Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&deeper),
            None
        ),
        InvocationCompletion::Terminates,
        "pending return does not become a normal result at this procedure boundary"
    );
    let option = [Literal("-code"), Dynamic, Dynamic];
    assert_eq!(
        registry.invocation_completion_words(
            "return",
            InvocationArguments::structured(&option),
            None
        ),
        InvocationCompletion::Unknown
    );
    assert_eq!(
        CommandRegistry::build_default().invocation_completion(
            "return",
            &["-level", "0", "value"],
            None
        ),
        InvocationCompletion::Unknown,
        "an unselected native grammar cannot license modern return options"
    );
}

#[test]
fn exact_return_options_and_process_exit_are_registry_owned() {
    use crate::completion::CompletionCode;

    let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
    // Oracle (tclsh 8.6/9.0): the default -level 1 return is caught by
    // `try on return`, even when its eventual procedure result is error.
    assert_eq!(
        reg.exact_invocation_completion("return", &["-code", "error", "payload"], None,),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
    );
    assert_eq!(
        reg.exact_invocation_completion(
            "return",
            &["-level", "0", "-code", "error", "payload"],
            None,
        ),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Error))
    );
    for args in [
        &["-foo", "bar", "payload"][..],
        &["-c", "error", "payload"][..],
        &["-level", "0x0", "-code", "error", "payload"][..],
    ] {
        assert_eq!(
            reg.exact_invocation_completion("return", args, None),
            Some(ExactInvocationCompletion::Tcl(if args[0] == "-level" {
                CompletionCode::Error
            } else {
                CompletionCode::Return
            }))
        );
    }
    for (spelling, expected) in [("2147483648", i32::MIN), ("4294967295", -1)] {
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", spelling, "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Other(
                expected
            )))
        );
    }
    for (spelling, expected) in [
        ("00", CompletionCode::Ok),
        ("+1", CompletionCode::Error),
        ("02", CompletionCode::Return),
        ("0x3", CompletionCode::Break),
        ("04", CompletionCode::Continue),
    ] {
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", spelling, "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(expected)),
            "{spelling}"
        );
    }
    assert_eq!(
        reg.exact_invocation_completion("exit", &["0"], None),
        Some(ExactInvocationCompletion::ProcessExit)
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["-options", "$dynamic", "payload"], None,),
        None
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["-code", "$dynamic", "payload"], None,),
        None
    );
    assert_eq!(
        reg.exact_invocation_completion("return", &["--", "-code", "error"], None,),
        Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
    );
}

#[test]
fn exit_status_completion_follows_the_release_specific_tcl_oracle() {
    use crate::completion::CompletionCode;
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        // Tcl 8.x's `exit` calls `Tcl_GetIntFromObj`. Despite the `int`
        // destination, its 64-bit implementation accepts the asymmetric
        // `-UINT_MAX..=UINT_MAX` range before casting to `int`.
        for args in [
            &[][..],
            &["0"][..],
            &["-7"][..],
            &["0x10"][..],
            &["-4294967295"][..],
            &["4294967295"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::ProcessExit),
                "{dialect}: exit {args:?}"
            );
        }
        // One value beyond either Tcl 8.x conversion endpoint never gets
        // as far as process termination: it is a catchable Tcl error.
        for args in [
            &["nope"][..],
            &["1.5"][..],
            &["-4294967296"][..],
            &["4294967296"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: exit {args:?}"
            );
        }
    }

    for dialect in ["tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        // Tcl 9.0+ changed `exit` to `TclGetWideBitsFromObj`: every
        // integer, including a bignum beyond wide range, reaches
        // `Tcl_Exit` and has its low bits cast to `int` there.
        for args in [
            &[][..],
            &["0"][..],
            &["-4294967296"][..],
            &["4294967296"][..],
            &["18446744073709551616"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::ProcessExit),
                "{dialect}: exit {args:?}"
            );
        }
        for args in [&["nope"][..], &["1.5"][..], &["0", "extra"][..]] {
            assert_eq!(
                reg.exact_invocation_completion("exit", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: exit {args:?}"
            );
        }
    }

    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let dynamic = [InvocationWord::Dynamic];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "exit",
                InvocationArguments::structured(&dynamic),
                None,
            ),
            Some(InvocationCompletionKnowledge::ExitOrError),
            "{dialect}: exit $status is only process-exit or Tcl error"
        );
        let expanded = [InvocationWord::Expanded];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "exit",
                InvocationArguments::structured(&expanded),
                None,
            ),
            Some(InvocationCompletionKnowledge::ExitOrError),
            "{dialect}: exit {{*}}$statuses has no normal completion either"
        );
    }
}

#[test]
fn exact_return_option_shapes_follow_the_tcl86_and_tcl90_oracle() {
    use crate::completion::CompletionCode;

    // Tcl parses an option only when a following argv word can supply its
    // value. A lone option-shaped word is therefore return's sole result;
    // with `-level 0`, it exposes the default TCL_OK instead.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        for (args, expected) in [
            (&["-code"][..], CompletionCode::Return),
            (&["-level"][..], CompletionCode::Return),
            (&["-options"][..], CompletionCode::Return),
            (&["-foo"][..], CompletionCode::Return),
            (&["-level", "0", "-code"][..], CompletionCode::Ok),
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(expected)),
                "{dialect}: {args:?}"
            );
        }
        // Once a following word exists, the same spellings are options;
        // malformed option values remain TCL_ERROR.
        for args in [
            &["-code", "bogus", "payload"][..],
            &["-level", "-1", "payload"][..],
            &["-code", "-level"][..],
            &["-level", "-code"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {args:?}"
            );
        }
        // Native worker383 confirms ordinary_pair catch=2/result empty
        // for these exact C86/C90 workers. Two words are an option and its value, whatever the first
        // spells, which the options dictionary keeps: `proc p {} {return
        // payload extra}` returns the empty string (tclsh 8.6, 9.0).
        assert_eq!(
            reg.exact_invocation_completion("return", &["payload", "extra"], None),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            "{dialect}"
        );
        // Past `INT_MIN` the conversion's range is the release's own:
        // 8.6 wraps `-2147483649` to 2147483647 and 9.0 rejects it.
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", "-2147483649", "payload"],
                None,
            ),
            None,
            "{dialect}"
        );
    }
}

#[test]
fn exact_invalid_arity_of_known_terminators_is_tcl_error() {
    use crate::completion::CompletionCode;

    // The descriptor of a well-formed call must never hide Tcl's prior
    // arity validation.  This table covers every fixed-arity terminating
    // core primitive, including ones whose normal effect is process exit
    // or a loop transfer rather than TCL_ERROR.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        for (name, args) in [
            ("error", &[][..]),
            ("error", &["message", "info", "code", "extra"][..]),
            ("throw", &[][..]),
            ("throw", &["type"][..]),
            ("throw", &["type", "message", "extra"][..]),
            ("break", &["extra"][..]),
            ("continue", &["extra"][..]),
            ("exit", &["0", "extra"][..]),
        ] {
            assert_eq!(
                reg.exact_invocation_completion(name, args, None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {name} {args:?}"
            );
        }
    }
}

/// The query answers per call, and only for a command that substitutes at
/// all — so a consumer never has to read a switch spelling itself.
#[test]
fn substitutions_performed_answers_per_call_and_only_for_substituting_commands() {
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert_eq!(
        registry.substitutions_performed("set", &["x", "1"]),
        None,
        "a command that substitutes nothing has no answer to give"
    );
    let all = registry
        .substitutions_performed("subst", &["hello $name"])
        .expect("subst substitutes");
    assert!(all.variables && all.commands && all.backslashes);
    let off = registry
        .substitutions_performed("subst", &["-novariables", "hello $name"])
        .expect("subst substitutes");
    assert!(
        !off.variables && off.commands,
        "only the named kind is disabled"
    );
}

#[test]
fn exact_return_source_words_distinguish_literals_from_substitution() {
    use crate::completion::CompletionCode;
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    // Tcl 8.6 and 9.0 oracle: braces keep `$code` and `\\x32` literal,
    // while a bare/quoted substitution remains unknown and a decoded bare
    // escape can be supplied to this API as its effective static value.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let trailing_option = [InvocationWord::Literal("-code")];
        assert_eq!(
            reg.exact_invocation_completion_words(
                "return",
                InvocationArguments::structured(&trailing_option),
                None
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            "{dialect}: source-aware lone -code remains the result word"
        );
        for words in [
            vec![
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("$code"),
            ],
            vec![
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("\\x32"),
            ],
        ] {
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&words),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                "{dialect}: {words:?}"
            );
        }
        let decoded = [
            InvocationWord::Literal("-code"),
            InvocationWord::Literal("2"),
        ];
        assert_eq!(
            reg.exact_invocation_completion_words(
                "return",
                InvocationArguments::structured(&decoded),
                None
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
        );
        for dynamic in [InvocationWord::Dynamic, InvocationWord::Dynamic] {
            let words = [InvocationWord::Literal("-code"), dynamic];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&words),
                    None
                ),
                None,
            );
        }
        let dynamic_option = [InvocationWord::Dynamic, InvocationWord::Literal("error")];
        assert_eq!(
            reg.invocation_completion_knowledge(
                "return",
                InvocationArguments::structured(&dynamic_option),
                None,
            ),
            Some(InvocationCompletionKnowledge::Dynamic),
        );
    }
}

#[test]
fn authored_source_model_roster_keeps_context_separate_from_native_release_evidence() {
    // naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    let driver = crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let older = older.with_command_store(driver.commands().snapshot().shared_registry());
    for (context, lmap) in [(&*driver, true), (&older, false)] {
        let model = context.commands().authored_source_semantics_in_context(
            context.context(),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert!(model.binding_names().contains("::proc"));
        assert_eq!(model.binding_names().contains("::lmap"), lmap);
        assert!(!model.binding_names().contains("::tcltest::test"));
        assert!(!model.binding_names().contains("::tcl::string::equal"));
    }
    let mut required = driver.context().clone();
    required.require_package("tcltest", None);
    let required_model = driver.commands().authored_source_semantics_in_context(
        &required,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    assert!(required_model.binding_names().contains("::tcltest::test"));
    let native =
        driver
            .commands()
            .effective_semantics_for_dialect(crate::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
            ));
    assert!(
        native.binding_names().is_empty(),
        "a source model cannot fill an unaudited Native roster"
    );
}

#[test]
fn binding_rosters_preserve_exact_custom_descriptor_components() {
    let mut registry = CommandRegistry::build_default();
    let rows = [
        ("NAME_BINDING::", "::NAME_BINDING::"),
        ("NAME_BINDING:::member", "::NAME_BINDING:::member"),
        ("::NAME_BINDING::::member", "::NAME_BINDING::::member"),
        ("::::NAME_BINDING", "::::NAME_BINDING"),
        (":NAME_BINDING", ":::NAME_BINDING"),
    ];
    for (name, _) in rows {
        registry.insert(CommandSpec {
            name,
            traits: Traits::UNRESOLVED_COMMAND_HANDLER,
            ..CommandSpec::DEFAULT
        });
    }
    let union = registry.effective_semantics();
    let context = crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let source = registry.authored_source_semantics_in_context(
        context.context(),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let mut models = vec![union, source];
    models.extend(tcl_dialect::TclVersion::ALL.into_iter().map(|version| {
        registry.effective_semantics_for_dialect(crate::InvocationDialect::for_version(version))
    }));
    for model in models {
        for (label, rooted) in rows {
            assert!(model.command(label).is_some(), "{label}");
            assert!(model.binding_names().contains(rooted), "{label}");
            assert!(
                model.unresolved_command_handlers().contains(rooted),
                "{label}"
            );
        }
        for folded in ["::NAME_BINDING", "::NAME_BINDING::member"] {
            assert!(!model.binding_names().contains(folded), "{folded}");
            assert!(
                !model.unresolved_command_handlers().contains(folded),
                "{folded}"
            );
        }
    }
}

#[test]
fn jim_stock_admission_compares_exact_descriptor_components() {
    let dialect = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    for name in ["namespace", "::namespace", "json::decode", "::json::decode"] {
        assert_eq!(
            CommandRegistry::native_stock_registration_admission(dialect, name),
            Some(true),
            "{name}"
        );
    }
    for name in [
        "namespace::",
        "::::namespace",
        "json:::decode",
        "::json::::decode",
        ":::",
        "",
    ] {
        assert_eq!(
            CommandRegistry::native_stock_registration_admission(dialect, name),
            Some(false),
            "{name}"
        );
    }
    let mut registry = CommandRegistry::build_default();
    for name in ["namespace::", "::::namespace", "json:::decode"] {
        registry.insert(CommandSpec {
            name,
            ..CommandSpec::DEFAULT
        });
    }
    let model = registry.effective_semantics_for_dialect(dialect);
    for name in ["namespace::", "::::namespace", "json:::decode"] {
        assert!(model.command(name).is_none(), "{name}");
    }
    assert!(model.command("namespace").is_some());
    assert!(model.binding_names().contains("::namespace"));
    let unaudited = crate::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    );
    assert_eq!(
        CommandRegistry::native_stock_registration_admission(unaudited, "namespace"),
        None
    );
    assert!(
        registry
            .effective_semantics_for_dialect(unaudited)
            .binding_names()
            .is_empty()
    );
}

#[test]
fn fresh_dialect_bindings_follow_availability_and_package_provenance() {
    let registry = CommandRegistry::build_default();
    for release in tcl_dialect::TclVersion::ALL {
        let semantics = registry
            .effective_semantics_for_dialect(crate::InvocationDialect::for_version(release));
        assert!(semantics.binding_names().contains("::set"));
        assert_eq!(
            semantics.binding_names().contains("::dict"),
            release >= tcl_dialect::TclVersion::V8_5
        );
        assert!(!semantics.binding_names().contains("::tcltest::test"));
    }
    let modern = registry.effective_semantics_for_dialect(crate::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    ));
    assert!(modern.binding_names().contains("::alias"));
    assert!(!modern.binding_names().contains("::trace"));
    let older = registry.effective_semantics_for_dialect(crate::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    ));
    assert!(
        older.binding_names().is_empty(),
        "an unmeasured build cannot inherit current Jim presence"
    );
}

#[test]
fn platform_bindings_require_the_selected_native_ambient_profile() {
    for (profile_name, commands) in [
        ("f5-irules", &["::HTTP2::header"][..]),
        (
            "f5-tmsh",
            &["::tmsh::create", "::tmsh::list", "::tmsh::log"][..],
        ),
    ] {
        let owner = crate::model::ingress::static_context_for(profile_name);
        let registry = owner.commands();
        let profile = registry.profile().expect("selected platform profile");
        let dialect = crate::InvocationDialect::of_profile(profile);
        let semantics = registry.effective_semantics_for_dialect(dialect);
        for command in commands {
            assert!(
                semantics.binding_names().contains(*command),
                "{profile_name}: {command}"
            );
        }
        assert!(!semantics.binding_names().contains("::snit::type"));
        let other = registry.effective_semantics_for_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        for command in commands {
            assert!(
                !other.binding_names().contains(*command),
                "foreign profile: {command}"
            );
        }
    }
}

#[test]
fn receiver_vocabulary_does_not_allocate_global_dispatchers() {
    let owner = crate::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
    let semantics = registry.effective_semantics_for_dialect(dialect);
    assert!(!semantics.binding_names().contains("::my"));
    assert!(!semantics.binding_names().contains("::self"));
    assert!(semantics.binding_names().contains("::oo::Helpers::self"));
}

#[test]
fn exact_completion_counts_positionals_after_known_selector_and_terminator() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = CommandRegistry::build_default();
    let extra_names = [Literal("create"), Literal("--"), Dynamic, Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&extra_names),
            None,
        ),
        Some(ExactInvocationCompletion::Tcl(crate::CompletionCode::Error)),
        "dynamic positional bytes do not erase a known selector and excess count"
    );
    let unknown_option = [Literal("create"), Dynamic, Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&unknown_option),
            None,
        ),
        None,
        "an unknown leading option cannot establish rejection by count"
    );
    let one_name = [Literal("create"), Literal("--"), Dynamic];
    assert_eq!(
        registry.exact_invocation_completion_words(
            "interp",
            InvocationArguments::structured(&one_name),
            None,
        ),
        None,
        "accepted arity does not prove successful creation"
    );
}

#[test]
fn exact_process_exit_obeys_explicit_snapshot_without_catalogue_profile() {
    let registry = CommandRegistry::build_default();
    for release in tcl_dialect::TclVersion::ALL {
        let arguments = crate::InvocationArguments::literals(&["4294967296"])
            .with_dialect(crate::InvocationDialect::for_version(release));
        assert_eq!(
            registry.exact_invocation_completion_words("exit", arguments, None),
            Some(if release >= tcl_dialect::TclVersion::V9_0 {
                ExactInvocationCompletion::ProcessExit
            } else {
                ExactInvocationCompletion::Tcl(crate::completion::CompletionCode::Error)
            })
        );
    }
    let arguments = crate::InvocationArguments::literals(&["18446744073709551616"]).with_dialect(
        crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        )),
    );
    assert_eq!(
        registry.exact_invocation_completion_words("exit", arguments, None),
        Some(ExactInvocationCompletion::Tcl(
            crate::completion::CompletionCode::Other(6)
        ))
    );
}

#[test]
#[allow(clippy::too_many_lines)] // descriptor layout matrix
fn case_list_invocation_layout_is_registry_owned() {
    use crate::spec::CaseMatchMode;
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    let Some((_, two_arg)) = reg.case_invocation(
        "switch",
        &["-glob", "default {}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
    ) else {
        panic!("two-argument case form must parse");
    };
    assert_eq!(two_arg.subject_index, Some(0));
    assert_eq!(two_arg.mode, CaseMatchMode::Exact);
    assert_eq!(two_arg.clause_list_index, Some(1));

    let Some((_, options)) = reg.case_invocation(
        "switch",
        &["-glob", "-nocase", "--", "subject", "p {}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.0")),
    ) else {
        panic!("option-bearing case form must parse");
    };
    assert_eq!(options.subject_index, Some(3));
    assert_eq!(options.mode, CaseMatchMode::Glob);
    assert!(options.nocase);

    let tcl91 = crate::model::ingress::static_context_for("tcl9.1").commands();
    let Some((_, integer)) = tcl91.case_invocation(
        "switch",
        &["-integer", "--", "1", "1 {set x 1}"],
        Some(SurfaceQuery::core(Family::Tcl, "9.1")),
    ) else {
        panic!("Tcl 9.1 integer switch must retain its case-list body");
    };
    assert_eq!(integer.mode, CaseMatchMode::Other);
    assert_eq!(integer.clause_list_index, Some(3));
    for args in [
        ["-integer", "-nocase", "1", "1 {set x 1}"],
        ["-nocase", "-integer", "1", "1 {set x 1}"],
    ] {
        assert!(
            tcl91
                .case_invocation(
                    "switch",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                )
                .is_none(),
            "integer and nocase must be incompatible: {args:?}"
        );
        assert!(
            tcl91
                .arg_indices_for_role("switch", &args, ArgRole::Body)
                .is_empty(),
            "invalid integer/nocase form must expose no body: {args:?}"
        );
    }
    for args in [
        ["-integer", "-glob", "1", "1 {set x 1}"],
        ["-glob", "-integer", "1", "1 {set x 1}"],
        ["-integer", "-integer", "1", "1 {set x 1}"],
    ] {
        assert!(
            tcl91
                .case_invocation(
                    "switch",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                )
                .is_none(),
            "multiple match modes must be rejected: {args:?}"
        );
        assert!(
            tcl91
                .arg_indices_for_role("switch", &args, ArgRole::Body)
                .is_empty(),
            "multiple match modes must expose no body: {args:?}"
        );
    }
    assert_eq!(
        tcl91.arg_indices_for_role(
            "switch",
            &["-integer", "--", "1", "1 {set x 1}"],
            ArgRole::Body,
        ),
        vec![3]
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern", "body", "orphan"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .is_none()
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern {} orphan"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none()
    );

    let tcl84 = crate::model::ingress::static_context_for("tcl8.4").commands();
    for two_arg_switch in [
        ["-regexp", "default {puts hit}"],
        ["--", "default {puts hit}"],
    ] {
        assert!(
            tcl84
                .case_invocation(
                    "switch",
                    &two_arg_switch,
                    Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                )
                .is_none(),
            "Tcl 8.4 scans the option-like first word and rejects the missing subject: {two_arg_switch:?}"
        );
        assert!(
            tcl84
                .arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body)
                .is_empty(),
            "Tcl 8.4 must not assign the second word a body role: {two_arg_switch:?}"
        );
        assert_eq!(
            tcl84
                .resolve_option_terminator(
                    "switch",
                    &two_arg_switch,
                    Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                )
                .expect("switch declares --")
                .reserved_trailing_words,
            0,
            "Tcl 8.4 must scan both words in the option-like two-word form: {two_arg_switch:?}"
        );
    }
    for args in [
        &["subject", "default {puts hit}"][..],
        &["-regexp", "subject", "default {puts hit}"][..],
        &["--", "-subject", "default {puts hit}"][..],
    ] {
        assert!(
            tcl84
                .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
                .is_some(),
            "Tcl 8.4 accepts the unambiguous switch case-list form: {args:?}"
        );
        assert_eq!(
            tcl84.arg_indices_for_role("switch", args, ArgRole::Body),
            vec![args.len() - 1],
            "Tcl 8.4 assigns the final case-list word a body role: {args:?}"
        );
    }
    assert!(
        tcl84
            .case_invocation(
                "switch",
                &["-nocase", "subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            )
            .is_none()
    );
    let tcl85 = crate::model::ingress::static_context_for("tcl8.5").commands();
    for (dialect, availability) in [
        ("tcl8.5", Some(SurfaceQuery::core(Family::Tcl, "8.5"))),
        ("tcl8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        ("tcl9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
    ] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        for two_arg_switch in [
            ["-regexp", "default {puts hit}"],
            ["--", "default {puts hit}"],
        ] {
            assert!(
                registry
                    .case_invocation("switch", &two_arg_switch, availability)
                    .is_some(),
                "{dialect} accepts the optionless two-word switch form: {two_arg_switch:?}"
            );
            assert_eq!(
                registry.arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body),
                vec![1],
                "{dialect} assigns the clause-list word a body role: {two_arg_switch:?}"
            );
            assert_eq!(
                registry
                    .resolve_option_terminator("switch", &two_arg_switch, availability)
                    .expect("switch declares --")
                    .reserved_trailing_words,
                2,
                "{dialect} reserves the subject and clause-list words: {two_arg_switch:?}"
            );
        }
    }
    assert!(
        tcl85
            .case_invocation(
                "switch",
                &["-nocase", "subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.5")),
            )
            .is_some()
    );
    let default = CommandRegistry::build_default();
    assert!(
        default
            .case_invocation(
                "switch",
                &["subject", "pattern {}"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_some()
    );
    assert!(
        default
            .case_invocation(
                "switch",
                &["--", "-x", "-x {puts hit} default {puts miss}"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_some(),
        "the descriptor's -- terminator keeps a hyphenated switch subject positional"
    );
    assert!(
        reg.case_invocation(
            "switch",
            &["subject", "pattern -"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none()
    );

    let expect = crate::model::ingress::static_context_for("expect").commands();
    assert!(
        expect
            .case_invocation(
                "expect",
                &["\"password:\" {send pw} -re {ye+s} {send yes} timeout {puts slow}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_some(),
        "clause-leading flags must not break valid Expect pattern/body pairs"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["{-re} {send literal}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_some(),
        "a braced flag-shaped pattern is literal text, not a clause flag"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-re {ye+s}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "Expect permits a final pattern without an action"
    );
    for args in [
        ["-timeout", "5", "ready {action}"],
        ["-i", "spawn", "ready {action}"],
    ] {
        let (_, invocation) = expect
            .case_invocation(
                "expect",
                &args,
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("outer Expect value option followed by a final pattern");
        assert_eq!(invocation.clause_list_index, None, "{args:?}");
        assert_eq!(invocation.inline_clause_start, Some(2), "{args:?}");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, invocation.inline_clause_start.unwrap())
            .expect("one action-less inline clause");
        assert_eq!(clauses.len(), 1, "{args:?}");
        assert_eq!(clauses[0].pattern_index, 2, "{args:?}");
        assert_eq!(clauses[0].body_index, None, "{args:?}");
    }
    for pattern in ["#", ";"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[&format!(
                        "{pattern} {{send literal}} default {{send other}}"
                    )],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_some(),
            "{pattern:?} is a literal Tcl list pattern, not script syntax"
        );
    }
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-timeout"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_none(),
        "a value-taking clause flag without a value/pattern/body is invalid"
    );
}

#[test]
fn inline_expect_case_flags_and_actions_are_registry_owned() {
    let expect = crate::model::ingress::static_context_for("expect").commands();
    let args = [
        "-re",
        "{ye+s}",
        "{send yes}",
        "-timeout",
        "5",
        "timeout",
        "{send slow}",
    ];
    let (_, inline) = expect
        .case_invocation(
            "expect",
            &args,
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("inline Expect flags and value flags must parse");
    let clauses = crate::CaseListSpec::EXPECT
        .inline_clauses(&args, inline.inline_clause_start.expect("inline form"))
        .expect("inline clauses");
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].pattern_index, 1);
    assert_eq!(clauses[0].body_index, Some(2));
    assert_eq!(clauses[0].mode, crate::spec::CaseMatchMode::Regexp);
    assert_eq!(clauses[1].pattern_index, 5);
    assert_eq!(clauses[1].body_index, Some(6));
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-not", "ready", "{send ok}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "unique Expect flag abbreviations must retain the action body"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Expect oracle layout matrix
fn expect_inline_flag_table_matches_the_oracle() {
    let expect = crate::model::ingress::static_context_for("expect").commands();
    for flag in [
        "-glob",
        "-regexp",
        "-exact",
        "-notransfer",
        "-nocase",
        "-i",
        "-indices",
        "-iread",
        "-timestamp",
        "-nobrace",
    ] {
        let args = if matches!(flag, "-i") {
            vec![flag, "spawn", "pattern", "{action}"]
        } else {
            vec![flag, "pattern", "{action}"]
        };
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "canonical {flag} must parse"
        );
    }
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-timeout", "5", "pattern", "{action}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some()
    );
    for flag in ["-gl", "-re", "-ex", "-not"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "unique abbreviation {flag} must parse"
        );
    }
    for flag in ["-g", "-r", "-e", "-noc", "-nob"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "unique canonical-prefix {flag} must remain an inline clause flag"
        );
    }
    for flag in ["-n", "-bogus"] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &[flag, "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "ambiguous or unknown {flag} must invalidate the invocation"
        );
    }
    let args = ["--", "-re", "{action}"];
    let (_, invocation) = expect
        .case_invocation(
            "expect",
            &args,
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("-- makes -re a pattern");
    let clauses = crate::CaseListSpec::EXPECT
        .inline_clauses(&args, invocation.inline_clause_start.expect("inline"))
        .expect("clause");
    assert_eq!(clauses[0].pattern_index, 1);
    assert_eq!(clauses[0].body_index, Some(2));
    let (_, omitted) = expect
        .case_invocation(
            "expect",
            &["-re", "pattern"],
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("omitted final action is valid");
    assert_eq!(
        crate::CaseListSpec::EXPECT
            .inline_clauses(&["-re", "pattern"], omitted.inline_clause_start.unwrap())
            .unwrap()[0]
            .body_index,
        None
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-nobrace", "{pattern}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                )
            )
            .is_some(),
        "-nobrace makes one braced word an action-less pattern"
    );
    let (_, brace) = expect
        .case_invocation(
            "expect",
            &["-brace", "{default {return FOLDED}}"],
            Some(
                SurfaceQuery::core(Family::Tcl, "8.6")
                    .with_packages(&[PackageFloor::named("expect")]),
            ),
        )
        .expect("exact -brace selects a clause list");
    assert_eq!(brace.clause_list_index, Some(1));
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-b", "{default {return FOLDED}}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_none(),
        "-brace is exact-only, so -b is not a clause flag abbreviation"
    );
    assert!(
        expect
            .case_invocation(
                "expect",
                &["-brac", "{default {return FOLDED}}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")])
                ),
            )
            .is_none(),
        "-brace must not accept a near-complete prefix either"
    );
    for args in [
        &["-timeout", "5", "-brace", "{default {return FOLDED}}"] as &[&str],
        &["-i", "spawn", "-brace", "{default {return FOLDED}}"],
        &["-brace", "{default {return FOLDED}}", "extra"],
    ] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "force-list selector must be first and have one remainder: {args:?}"
        );
    }
}

#[test]
fn subjectless_case_list_defers_clause_flags_after_outer_value_options() {
    use crate::hover::{OptionSpec, OptionValue};
    use crate::spec::CaseListSpec;

    // This deliberately does not name `expect`: a future descriptor with
    // one value-taking outer option and one overlapping clause flag must
    // use the same precedence rule. The body is the observable proof that
    // `-clause` was not swallowed by the outer scan.
    let case = CaseListSpec {
        subject_args: 0,
        two_arg_optionless_surface: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &["-clause", "--"],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: Some("--"),
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: true,
        keyword_patterns: &[],
        keyword_patterns_require_final: false,
        exhaustive_keyword_patterns: &[],
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
        default_mode: crate::spec::CaseMatchMode::Exact,
        pattern_words: crate::spec::PatternWords::Single,
    };
    let options = [
        OptionSpec {
            name: "-outer",
            value: OptionValue::value("value"),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-clause",
            ..OptionSpec::DEFAULT
        },
    ];
    let option_refs: Vec<&OptionSpec> = options.iter().collect();
    let args = ["-outer", "value", "-clause", "pattern", "{body}"];
    let invocation = case
        .invocation(
            &args,
            &option_refs,
            Some(SurfaceQuery::any_release(Family::Tcl)),
        )
        .expect("the outer value must precede, not consume, the clause flag");
    assert_eq!(invocation.inline_clause_start, Some(2));
    let clauses = case
        .inline_clauses(&args, invocation.inline_clause_start.unwrap())
        .expect("the deferred flag must retain the action body");
    assert_eq!(clauses[0].pattern_index, 3);
    assert_eq!(clauses[0].body_index, Some(4));
}

#[test]
fn case_list_invocation_abstains_on_empty_and_truncated_shapes() {
    let switch = crate::model::ingress::static_context_for("tcl9.0").commands();
    for args in [
        &[][..],
        &["subject"][..],
        &["subject", ""][..],
        &["-regexp"][..],
        &["-regexp", "subject"][..],
        &["--"][..],
    ] {
        assert!(
            switch
                .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
                .is_none(),
            "truncated switch invocation must abstain: {args:?}",
        );
    }

    let expect = crate::model::ingress::static_context_for("expect").commands();
    for args in [
        &[][..],
        &["-brace"][..],
        &["-nobrace"][..],
        &["-timeout"][..],
        &["-timeout", "5"][..],
        &[""][..],
    ] {
        assert!(
            expect
                .case_invocation(
                    "expect",
                    args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "truncated Expect invocation must abstain: {args:?}",
        );
    }

    // Exercise the generic selector branch independently of either
    // built-in descriptor. The selector is intentionally outside the
    // clause-flag vocabulary, so a complete call consumes it as shape
    // syntax while a missing subject/body must simply abstain.
    let custom = crate::CaseListSpec {
        clause_force_inline_flag: Some("-inline"),
        allow_omitted_final_body: false,
        ..crate::CaseListSpec::EXPECT
    };
    for args in [
        &[][..],
        &["-inline"][..],
        &["-inline", "subject"][..],
        &[""][..],
    ] {
        assert!(
            custom
                .invocation(args, &[], Some(SurfaceQuery::any_release(Family::Tcl)))
                .is_none(),
            "truncated custom selector invocation must abstain: {args:?}",
        );
    }
    assert!(
        custom
            .invocation(
                &["-inline", "pattern", "{body}"],
                &[],
                Some(SurfaceQuery::any_release(Family::Tcl)),
            )
            .is_some(),
        "complete custom inline-selector invocation remains valid",
    );

    // A dynamic two-word switch cannot expose a static body region, but
    // asking for roles must remain a conservative abstention rather than
    // a panic. The compiler's W304 regression separately proves Tcl's
    // optionless two-word rule is retained for diagnostics.
    assert_eq!(
        switch
            .arg_indices_for_role("switch", &["$subject", "$cases"], ArgRole::Body)
            .len(),
        0,
    );
}

/// The whole registry-declared boundary surface, resolved by name: every
/// command that widens a file's caller set reports its kind, a
/// `::`-qualified spelling resolves the same, and a command that does
/// neither reports nothing.
#[test]
fn unit_linkage_covers_every_declared_boundary_command() {
    let reg = CommandRegistry::build_default();
    let empty = None;
    for (name, args, want) in [
        ("source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("::source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("load", &["libx.so"][..], Traits::LOADS_EXTERNAL_UNIT),
        ("auto_load", &["helper"][..], Traits::LOADS_EXTERNAL_UNIT),
        (
            "auto_import",
            &["::lib::*"][..],
            Traits::LOADS_EXTERNAL_UNIT,
        ),
        // `namespace import` is deliberately not a boundary — see its
        // own spec comment.
        (
            "namespace",
            &["import", "::lib::helper"][..],
            Traits::empty(),
        ),
        (
            "namespace",
            &["export", "helper"][..],
            Traits::EXPORTS_COMMAND,
        ),
        (
            "namespace",
            &["ensemble", "create"][..],
            Traits::EXPORTS_COMMAND,
        ),
        ("namespace", &["eval", "::ns", "{}"][..], Traits::empty()),
        ("set", &["x", "1"][..], Traits::empty()),
        ("no_such_command_xyz", &["source"][..], Traits::empty()),
    ] {
        assert_eq!(
            reg.unit_linkage(name, args, empty),
            want,
            "unit_linkage({name}, {args:?})"
        );
    }
}

/// `SAFE_INTERP_HIDDEN` has its own bit, distinct from `TRANSFERS_CONTROL`:
/// every `break`/`continue`/`tailcall`/`yield` reads as safe-interp-hidden
/// and every `cd`/`exec`/`glob`/… reads as control-transferring, which
/// `FRAME_SENSITIVE_TRAITS` consumes directly.
#[test]
fn safe_interp_hidden_no_longer_aliases_transfers_control() {
    let reg = CommandRegistry::build_default();
    assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
    let cd = reg.get("cd").expect("cd is registered");
    assert!(cd.traits.contains(Traits::SAFE_INTERP_HIDDEN));
    assert!(!cd.traits.contains(Traits::TRANSFERS_CONTROL));
    assert!(!reg.is_frame_sensitive("cd"));
    let brk = reg.get("break").expect("break is registered");
    assert!(brk.traits.contains(Traits::TRANSFERS_CONTROL));
    assert!(!brk.traits.contains(Traits::SAFE_INTERP_HIDDEN));
}

/// `declares_command_at` answers "is this in a fresh
/// interpreter's command table", which is strictly narrower than `get`'s
/// "is this a spelling a call site may write".
///
/// Every row is oracle-verified on tclsh 9.0.4 and 8.6.14 via
/// `info commands <name>` in a fresh `interp create`.
#[test]
fn declares_command_at_is_the_fresh_interpreter_command_table() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    // `info commands ::set` -> ::set
    for name in ["set", "::set", "puts", "::puts", "if", "foreach"] {
        assert!(reg.declares_command_at(name), "{name} is a global builtin");
    }
    // `info commands ::tcl::mathop::+` -> ::tcl::mathop::+
    for name in [
        "tcl::mathop::+",
        "::tcl::mathop::+",
        "oo::define",
        "::oo::define",
    ] {
        assert!(
            reg.declares_command_at(name),
            "{name} is a namespaced builtin"
        );
    }
    // `info commands ::+` -> {} and `+ 1 2` -> invalid command name "+".
    // `get` still answers `Some` for these — that is the whole point.
    for name in ["+", "::+", "eq", "::eq", "in", "::in"] {
        assert!(
            reg.get(name).is_some(),
            "{name} must stay a resolvable *spelling*",
        );
        assert!(
            !reg.declares_command_at(name),
            "{name} is only callable after `namespace import ::tcl::mathop::*`",
        );
    }
    // A name nothing declares.
    for name in ["notACommand", "::notACommand", "define"] {
        assert!(!reg.declares_command_at(name), "{name}");
    }
}

/// The answer is per dialect, because the command table is: `HTTP::uri`
/// exists for iRules and nowhere else.
#[test]
fn declares_command_at_is_dialect_specific() {
    assert!(
        crate::model::ingress::static_context_for("f5-irules")
            .commands()
            .declares_command_at("HTTP::uri"),
        "iRules declares HTTP::uri",
    );
    assert!(
        !crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .declares_command_at("HTTP::uri"),
        "plain Tcl does not",
    );
}

#[test]
fn smoke_lookup_set_command() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get("set").expect("`set` must be a known command");
    assert!(
        spec.arity.min <= 1 && spec.arity.max >= 1,
        "set must accept at least a variable name: {:?}",
        spec.arity
    );
}
