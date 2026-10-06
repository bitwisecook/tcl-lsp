// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Semantic equality excludes only derived query caches.

use super::{Arc, CommandAllocationSite, SourceCommandBindings, SourceInvocationBinding};

#[test]
fn cfg_replay_retains_actual_entry_without_later_world_mutations() {
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    let assistance = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let actual = tcl_registry::model::ingress::static_context_for("tcl8.4");
    let dialect = tcl_registry::InvocationDialect::of_profile(
        actual.commands().profile().expect("actual C8.4 fixture"),
    );
    for mode in [
        NativeCompilationMode::Direct,
        NativeCompilationMode::BytecodeObject,
    ] {
        for unknown_entry in [false, true] {
            let entry = super::SourceAnalysisEntry {
                invocation_realm: tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                invocation_dialect: Some(dialect),
                unknown_entry,
                native_compilation: NativeCompilationContext {
                    mode,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..Default::default()
            };
            let cu = CompilationUnit::build_with_source_entry(
                "proc introduced {} {}; list TOKEN",
                UnitBuildOptions {
                    registry: assistance.commands(),
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::default(),
                    dialect: assistance.commands().profile(),
                    external_call_sites: None,
                    declared_commands: None,
                },
                &entry,
            );
            let cfg = &cu.function("::top").expect("top-level CFG").cfg;
            let seed = super::cfg_entry_state(cfg, assistance.commands());
            assert_eq!(seed.baseline.dialect, entry.invocation_dialect);
            assert_eq!(seed.baseline.invocation_realm, entry.invocation_realm);
            assert_eq!(seed.baseline.native_compilation, entry.native_compilation);
            assert_eq!(seed.baseline.unknown_entry, unknown_entry);
            assert_eq!(seed.opaque_binding_mutation, unknown_entry);
            assert!(!seed.bindings.contains_key("::introduced"));
        }
    }
}

#[test]
fn cfg_replay_declines_conflicting_or_missing_retained_entry_contexts() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc introduced {} {}; list TOKEN",
        &registry,
        false,
    );
    for missing in [false, true] {
        let mut cfg = cu.function("::top").expect("top-level CFG").cfg.clone();
        let mut bindings = cfg
            .blocks
            .values_mut()
            .flat_map(|block| &mut block.statements)
            .filter_map(crate::ir::Statement::tokens_mut)
            .filter_map(|tokens| tokens.source_binding.as_mut());
        let _ = bindings.next().expect("first retained context");
        let changed = bindings.next().expect("second retained context");
        if missing {
            changed.lookup_state = None;
        } else {
            let snapshot = Arc::make_mut(changed.lookup_state.as_mut().unwrap());
            let baseline = Arc::make_mut(&mut snapshot.state.baseline);
            baseline.native_compilation.loop_depth += 1;
            baseline.refresh_fingerprint();
            snapshot.fingerprint = std::sync::OnceLock::new();
        }
        assert!(super::cfg_entry_state(&cfg, &registry).opaque_binding_mutation);
    }
}

#[test]
fn source_proof_equality_ignores_query_caches_but_retains_authority() {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = owner
        .commands()
        .profile()
        .expect("tcl8.6 fixture has an explicit profile");
    let analyse = || {
        SourceCommandBindings::analyse_with_options(
            "set selected YES",
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            owner.commands(),
            super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..Default::default()
            },
        )
    };
    let original = analyse();
    let queried = analyse();
    assert_eq!(original, queried);
    for point in &queried.points {
        let _ = point.lookup_binding("set");
    }
    queried
        .unpositioned_projections
        .0
        .lock()
        .unwrap()
        .insert("set".to_owned(), SourceInvocationBinding::default());
    assert_eq!(original, queried);

    let mut changed = original.clone();
    Arc::make_mut(&mut changed.final_state).mark_opaque_binding_mutation();
    assert_ne!(original, changed);

    let mut changed = original.clone();
    changed.normal_variable_continuations.insert(
        CommandAllocationSite {
            source: Arc::clone(original.root_origin.as_ref().unwrap()),
            offset: 0,
        },
        None,
    );
    assert_ne!(original, changed);

    let mut changed = original.clone();
    changed.lexer_config = None;
    assert_ne!(original, changed);
}

#[test]
fn native_entry_grammar_selects_original_reads_before_assistance() {
    use super::SourceAnalysisOptions;
    for (actual, assistance, source, expected) in [
        (
            "jim",
            "tcl8.6",
            "set café UNICODE; set caf ASCII; list $café",
            "$café",
        ),
        (
            "tcl8.6",
            "jim",
            "set café UNICODE; set caf ASCII; list $café",
            "$caf",
        ),
        (
            "tcl8.4",
            "tcl9.0",
            r#"set "a{b" OLD; set {a{b}c} NEW; list ${a{b}c}"#,
            "${a{b}",
        ),
        (
            "tcl9.0",
            "tcl8.4",
            r#"set "a{b" OLD; set {a{b}c} NEW; list ${a{b}c}"#,
            "${a{b}c}",
        ),
    ] {
        let owner = tcl_registry::model::ingress::static_context_for(assistance);
        let native = tcl_registry::model::ingress::static_context_for(actual);
        let profile = native.commands().profile().expect("actual fixture profile");
        let mut config = tcl_lexer::LexerConfig::for_profile(owner.commands().profile());
        config.strict_quoting = true;
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            owner.commands(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let site = u32::try_from(source.rfind("list").unwrap()).unwrap();
        let reads = bindings.variable_accesses_for_invocation_args(site);
        assert_eq!(
            reads
                .iter()
                .map(|read| read.original_spelling.as_str())
                .collect::<Vec<_>>(),
            [expected],
            "actual={actual}, assistance={assistance}, source={source}",
        );
        assert_eq!(
            bindings.lexer_config,
            Some(config.with_grammar(profile.grammar)),
            "the selected grammar retains caller lexical modes",
        );
    }
}

#[test]
fn unavailable_native_grammar_retains_supplied_lexical_configuration() {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let supplied = tcl_registry::model::ingress::static_context_for("jim");
    let config = tcl_lexer::LexerConfig {
        strict_quoting: true,
        base_offset: 73,
        base_line: 11,
        base_col: 5,
        ..tcl_lexer::LexerConfig::for_profile(supplied.commands().profile())
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        "set selected YES",
        config,
        owner.commands(),
        super::SourceAnalysisOptions {
            invocation_dialect: None,
            unknown_entry: true,
            ..Default::default()
        },
    );
    assert_eq!(bindings.lexer_config, Some(config));
}

fn lookup_snapshot_hash(snapshot: &super::SourceLookupSnapshot) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    snapshot.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn source_lookup_snapshot_memo_is_lazy_and_preserves_clone_semantics() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let original =
        super::SourceLookupSnapshot::new(super::ModuleCommandBindings::initial(&registry));
    assert!(original.fingerprint.get().is_none());
    let untouched = original.clone();
    assert!(untouched.fingerprint.get().is_none());
    let hash = lookup_snapshot_hash(&original);
    assert!(original.fingerprint.get().is_some());
    assert!(untouched.fingerprint.get().is_none());
    let cached = original.clone();
    assert_eq!(cached.fingerprint.get(), original.fingerprint.get());
    let independent =
        super::SourceLookupSnapshot::new(super::ModuleCommandBindings::initial(&registry));
    for equivalent in [untouched, cached, independent] {
        assert_eq!(original, equivalent);
        assert_eq!(hash, lookup_snapshot_hash(&equivalent));
    }
}

#[test]
fn source_lookup_snapshot_memo_keeps_realm_and_full_state_identity() {
    use tcl_dialect::model::InvocationRealm;
    let registry = tcl_registry::CommandRegistry::build_default();
    let state = super::ModuleCommandBindings::initial(&registry);
    let mut original = super::SourceLookupSnapshot::new(state.clone());
    let mut changed_state = state.clone();
    changed_state.mark_opaque_binding_mutation();
    let mut changed = super::SourceLookupSnapshot::new(changed_state);
    // Deliberate hash collisions do not supply semantic identity or close an
    // unknown world. Realm remains an independent part of the retained key.
    original.fingerprint = std::sync::OnceLock::from(7);
    changed.fingerprint = std::sync::OnceLock::from(7);
    assert_ne!(original, changed);
    let mut foreign =
        super::SourceLookupSnapshot::in_realm(state, InvocationRealm::InterpreterRuntime);
    foreign.fingerprint = std::sync::OnceLock::from(7);
    assert_ne!(original, foreign);
}

#[test]
fn source_lookup_snapshot_join_invalidates_only_the_detached_point() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let state = super::ModuleCommandBindings::initial(&registry);
    let original = super::source_binding(&state, "set", "::");
    let original_snapshot = original.lookup_state.as_ref().unwrap();
    let original_hash = lookup_snapshot_hash(original_snapshot);
    let mut changed = original.clone();
    let mut incoming = state;
    incoming.mark_opaque_binding_mutation();
    changed.join(&super::source_binding(&incoming, "set", "::"));
    let joined = changed.lookup_state.as_ref().unwrap();
    assert!(!Arc::ptr_eq(original_snapshot, joined));
    assert!(joined.fingerprint.get().is_none());
    assert!(original_snapshot.fingerprint.get().is_some());
    assert_eq!(original_hash, lookup_snapshot_hash(original_snapshot));
    let fresh = super::SourceLookupSnapshot::in_realm(joined.state.clone(), joined.realm);
    assert_eq!(joined.as_ref(), &fresh);
    assert_eq!(lookup_snapshot_hash(joined), lookup_snapshot_hash(&fresh));
    assert_ne!(original_snapshot, joined);
}
