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

//! Native registration descriptors, caches, facts and compiler rows.

use super::*;

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
        assert_eq!(super::super::spec_pack_of(root), Some("tcl"));
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
        super::super::spec_pack_of(authored),
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

#[test]
fn a_jim_registration_without_a_compiler_declares_its_hook_absent() {
    // Jim compiles no commands, so the vm's captured entry row must read Absent
    // for `namespace` rather than Unknown, which preflight refuses.
    let registry = CommandRegistry::build_default();
    let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    let namespace = registry
        .native_compilation_for_registration("namespace", jim)
        .expect("a Jim registration carries a hook-presence descriptor");
    assert_eq!(namespace.compiler_hook_presence(jim), Some(false));
}
