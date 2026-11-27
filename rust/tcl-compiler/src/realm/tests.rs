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
fn fresh_hosted_realm_keeps_loader_event_and_source_owners() {
    use std::sync::Arc;
    let profile = tcl_dialect::DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "::when HTTP_REQUEST {pool p}";
    let realm = super::document_realm_bindings_for_fresh_hosted_source(source, &input).unwrap();
    let loader = realm.invocation_at_source("::when", 0);
    assert_eq!(
        loader.invocation_realm(),
        Some(tcl_dialect::model::InvocationRealm::RuleLoader)
    );
    assert!(
        loader
            .proved_target()
            .is_some_and(|target| target.registry_backed && target.command == "::when")
    );
    let pool_at = u32::try_from(source.find("pool").unwrap()).unwrap();
    let event = realm.invocation_at_source("pool", pool_at);
    assert_eq!(
        event.invocation_realm(),
        Some(tcl_dialect::model::InvocationRealm::InterpreterRuntime)
    );
    assert!(
        event
            .proved_target()
            .is_some_and(|target| target.registry_backed && target.command == "::pool")
    );
    assert!(realm.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config));
    assert!(!realm.matches_original_source_image(
        &tcl_lexer::SourceImage::document("::when HTTP_REQUEST {pool q}"),
        config
    ));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let mismatched = crate::analyser::ResolvedAnalysisInput::new(profile, profile, foreign, config);
    assert!(super::document_realm_bindings_for_fresh_hosted_source(source, &mismatched).is_none());
}

#[test]
fn original_module_realm_reuses_positioned_inventory_across_native_engine_profiles() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let source = "namespace eval N {proc p {} {return OK}}; interp alias {} forwarding {} N::p; rename forwarding moved; moved";
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let module = crate::lowering::lower_to_ir_with_config(
            source,
            registry,
            tcl_lexer::LexerConfig::default(),
        );
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let bindings = retained.positioned_for_module(&module, registry).unwrap();
        let projected = document_realm_bindings_for_module(&module, registry).unwrap();
        assert!(
            std::sync::Arc::ptr_eq(
                projected.bindings.source_origin().unwrap(),
                bindings.source_origin().unwrap(),
            ),
            "{engine}"
        );
        assert_eq!(&projected.bindings, bindings, "{engine}");
        let independent = document_realm_bindings_with_source_entry(
            source,
            module.lexer_config,
            registry,
            &module.source_entry,
        );
        assert_eq!(*projected, independent, "{engine}");
    }
}

#[test]
fn original_module_realm_refuses_missing_and_same_store_foreign_availability() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands())),
    );
    assert!(std::sync::Arc::ptr_eq(current.commands(), older.commands()));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&current),
        config,
    );
    let source = "proc retained {} {set original VALUE}; retained";
    let original = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(current.commands(), config)
            .with_resolved_analysis_input(input.clone()),
        source,
    );
    let realm = document_realm_bindings_for_module(&original, current.commands()).unwrap();
    assert!(realm.matches_resolved_analysis_input(&input));
    let mut missing = original.clone();
    missing.retained_source_bindings = None;
    assert!(document_realm_bindings_for_module(&missing, current.commands()).is_none());
    missing = original.clone();
    missing.source_metadata_input = None;
    assert!(document_realm_bindings_for_module(&missing, current.commands()).is_none());
    let old_input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, older, config);
    let mut changed = original.clone();
    changed.source_metadata_input = Some(old_input.clone());
    assert!(document_realm_bindings_for_module(&changed, current.commands()).is_none());
    changed.source_entry.logical_source_input = Some(old_input.clone());
    changed.source_entry.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
            &old_input,
        ));
    assert!(document_realm_bindings_for_module(&changed, current.commands()).is_none());
    assert!(
        document_realm_bindings_for_module(&original, current.commands())
            .unwrap()
            .matches_resolved_analysis_input(&input)
    );
}

#[test]
fn resolved_input_preserves_math_policy_without_catalogue_default() {
    // naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    use tcl_registry::mathfunc::NativeMathFunctionDispatch;
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let environment = crate::environment_ingress::resolve_environment(name);
        let profile = environment.analyser_profile();
        let context = environment
            .context_registry(&tcl_registry::model::KeyedVersions::default(), 0)
            .expect("the original zero-overlay environment is installed");
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            environment.unit_profile(),
            context,
            config,
        );
        let source = "expr {Pi()}\n";
        let realm = document_realm_bindings_with_resolved_input(source, &input, None);
        let registry = input.context_registry();
        let image = tcl_lexer::SourceImage::document(source);
        let occurrences = realm
            .source_bindings_ref()
            .original_math_functions_in_source(
                registry.commands(),
                &image,
                config,
                tcl_lexer::Span::new(6, 8),
            );
        let [occurrence] = occurrences.as_slice() else {
            panic!("{name}: retained complete source expression: {occurrences:?}");
        };
        assert_eq!(occurrence.bytes(), b"Pi", "{name}");
        assert_eq!(occurrence.span(), tcl_lexer::Span::new(6, 8));
        assert_eq!(
            occurrence.dispatch(),
            if matches!(name, "tcl8.4" | "jim") {
                NativeMathFunctionDispatch::FixedTable
            } else {
                NativeMathFunctionDispatch::CommandTable
            }
        );
        assert_eq!(
            occurrence.name_policy().recipe(),
            tcl_native_model::InvocationDialect::of_profile(input.unit_profile())
                .native_name_protocol()
                .unwrap()
        );
        assert!(occurrence.matches_source(&image, config, registry.commands()));
        assert!(!occurrence.matches_source(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            config,
            registry.commands(),
        ));
        let mut foreign = config;
        foreign.strict_quoting ^= true;
        assert!(!occurrence.matches_source(&image, foreign, registry.commands()));
        assert!(occurrence.command_reference().is_none());
    }
}

#[test]
fn explicit_profile_retains_native_policy_independently_of_lexical_rules() {
    let registry = CommandRegistry::build_default();
    for name in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).expect("native core");
        let realm = document_realm_bindings("set x value", profile, &registry);
        assert_eq!(
            realm
                .invocation_at_source("set", 0)
                .variable_context
                .invocation_dialect,
            Some(tcl_native_model::InvocationDialect::of_profile(profile))
        );
    }
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let realm = document_realm_bindings("set x value", profile, &registry);
    assert_eq!(
        realm
            .invocation_at_source("set", 0)
            .variable_context
            .invocation_dialect
            .and_then(|dialect| dialect.tcl_version),
        Some(tcl_dialect::TclVersion::V9_0)
    );
}

fn map_for(src: &str) -> CommandBindingRealm {
    let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
    document_realm_bindings(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        registry,
    )
}

fn irules_map_for(src: &str) -> CommandBindingRealm {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    document_realm_bindings(src, tcl_dialect::DialectProfile::irules(), registry)
}

/// Offset just past the first line of `src`, i.e. "after statement 1".
fn after_first_line(src: &str) -> u32 {
    u32::try_from(src.find('\n').map_or(src.len(), |i| i + 1)).unwrap_or(0)
}

/// The [`BindingKnowledge`] view for head-identity consumers: document
/// facts answer `Must(Spec)` / `Must(Document)` / `Absent`; with no fact
/// the environment answers, and world policy decides `Absent` vs
/// `Unknown` for an unprovided name.
#[test]
fn knowledge_composes_facts_over_the_environment() {
    let generation = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let context = generation.context();
    let registry = generation.commands();
    let src = "rename format origfmt\nproc lindex {args} { return MINE }\nrename lsort {}\n";
    let map = document_realm_bindings(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        registry,
    );
    let end = u32::try_from(src.len()).unwrap_or(0);
    // A proven rename chain is a spec-keyed Must — the I4 licence.
    let origfmt = map.knowledge_at(context, registry, "origfmt", end);
    assert_eq!(
        origfmt.proved_spec().map(SpecKey::name),
        Some("format"),
        "a proven chain carries the registry spec"
    );
    // A user-proc takeover exists but licenses no hook.
    let lindex = map.knowledge_at(context, registry, "lindex", end);
    assert!(lindex.is_proved());
    assert_eq!(lindex.proved_spec(), None, "no catalogue semantics (I4)");
    // A deletion is honest absence.
    assert_eq!(
        map.knowledge_at(context, registry, "lsort", end),
        BindingKnowledge::Absent
    );
    // No fact: the environment resolves…
    let set = map.knowledge_at(context, registry, "set", end);
    assert_eq!(set.proved_spec().map(SpecKey::name), Some("set"));
    // …and an unprovided name under tcl9.0's open world stays Unknown
    // (a `package require` can introduce it at load time).
    assert_eq!(
        map.knowledge_at(context, registry, "no-such-cmd", end),
        BindingKnowledge::Unknown
    );
}

/// Under a closed world (`f5-irules`) an unprovided name is proved
/// `Absent` — the static decidability iRules derives rather than assumes.
#[test]
fn a_closed_world_proves_absence() {
    let generation = tcl_registry::model::ingress::static_context_for("f5-irules");
    let context = generation.context();
    let registry = generation.commands();
    let map = CommandBindingRealm::default();
    assert_eq!(
        map.knowledge_at(context, registry, "no-such-cmd", 0),
        BindingKnowledge::Absent
    );
    // The disabled mutators are likewise absent from the surface…
    assert_eq!(
        map.knowledge_at(context, registry, "interp", 0),
        BindingKnowledge::Absent
    );
    // …while a provided command proves Must.
    assert!(
        map.knowledge_at(context, registry, "when", 0)
            .proved_spec()
            .is_some()
    );
}

#[test]
fn static_rename_moves_the_identity_and_clears_the_old_name() {
    let src = "rename format origfmt\norigfmt %d 7\nformat x\n";
    let map = map_for(src);
    let at = after_first_line(src);
    assert_eq!(map.resolve("origfmt", at), RealmBinding::Command("format"));
    // The `::`-qualified spelling of the new name resolves alike.
    assert_eq!(
        map.resolve("::origfmt", at),
        RealmBinding::Command("format")
    );
    // The old name is gone from the rename onwards …
    assert_eq!(map.resolve("format", at), RealmBinding::Rebound);
    // … but calls *before* it still see the built-in.
    let src = "format {%08x} 42\nrename format origfmt\n";
    let map = map_for(src);
    assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
    assert_eq!(
        map.resolve("format", after_first_line(src)),
        RealmBinding::Command("format"),
        "the rename entry still sees the old command before its mutation"
    );
    assert_eq!(
        map.resolve("format", u32::try_from(src.len()).unwrap()),
        RealmBinding::Rebound,
        "the successful rename continuation has retired the old slot"
    );
}

#[test]
fn interp_alias_aliases_only_the_argument_preserving_form() {
    let src = "interp alias {} myfmt {} format\n";
    let map = map_for(src);
    let at = after_first_line(src);
    assert_eq!(map.resolve("myfmt", at), RealmBinding::Command("format"));
    assert_eq!(map.resolve("::myfmt", at), RealmBinding::Command("format"));

    // Pre-bound arguments shift every index — the layout cannot be reused.
    let src = "interp alias {} pad {} format %08x\n";
    let map = map_for(src);
    assert_eq!(
        map.resolve("pad", after_first_line(src)),
        RealmBinding::Rebound
    );
}

#[test]
fn dynamic_bindings_abstain() {
    for src in [
        "rename $old myfmt\n",
        "interp alias {} $n {} format\n",
        "interp alias {} myfmt {} $t\n",
    ] {
        let map = map_for(src);
        let at = after_first_line(src);
        assert_eq!(
            map.resolve("myfmt", at),
            RealmBinding::Rebound,
            "an unresolved target must not acquire a registry identity: {src}"
        );
        // Missing argv values prevent the transition from being reached;
        // this cannot donate a new target identity for the alias.
        assert!(
            map.invocation_at_source("myfmt", at)
                .proved_target()
                .is_none()
        );
    }
}

#[test]
fn a_child_interpreter_alias_changes_nothing_here() {
    let src = "interp alias slave myfmt {} format\n";
    let map = map_for(src);
    assert!(map.is_empty(), "a foreign srcPath states no fact here");
}

/// Child visibility operations do not rebind the parent's stock commands.
/// Creating a named child also installs its own parent command.
#[test]
fn a_safe_interpreters_hidden_command_states_nothing_here() {
    // naming.interpreter.original-created-parent-command
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command.md
    // Native CLI observes named child creation/deletion and handle/path
    // evaluation. These assertions exercise source command-table advice;
    // they do not derive native tokens or effects from the CLI result.
    let creation = "interp create -safe s\n";
    let map = map_for(creation);
    assert!(!map.is_empty(), "creation installs a parent command");
    let offset = u32::try_from(creation.len()).unwrap();
    assert_eq!(map.binding_at("s", offset), RealmBindingFact::Rebound);
    assert_eq!(
        map.resolve("format", offset).spec_name(),
        map_for("").resolve("format", 0).spec_name(),
        "creating a child preserves the parent's stock format binding",
    );
    for src in [
        "interp hide s format\n",
        "interp expose s format\n",
        "interp invokehidden s format %d 7\n",
    ] {
        let map = map_for(src);
        assert!(
            map.is_empty(),
            "child visibility is a separate table: {src}"
        );
        assert_eq!(
            map.resolve("format", u32::try_from(src.len()).unwrap())
                .spec_name(),
            map_for("").resolve("format", 0).spec_name(),
        );
    }
}

/// TN — a malformed call states nothing rather than half a fact.
#[test]
fn malformed_binding_calls_abstain() {
    for src in [
        // Arity too short for a rename / an alias creation.
        "rename\n",
        "rename format\n",
        "interp alias\n",
        "interp alias {}\n",
        // Not the alias subcommand at all (`aliases` merely queries).
        "interp aliases {}\n",
        // `proc` with no name.
        "proc\n",
    ] {
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", u32::MAX),
            RealmBinding::Command("format"),
            "a malformed binding must not disturb `format`: {src}"
        );
    }
}

#[test]
fn a_foreign_target_path_rebinds_without_aliasing() {
    let src = "interp alias {} myfmt slave format\n";
    let map = map_for(src);
    assert_eq!(
        map.resolve("myfmt", after_first_line(src)),
        RealmBinding::Rebound
    );
}

#[test]
fn a_top_level_proc_shadows_the_builtin_it_names() {
    let src = "proc format {args} { return USER }\n";
    let map = map_for(src);
    assert_eq!(
        map.resolve("format", after_first_line(src)),
        RealmBinding::Rebound
    );
    // A fresh procedure changes the table even without shadowing a builtin.
    let src = "proc mything {args} { return 1 }\n";
    let map = map_for(src);
    assert!(!map.is_empty());
    assert_eq!(map.resolve("mything", u32::MAX), RealmBinding::Rebound);
    assert_eq!(
        map.resolve("format", u32::MAX),
        RealmBinding::Command("format")
    );
    // A qualified proc defines a different command entirely.
    let src = "namespace eval ns {}; proc ::ns::format {args} { return 1 }\n";
    let map = map_for(src);
    assert_eq!(map.resolve("::ns::format", u32::MAX), RealmBinding::Rebound);
    assert_eq!(
        map.resolve("format", u32::MAX),
        RealmBinding::Command("format")
    );
}

#[test]
fn a_nested_binding_is_not_a_document_wide_fact() {
    // A conditional or deferred binding is not an unconditional statement
    // about the document's command table.
    for src in [
        "if {0} { rename format origfmt }\n",
        "proc p {} { rename format origfmt }\n",
    ] {
        assert_eq!(
            map_for(src).resolve("format", u32::MAX),
            RealmBinding::Command("format"),
            "deferred binding leaked: {src}"
        );
    }
    // Immediate eval and uplevel actually execute their literal bodies.
    for src in [
        "eval { rename format origfmt }\n",
        "uplevel #0 { rename format origfmt }\n",
    ] {
        let map = map_for(src);
        assert_eq!(
            map.resolve("format", u32::MAX),
            RealmBinding::Rebound,
            "executed rename omitted: {src}"
        );
        assert_eq!(
            map.resolve("origfmt", u32::MAX),
            RealmBinding::Command("format")
        );
    }
    // A `namespace eval` body *is* unconditional, but it binds in its own
    // namespace: `proc format` there defines `::n::format` and leaves the
    // global command table alone (#2065), so the fact is stated — scoped
    // to that namespace — and never reaches a head outside it.
    let src = "namespace eval ::n { proc format {a} { return 1 } }\nformat %b 5\n";
    let map = map_for(src);
    assert_eq!(
        map.resolve("format", u32::try_from(src.len()).unwrap_or(0)),
        RealmBinding::Command("format"),
        "a namespace-local shadow must not leak to the document"
    );
}

fn logical_namespace_map_for(source: &str) -> CommandBindingRealm {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .expect("known Logical fixture metadata")
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    document_lexical_realm_with_resolved_input(source, &input, None)
}

#[test]
fn a_namespace_local_proc_shadows_the_builtin_inside_that_namespace() {
    let source = "namespace eval n {\n    proc format {args} { return NS }\n    format %b 5\n}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let inside = u32::try_from(source.find("format %b").unwrap()).unwrap();
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", inside), RealmBinding::Rebound);
    assert_eq!(
        map.resolve("format", outside),
        RealmBinding::Command("format")
    );
    assert_eq!(
        map.resolve("::format", inside),
        RealmBinding::Command("format")
    );
}

#[test]
fn logical_namespace_local_declaration_keeps_the_global_table_independent() {
    let source = "namespace eval n {proc format {args} {return NS}}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(
        map.resolve("format", outside),
        RealmBinding::Command("format")
    );
    assert_eq!(
        map.resolve("::format", outside),
        RealmBinding::Command("format")
    );
}

#[test]
fn logical_namespace_local_call_preserves_only_unchanged_global_slots() {
    let source = "namespace eval n {proc format {args} {rename ::format ::saved_format; return NS}; format %b 5}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", outside), RealmBinding::Rebound);
    assert_eq!(map.resolve("::format", outside), RealmBinding::Rebound);
}

#[test]
fn logical_namespace_builtin_dispatch_does_not_issue_native_evidence() {
    let source = "namespace eval n {proc format {args} {return NS}; format %b 5}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let proc = u32::try_from(source.find("proc format").unwrap()).unwrap();
    let binding = map.invocation_at_source("proc", proc);
    assert!(
        binding
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
    assert!(!binding.unobserved_native_dispatch());
    assert!(binding.native_handler_envelope.is_none());
    assert_eq!(binding.compiled_candidates, []);
}

#[test]
fn logical_dispatch_refuses_native_evidence_but_fresh_authoring_keeps_it() {
    let source = "format %d 5\n";
    let logical = logical_namespace_map_for(source).invocation_at_source("format", 0);
    assert!(
        logical
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
    assert!(!logical.unobserved_native_dispatch());
    let fresh = map_for(source).invocation_at_source("format", 0);
    assert!(
        fresh
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
    assert!(fresh.unobserved_native_dispatch());
}

#[test]
fn logical_namespace_unknown_callee_keeps_global_lookup_unknown() {
    let source = "namespace eval n {proc format {args} {$unknown}; format %b 5}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let inside = u32::try_from(source.find("format %b").unwrap()).unwrap();
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", inside), RealmBinding::Rebound);
    assert_eq!(map.resolve("format", outside), RealmBinding::Rebound);
}

#[test]
fn logical_namespace_multiple_command_body_keeps_global_lookup_unknown() {
    let source =
        "namespace eval n {proc format {args} {set a 1; return NS}; format %b 5}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", outside), RealmBinding::Rebound);
}

#[test]
fn logical_namespace_wrong_arity_cannot_donate_quiet_continuation() {
    for call in ["format", "format x y"] {
        let source = format!(
            "namespace eval n {{proc format {{required}} {{return NS}}; {call}}}\nformat %d 5\n"
        );
        let map = logical_namespace_map_for(&source);
        let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
        assert_eq!(
            map.resolve("format", outside),
            RealmBinding::Rebound,
            "{call}"
        );
    }
}

#[test]
fn logical_namespace_dynamic_target_and_observers_keep_lookup_unknown() {
    for source in [
        "namespace eval $target {proc format {args} {return NS}; format %b 5}\nformat %d 5\n",
        "trace add execution namespace enter observer\nnamespace eval n {proc format {args} {return NS}; format %b 5}\nformat %d 5\n",
    ] {
        let map = logical_namespace_map_for(source);
        let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
        assert_eq!(
            map.resolve("format", outside),
            RealmBinding::Rebound,
            "{source}"
        );
    }
}

#[test]
fn logical_namespace_reference_formal_does_not_donate_quiet_continuation() {
    let source = "namespace eval n {proc format {&value} {return NS}; format name}\nformat %d 5\n";
    let map = logical_namespace_map_for(source);
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", outside), RealmBinding::Rebound);
}

#[test]
fn ordinary_native_unknown_local_call_does_not_donate_global_lookup() {
    let source = "namespace eval n {proc format {args} {$unknown}; format %b 5}\nformat %d 5\n";
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .expect("known actual software Native owner");
    let profile = environment.analyser_profile();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        crate::command_binding::SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_point(
                entry
                    .execution_point
                    .expect("captured actual execution point"),
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let map = realm_from_source_bindings(bindings, context.commands());
    let outside = u32::try_from(source.rfind("format %d").unwrap()).unwrap();
    assert_eq!(map.resolve("format", outside), RealmBinding::Rebound);
    assert_eq!(map.resolve("::format", outside), RealmBinding::Rebound);
}

/// The shadow is keyed by namespace, not by body: a later block re-opening
/// the same namespace sees it, a nested one does not.
#[test]
fn a_namespace_local_shadow_follows_the_namespace_not_the_block() {
    let src = "namespace eval a {\n    proc format {args} { return A }\n    namespace eval b { format %b 1 }\n}\nnamespace eval a { format %b 2 }\n";
    let map = map_for(src);
    let nested = u32::try_from(src.find("format %b 1").unwrap()).unwrap();
    let reopened = u32::try_from(src.find("format %b 2").unwrap()).unwrap();
    assert_eq!(
        map.resolve("format", nested),
        RealmBinding::Command("format"),
        "`::a::b` resolves its own table then the global one, never `::a`"
    );
    assert_eq!(
        map.resolve("format", reopened),
        RealmBinding::Rebound,
        "re-opening `::a` sees the proc the first block declared"
    );
}

/// An unreadable body retains uncertainty rather than a registry identity.
#[test]
fn an_unreadable_namespace_body_states_no_local_fact() {
    for src in [
        // A dynamic namespace word names a namespace we cannot identify.
        "namespace eval $ns { proc format {args} { return 1 }\nformat %b 5 }\n",
        // A non-braced body is not the script Tcl finally runs.
        "namespace eval n $body\n",
    ] {
        let map = map_for(src);
        assert_eq!(
            map.binding_at("format", u32::try_from(src.len()).unwrap()),
            RealmBindingFact::Rebound,
            "unreadable namespace body must not license registry semantics: {src}"
        );
    }
}

#[test]
fn a_qualified_mutator_head_states_the_same_fact() {
    // C Tcl resolves `::rename` to `::rename` — the mutator's own spelling
    // must not be a false negative either.
    let src = "::rename format origfmt\n";
    let map = map_for(src);
    assert_eq!(
        map.resolve("origfmt", after_first_line(src)),
        RealmBinding::Command("format")
    );
}

#[test]
fn the_latest_applicable_fact_wins() {
    let src = "rename format origfmt\norigfmt %d 7\nproc origfmt {args} { return 1 }\n";
    let map = map_for(src);
    let after_all = u32::try_from(src.len()).unwrap_or(0);
    // Between the `rename` and the `proc`, `origfmt` *is* the built-in …
    assert_eq!(
        map.resolve("origfmt", after_first_line(src)),
        RealmBinding::Command("format")
    );
    // … and after the `proc` takes the name back it is a user command,
    // even though `origfmt` is not itself a registry name.
    assert_eq!(map.resolve("origfmt", after_all), RealmBinding::Rebound);
}

/// Chained bindings compose.
///
/// On tclsh 8.6.16 and 9.0.4 (byte-identical):
///
/// ```tcl
/// interp alias {} a {} format ; rename a b ; b %08x 42   ;# 0000002a
/// info commands a                                        ;# (empty)
/// rename lindex li1 ; rename li1 li2 ; li2 {x y z} 2      ;# z
/// rename lsort mysort ; interp alias {} sorter {} mysort
/// sorter {c a b}                                          ;# a b c
/// ```
#[test]
fn chained_bindings_compose() {
    // alias → rename
    let src = "interp alias {} a {} format\nrename a b\n";
    let map = map_for(src);
    let end = u32::try_from(src.len()).unwrap_or(0);
    assert_eq!(map.resolve("b", end), RealmBinding::Command("format"));
    assert_eq!(map.resolve("::b", end), RealmBinding::Command("format"));
    // The intermediate name is gone once the rename moves it.
    assert_eq!(map.resolve("a", end), RealmBinding::Rebound);

    // rename → rename
    let src = "rename lindex li1\nrename li1 li2\n";
    let map = map_for(src);
    let end = u32::try_from(src.len()).unwrap_or(0);
    assert_eq!(map.resolve("li2", end), RealmBinding::Command("lindex"));

    // rename → alias
    let src = "rename lsort mysort\ninterp alias {} sorter {} mysort\n";
    let map = map_for(src);
    let end = u32::try_from(src.len()).unwrap_or(0);
    assert_eq!(map.resolve("sorter", end), RealmBinding::Command("lsort"));
}

/// A chain must not inherit a *broken* binding: the rename moves the user
/// proc, not the built-in it shadowed, so the new name gets no grammar.
#[test]
fn a_chain_through_a_shadowed_builtin_stays_rebound() {
    let src = "proc format {args} { return USER }\nrename format myfmt\n";
    let map = map_for(src);
    let end = u32::try_from(src.len()).unwrap_or(0);
    assert_eq!(map.resolve("myfmt", end), RealmBinding::Rebound);
    assert_eq!(map.resolve("::myfmt", end), RealmBinding::Rebound);
}

/// An alias whose target is an ordinary user proc still *takes over* the
/// name it binds — C Tcl lets an alias shadow an existing command
/// (tclsh 8.6.16 / 9.0.4: `proc myproc …; interp alias {} lindex {} myproc`
/// makes `lindex {a b c} 1` answer `MINE`).
#[test]
fn an_alias_over_a_builtin_rebinds_it() {
    let src = "proc myproc {args} { return MINE }\ninterp alias {} lindex {} myproc\n";
    let map = map_for(src);
    let end = u32::try_from(src.len()).unwrap_or(0);
    assert_eq!(map.resolve("lindex", end), RealmBinding::Rebound);
}

#[test]
fn an_unpositioned_read_abstains_when_the_facts_disagree() {
    // The renamed slot was absent before creation. An offset-free query
    // must retain that disagreement rather than choose the final state.
    let src = "rename format origfmt\n";
    let map = map_for(src);
    assert_eq!(map.resolve_unpositioned("origfmt"), RealmBinding::Rebound);
    assert_eq!(map.resolve_unpositioned("format"), RealmBinding::Rebound);
    // A head nothing binds keeps its own spelling.
    assert_eq!(
        map.resolve_unpositioned("lindex"),
        RealmBinding::Command("lindex")
    );

    // Two facts that disagree — without a position, neither can be chosen.
    let src = "rename format origfmt\nproc origfmt {args} { return 1 }\n";
    let map = map_for(src);
    assert_eq!(map.resolve_unpositioned("origfmt"), RealmBinding::Rebound);
}

#[test]
fn the_shared_empty_map_binds_nothing() {
    let map = CommandBindingRealm::none();
    assert!(map.is_empty());
    assert_eq!(map.resolve("format", 0), RealmBinding::Command("format"));
    assert_eq!(
        map.resolve_unpositioned("format"),
        RealmBinding::Command("format")
    );
}

#[test]
fn a_rebound_head_resolves_to_no_registry_spec() {
    // The `Rebound` sentinel must be unresolvable, since that is what makes
    // every registry query answer "unknown" without a variant check.
    let registry = tcl_registry::model::ingress::static_context_for("tcl").commands();
    assert!(registry.get(RealmBinding::Rebound.spec_name()).is_none());
}

#[test]
fn irules_proc_identity_requires_a_closed_declaration_body() {
    // In iRules, unlike generic Tcl, only a real file-level declaration
    // takes over a command identity.  These malformed spellings must not
    // hide the following event handler.
    for src in [
        "proc when {} bare\nwhen HTTP_REQUEST {}\n",
        "proc when {} \"quoted\"\nwhen HTTP_REQUEST {}\n",
        "proc when {} {closed} trailing\nwhen HTTP_REQUEST {}\n",
        // The unterminated body consumes the tail as Tcl recovery does;
        // it still must not leave a persistent `when` rebinding behind.
        "proc when {} {unterminated",
    ] {
        let map = irules_map_for(src);
        assert_eq!(
            map.resolve("when", u32::MAX),
            RealmBinding::Command("when"),
            "malformed iRules proc poisoned `when`: {src:?}"
        );
    }

    let valid = irules_map_for("proc when {args} {}\nwhen HTTP_REQUEST {}\n");
    assert_eq!(valid.resolve("when", u32::MAX), RealmBinding::Rebound);

    // Generic Tcl deliberately preserves its existing `proc` binding
    // behaviour; the iRules declaration gate is profile-specific.
    let generic = map_for("proc format {} bare\nformat %d 1\n");
    assert_eq!(generic.resolve("format", u32::MAX), RealmBinding::Rebound);
}

#[test]
fn unavailable_irules_mutators_do_not_change_event_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for command in ["interp", "rename", "namespace"] {
        assert!(
            registry
                .get_for_surface(command, registry.own_surface_query())
                .is_none(),
            "F5 K36322151 disables {command} in iRules"
        );
    }
    let events = tcl_registry::events::EventRegistry::build();
    let profiles = tcl_registry::profiles::ProfileRegistry::build();
    // F5 K36322151: iRules disables `interp`, `rename`, and `namespace`.
    // Their Tcl shapes are data/error commands here, never identity facts.
    let source = "interp alias {} event {} when\nrename when event\nnamespace import ::x::*\nwhen HTTP_REQUEST {}\n";
    let identities =
        document_realm_bindings(source, tcl_dialect::DialectProfile::irules(), registry);
    assert_eq!(
        identities.resolve("when", 0),
        RealmBinding::Command("when"),
        "an unavailable mutator must not change the preceding event identity"
    );
    let inferred = tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
        source,
        &events,
        &profiles,
        registry,
        &identities,
    );
    assert!(inferred.contains(&"HTTP".to_owned()));

    // `proc` is available, so a genuine, executable rebinding still
    // removes the registry event-handler grammar.
    let rebound = "proc when {args} {}\nwhen HTTP_REQUEST {}\n";
    let identities =
        document_realm_bindings(rebound, tcl_dialect::DialectProfile::irules(), registry);
    let inferred = tcl_registry::profiles::compute_file_profiles_with_registry_and_head_resolver(
        rebound,
        &events,
        &profiles,
        registry,
        &identities,
    );
    assert!(!inferred.contains(&"HTTP".to_owned()));
}
