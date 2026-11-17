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

//! Typing under retained source metadata: selected owners, availability, and facts withdrawn for foreign or changed sources.

use super::*;

#[test]
fn original_generic_precision_uses_current_builtin_alias_and_rename_selection() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // This is the branch-level source metadata/precision gate. It proves
    // no Normal completion, handler effects or whole-function precision.
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    for (source, marker, head) in [
        ("info patchlevel", "info patchlevel", "info"),
        (
            "rename info inspect; inspect patchlevel",
            "inspect patchlevel",
            "inspect",
        ),
        (
            "interp alias {} inspect {} info; inspect patchlevel",
            "inspect patchlevel",
            "inspect",
        ),
    ] {
        let tokens = original_factory_tokens(source, version, marker, head);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_target()
                .is_some()
        );
        assert!(
            caller_safe_factory_invocation(metadata, &tokens, head, &["patchlevel"]),
            "{source}"
        );
        assert!(
            caller_safe_factory_invocation(
                metadata,
                &tokens,
                "unrelated_reported_head",
                &["unrelated_reported_argument"],
            ),
            "metadata comes from the authentic selected original invocation"
        );
    }
}

#[test]
fn original_generic_precision_does_not_repair_unknown_or_absent_lookup_from_metadata() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // The whole-function gate separately refuses the preceding mutation.
    // This checks this helper's actual lookup barrier, without claiming an
    // observed wrong whole-function result in the previous source state.
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    for source in [
        "set auto_index(unknown_original_mutation) {source lib.tcl}; unknown_original_mutation; info patchlevel",
        "rename info {}; info patchlevel",
    ] {
        let tokens = original_factory_tokens(source, version, "info patchlevel", "info");
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.unknown || binding.may_be_absent, "{source}");
        assert!(binding.proved_target().is_none());
        assert!(
            binding
                .original_invocation_metadata_at_point(&tokens, metadata.registry)
                .is_ok(),
            "original metadata retains unknown lookup independently"
        );
        assert!(
            !caller_safe_factory_invocation(metadata, &tokens, "info", &["patchlevel"]),
            "{source}"
        );
    }
}

#[test]
fn original_generic_precision_withdraws_foreign_and_changed_source_owners() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    let tokens = original_factory_tokens("info patchlevel", version, "info patchlevel", "info");
    assert!(caller_safe_factory_invocation(
        metadata,
        &tokens,
        "info",
        &["patchlevel"]
    ));
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(!caller_safe_factory_invocation(
        metadata,
        &missing,
        "info",
        &["patchlevel"]
    ));
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(!caller_safe_factory_invocation(
        metadata,
        &changed,
        "info",
        &["patchlevel"]
    ));
    let mut changed = tokens.clone();
    let foreign = original_factory_tokens("info tclversion", version, "info tclversion", "info");
    changed.source_binding = foreign.source_binding;
    assert!(!caller_safe_factory_invocation(
        metadata,
        &changed,
        "info",
        &["patchlevel"]
    ));
    let mut config = metadata.config;
    config.strict_quoting = !config.strict_quoting;
    assert!(!caller_safe_factory_invocation(
        TypePropagationMetadata { config, ..metadata },
        &tokens,
        "info",
        &["patchlevel"],
    ));
    assert!(!caller_safe_factory_invocation(
        TypePropagationMetadata {
            context: None,
            ..metadata
        },
        &tokens,
        "info",
        &["patchlevel"],
    ));
    let mut foreign = CommandRegistry::build_default();
    let mut changed_spec = foreign.get("info").unwrap().clone();
    changed_spec.return_type = Some(TclType::String);
    foreign.insert(changed_spec);
    assert!(!caller_safe_factory_invocation(
        TypePropagationMetadata {
            registry: &foreign,
            ..metadata
        },
        &tokens,
        "info",
        &["patchlevel"],
    ));
}

#[test]
fn original_generic_precision_retains_the_independent_declared_transition_predicate() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // Authenticated selection does not alter the separate predicate. This
    // control records its current declared-transition refusal, not a claim
    // that this ordinary list invocation mutates Tcl's command names.
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    let tokens = original_factory_tokens("list ONE", version, "list ONE", "list");
    let selected =
        crate::registry_invocation::original_structured_invocation_with_metadata_context(
            metadata.registry,
            metadata.context.unwrap(),
            &tokens,
        )
        .expect("authentic original selected list metadata");
    assert!(selected.facts.state_transitions.is_declared());
    assert!(!caller_safe_factory_invocation(
        metadata,
        &tokens,
        "list",
        &["ONE"]
    ));
}

#[test]
fn original_type_metadata_retains_actual_availability_and_independent_source_grammar() {
    // naming.compiler.retained-representation-metadata
    // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;

    let base =
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut commands = base
        .commands()
        .project_for_profile(base.commands().profile().unwrap());
    commands.insert_ambient_package("example", "1.0");
    let actual = Arc::new(base.with_command_store(Arc::new(commands)));
    let registry = actual.commands();
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        list_parse: tcl_dialect::ListParse::Lenient,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let unit = CompilationUnit::build_with_context_registry(
        "proc f {lst} {return [llength $lst]}",
        UnitBuildOptions {
            registry,
            dialect: Some(profile),
            defer_top_level: false,
            config,
            declared_commands: None,
            external_call_sites: None,
        },
        None,
        Arc::clone(&actual),
    );
    let function = unit.function("::f").unwrap();
    let metadata = TypePropagationMetadata::for_function(function, registry).unwrap();
    assert!(core::ptr::eq(
        metadata.context.unwrap().context(),
        actual.context()
    ));
    assert!(
        metadata
            .context
            .unwrap()
            .context()
            .ambient_package("example")
    );
    assert_eq!(metadata.config, config);
    assert_eq!(metadata.numbers, profile.grammar.numbers);
    assert_ne!(metadata.numbers, registry.numbers());
    assert!(
        tcl_syntax::word_rules::WordValueRules::from_config(&metadata.config)
            .split_list("{a")
            .is_ok()
    );
    assert!(
        tcl_syntax::word_rules::WordValueRules::TCL
            .split_list("{a")
            .is_err()
    );
    assert_eq!(
        tcl_lexer::word_parts::whole_var_ref(b"${a{b}", metadata.config)
            .unwrap()
            .unwrap()
            .name,
        b"a{b",
    );
    assert!(
        tcl_lexer::word_parts::whole_var_ref(
            b"${a{b}",
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
        .is_err()
    );
    let mut stale = function.clone();
    stale.source_config.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
    assert!(TypePropagationMetadata::for_function(&stale, registry).is_none());
}

#[test]
fn original_type_consumers_keep_positive_normal_results_and_refuse_missing_foreign_metadata() {
    // naming.compiler.retained-representation-metadata
    // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
    use crate::compilation_unit::CompilationUnit;

    let registry = registry();
    let unit = CompilationUnit::build_for(
        "proc f {lst} {set x [llength $lst]; return $x}",
        &registry,
        false,
    );
    let function = unit.function("::f").unwrap();
    let classes: HashSet<String> = HashSet::new();
    let escaping: HashSet<String> = HashSet::new();
    let metadata = TypePropagationMetadata::for_function(function, &registry).unwrap();
    let infer = |metadata| {
        infer_function_return_type(
            &function.cfg,
            &function.sccp,
            &function.types,
            metadata,
            &classes,
            &function.ssa,
        )
    };
    let propagate = |metadata| {
        propagate_types_with_metadata_context(
            &function.cfg,
            &function.ssa,
            &function.sccp,
            metadata,
            &classes,
            &escaping,
            crate::compilation_unit::ModuleTraceFacts::none(),
        )
    };
    assert_eq!(infer(metadata).tcl_type(), Some(TclType::Int));
    assert!(!propagate(metadata).is_empty());
    let missing = TypePropagationMetadata {
        context: None,
        ..metadata
    };
    assert_eq!(infer(missing).kind(), TypeKind::Unknown);
    assert!(propagate(missing).is_empty());
    let mut foreign = tcl_registry::CommandRegistry::build_default();
    let mut changed = foreign.get("llength").unwrap().clone();
    changed.return_type = Some(TclType::String);
    foreign.insert(changed);
    let foreign_metadata = TypePropagationMetadata {
        registry: &foreign,
        ..metadata
    };
    assert_eq!(infer(foreign_metadata).kind(), TypeKind::Unknown);
    assert!(propagate(foreign_metadata).is_empty());
    assert!(TypePropagationMetadata::for_function(function, &foreign).is_none());
    let mut absent = function.clone();
    absent.source_metadata_input = None;
    assert!(TypePropagationMetadata::for_function(&absent, &registry).is_none());
    let replacement = CompilationUnit::build_for(
        "proc llength {lst} {return other}; proc f {lst} {set x [llength $lst]; return $x}",
        &registry,
        false,
    );
    let replaced = replacement.function("::f").unwrap();
    let replacement_metadata = TypePropagationMetadata::for_function(replaced, &registry).unwrap();
    let replacement_result = infer_function_return_type(
        &replaced.cfg,
        &replaced.sccp,
        &replaced.types,
        replacement_metadata,
        &classes,
        &replaced.ssa,
    );
    assert_ne!(replacement_result.tcl_type(), Some(TclType::Int));
}

#[test]
fn positional_type_facts_retain_native_cell_keys_without_display_lookup() {
    use crate::command_binding::SourceNamespaceKey;
    use crate::var_resolve::VariableProofRelocation;
    use tcl_runtime_api::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};

    let registry = registry();
    let interpreter = NativeInterpreterIdentity {
        owner: NativeInterpreterIdentity::fresh_owner(),
        interpreter: 0,
    };
    let key = |token| VariableCellKey::Namespace {
        identity: SourceNamespaceKey::Native(NativeNamespaceContext {
            interpreter,
            token,
            path: tcl_core_types::ByteNamespacePath::from_segments(["N"]),
        }),
        simple: "x".into(),
    };
    let original = key(1);
    let replacement = key(2);
    let mut ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    ssa.intern_var("x");
    ssa.relocate_variable_proofs(&VariableProofRelocation {
        storage_keys: HashMap::from([("x".into(), original.clone())]),
        ..Default::default()
    });
    let uses = HashMap::new();
    let types = HashMap::new();
    let values = HashMap::new();
    let known_classes: HashSet<String> = HashSet::new();
    let context = WordTypingCtx {
        preparations: &[],
        math: &[],
        tokens: None,
        source: crate::ssa::SsaSourceView::unpositioned(&ssa),
        uses: &uses,
        defs: None,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
        config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
        types: &types,
        values: &values,
        registry: &registry,
        known_classes: &known_classes,
        namespace: "::",
        ssa: &ssa,
        numbers: numbers_of(&registry),
        active_writes: &[],
        normal_results: None,
    };
    let DefTyping::PerDef(facts) =
        elements_of_def_typing(&context, &["item"], &["x".into()], Some(&[1]), 0)
    else {
        panic!("positional definition types");
    };
    assert_eq!(
        facts.get(&original),
        Some(&TypeLattice::of(TclType::String))
    );
    assert_eq!(facts.get(&replacement), None);
    assert_eq!(facts.get(&original.compatibility_name()), None);
    assert_eq!(facts.get("x"), None);
}

#[test]
fn constructor_types_follow_current_namespace_and_written_name_grammar() {
    let registry = registry();
    let classes = HashSet::from(["::C".to_owned(), "::ns::C".to_owned(), "::ns::".to_owned()]);
    for (command, namespace, expected) in [
        ("C", "::ns", "::ns::C"),
        ("::C", "::ns", "::C"),
        ("::ns:::C", "::", "::ns::C"),
        ("::ns:::", "::", "::ns::"),
    ] {
        assert_eq!(
            constructor_object_type(&registry, command, &["new"], &classes, namespace),
            TypeLattice::object_of(expected),
        );
    }
    assert_eq!(
        constructor_object_type(&registry, "unknown", &["new"], &classes, "::ns"),
        TypeLattice::overdefined(),
    );
    assert_eq!(
        constructor_object_type(&registry, "C", &["unrecognised"], &classes, "::ns"),
        TypeLattice::overdefined(),
    );
    for version in tcl_dialect::TclVersion::ALL {
        if version < tcl_dialect::TclVersion::V8_6 {
            continue;
        }
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let result = tcl_test_support::run_script(
            &tclsh.path,
            include_bytes!("../../../tests/data/name-consumers/classes.tcl"),
        )
        .expect("run the C Tcl constructor oracle")
        .strict_text()
        .expect("constructor oracle succeeds");
        assert_eq!(
            result, "::ns::C\n::C\n::ns::C\n::ns::",
            "{}",
            tclsh.patchlevel
        );
    }
}
