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

//! Effective semantics: the cache and its invalidation, binding rosters and stock admission.

use super::*;

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
                .stock_native_implementation_slots(tcl_native_model::InvocationDialect::of_profile(
                    profile,
                ))
                .into_iter()
                .map(|lookup| lookup.slot.to_owned()),
        );
    }
    if let Some(profile) = registry.profile()
        && let Some(support) = registry
            .native_class_factory_recipe(
                "oo::configurable",
                tcl_native_model::InvocationDialect::of_profile(profile),
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
    let native = driver.commands().effective_semantics_for_dialect(
        tcl_native_model::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_79,
        )),
    );
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
        registry.effective_semantics_for_dialect(tcl_native_model::InvocationDialect::for_version(
            version,
        ))
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
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
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
    let unaudited = tcl_native_model::InvocationDialect::of_point(
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
        let semantics = registry.effective_semantics_for_dialect(
            tcl_native_model::InvocationDialect::for_version(release),
        );
        assert!(semantics.binding_names().contains("::set"));
        assert_eq!(
            semantics.binding_names().contains("::dict"),
            release >= tcl_dialect::TclVersion::V8_5
        );
        assert!(!semantics.binding_names().contains("::tcltest::test"));
    }
    let modern =
        registry.effective_semantics_for_dialect(tcl_native_model::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
    assert!(modern.binding_names().contains("::alias"));
    assert!(!modern.binding_names().contains("::trace"));
    let older =
        registry.effective_semantics_for_dialect(tcl_native_model::InvocationDialect::of_point(
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
        let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
        let semantics = registry.effective_semantics_for_dialect(dialect);
        for command in commands {
            assert!(
                semantics.binding_names().contains(*command),
                "{profile_name}: {command}"
            );
        }
        assert!(!semantics.binding_names().contains("::snit::type"));
        let other = registry.effective_semantics_for_dialect(
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
    let dialect = tcl_native_model::InvocationDialect::of_profile(registry.profile().unwrap());
    let semantics = registry.effective_semantics_for_dialect(dialect);
    assert!(!semantics.binding_names().contains("::my"));
    assert!(!semantics.binding_names().contains("::self"));
    assert!(semantics.binding_names().contains("::oo::Helpers::self"));
}
