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
use crate::cfg::{Block, Function};
use crate::ir::Statement;
use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
use std::collections::HashSet;
use tcl_lexer::Span;

fn original_factory_tokens(
    source: &str,
    version: tcl_dialect::TclVersion,
    marker: &str,
    head: &str,
) -> crate::ir::CommandTokens {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.rfind(marker).expect("original invocation")).unwrap();
    let point = bindings.invocation_at_source(head, offset);
    let mut tokens = point
        .original_recorded_command_tokens()
        .expect("original retained invocation vector");
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

fn standalone_factory_metadata(
    version: tcl_dialect::TclVersion,
) -> TypePropagationMetadata<'static> {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let context = tcl_registry::model::SemanticContext::for_environment(version.dialect_name());
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
    TypePropagationMetadata {
        registry,
        context: Some(context.into()),
        config: tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        numbers: registry.numbers(),
    }
}

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

fn assert_unsupported_caller_factories(
    version: tcl_dialect::TclVersion,
    metadata: TypePropagationMetadata<'_>,
) {
    for (source, args) in [
        (
            "oo::class create C {}; C create object",
            vec!["create", "object"],
        ),
        ("oo::class create C {}; C new value", vec!["new", "value"]),
        (
            "oo::class create C {constructor {} {return}}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {self method new args {return}}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {method observe args {}; filter observe}; C new",
            vec!["new"],
        ),
        (
            "oo::class create C {}; rename C retired; proc C args {}; C new",
            vec!["new"],
        ),
    ] {
        let tokens = original_factory_tokens(source, version, "C ", "C");
        assert!(
            !caller_safe_factory_invocation(metadata, &tokens, "C", &args),
            "{version:?}: {source}"
        );
    }
}

#[test]
fn original_caller_safe_factory_keeps_selected_layout_and_withdraws_changed_owners() {
    // naming.tcloo.original-default-manufacture-source-shape
    // docs/design/analysis/name-resolution-proofs/tcloo-original-default-manufacture-source-shape.md
    // This checks source layout and the TypeInfer precision gate only. The
    // explicit standalone source carrier supplies no actual Native frame,
    // successful allocation, Normal result or execution equivalence.
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let metadata = standalone_factory_metadata(version);
        let source = "oo::class create C {}; C new";
        let tokens = original_factory_tokens(source, version, "C new", "C");
        assert!(caller_safe_factory_invocation(
            metadata,
            &tokens,
            "C",
            &["new"]
        ));
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(!caller_safe_factory_invocation(
            metadata,
            &missing,
            "C",
            &["new"]
        ));
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "create".to_owned();
        assert!(!caller_safe_factory_invocation(
            metadata,
            &changed,
            "C",
            &["new"]
        ));
        let mut changed_config = metadata.config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                config: changed_config,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                context: None,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        // Actual availability without complete source input does not become
        // the explicitly tagged standalone carrier used above.
        let availability = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                context: Some(availability.into()),
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        let mut foreign = CommandRegistry::build_default();
        let mut changed_spec = foreign.get("set").unwrap().clone();
        changed_spec.return_type = Some(TclType::List);
        foreign.insert(changed_spec);
        assert!(!caller_safe_factory_invocation(
            TypePropagationMetadata {
                registry: &foreign,
                ..metadata
            },
            &tokens,
            "C",
            &["new"],
        ));
        assert_unsupported_caller_factories(version, metadata);
    }
}

#[test]
fn original_caller_safe_factory_does_not_promote_abstract_exports_or_custom_metaclasses() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    // The C9 public-call proof permits explicit export of Abstract new.
    // That does not issue the ordinary Native default recipe required by
    // this separate precision gate, or establish a completed allocation.
    for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
        let metadata = standalone_factory_metadata(version);
        for source in [
            "oo::abstract create C {}; C new",
            "oo::abstract create C {}; oo::objdefine C export new; C new",
            "oo::class create Meta {superclass oo::class}; Meta create C {}; C new",
        ] {
            let tokens = original_factory_tokens(source, version, "C new", "C");
            assert!(
                !caller_safe_factory_invocation(metadata, &tokens, "C", &["new"]),
                "this source lacks the ordinary default manufacture owner: {source}"
            );
        }
        let source = "oo::class create C {}; C new";
        let tokens = original_factory_tokens(source, version, "oo::class create", "oo::class");
        assert_eq!(
            empty_source_class(metadata, &tokens),
            Some("::C".to_owned())
        );
        let source = "oo::abstract create C {}; C new";
        let tokens =
            original_factory_tokens(source, version, "oo::abstract create", "oo::abstract");
        assert_eq!(
            empty_source_class(metadata, &tokens),
            Some("::C".to_owned()),
            "factory own public create is separate from the manufactured class defaults"
        );
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(empty_source_class(metadata, &missing).is_none());
    }
}

#[test]
fn original_nested_factory_precision_requires_actual_child_tokens() {
    // naming.tcloo.original-default-manufacture-source-shape
    // docs/design/analysis/name-resolution-proofs/tcloo-original-default-manufacture-source-shape.md
    let version = tcl_dialect::TclVersion::V9_1;
    let metadata = standalone_factory_metadata(version);
    let source = "oo::class create C {}; set made [C new]";
    let parent = original_factory_tokens(source, version, "set made", "set");
    let calls = crate::word_subst::checked_lifted_calls(&parent, metadata.config)
        .expect("complete original child inventory");
    assert_eq!(calls.len(), 1);
    let child = calls[0].tokens.as_ref().expect("actual child tokens");
    assert!(caller_safe_factory_invocation(
        metadata,
        child,
        "C",
        &["new"]
    ));
    let statement = Statement::AssignValue {
        span: parent.words()[0].source().span,
        name: "made".to_owned(),
        name_braced: false,
        value: "[C new]".to_owned(),
        value_needs_backsubst: false,
        tokens: Some(parent.clone()),
    };
    assert!(substitutions_have_only_caller_safe_factories(
        &statement, metadata
    ));
    let mut missing = parent;
    missing.source_binding = None;
    let mut statement = statement;
    if let Statement::AssignValue { tokens, .. } = &mut statement {
        *tokens = Some(missing);
    }
    assert!(!substitutions_have_only_caller_safe_factories(
        &statement, metadata
    ));
}

#[test]
fn original_literal_contents_keep_point_grammar_and_withdraw_stale_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl84,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input);
    let source = r"set value \U00000041";
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            module
                .top_level
                .retained_source_tokens_for_statement(statement)
        })
        .expect("actual original setter source");
    let literal = original_literal_argument_contents(context.commands(), &module, tokens, 1)
        .expect("original lexical value has complete source ownership");
    assert_eq!(literal.value(), "U00000041");
    assert_eq!(literal.contents_type().tcl_type(), Some(TclType::String));
    let mut changed = module.clone();
    changed.source_metadata_input = None;
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let mut changed = module.clone();
    changed.lexer_config.escapes = tcl_dialect::EscapeSyntax::Tcl86;
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("set value OTHER");
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let foreign = tcl_registry::CommandRegistry::build_default();
    assert!(original_literal_argument_contents(&foreign, &module, tokens, 1).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[2] = "OTHER".to_owned();
    assert!(original_literal_argument_contents(context.commands(), &module, &changed, 1).is_none());
}

#[test]
fn original_literal_contents_do_not_borrow_numeric_grammar_or_substituted_values() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    // Independently authored source numerals remain Tcl85 while command
    // availability is C9. This selects no Native parser or execution.
    let mut authored = tcl_dialect::DialectProfile::plain_tcl().clone();
    authored.grammar.numbers = NumberSyntax::Tcl85;
    let profile: &'static tcl_dialect::DialectProfile = Box::leak(Box::new(authored));
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    for (source, expected) in [
        ("set value 1_000", Some(TclType::String)),
        ("set value 2", Some(TclType::Int)),
        ("set value $external", None),
        ("set value [list x]", None),
        (r"set value \x00", None),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let tokens = module
            .top_level
            .statements
            .iter()
            .find_map(|statement| {
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .expect("whole original source vector");
        let contents = original_literal_argument_contents(context.commands(), &module, tokens, 1);
        assert_eq!(
            contents
                .as_ref()
                .and_then(|value| value.contents_type().tcl_type()),
            expected,
            "{source}"
        );
        if let Some(contents) = contents {
            assert_eq!(contents.value(), tokens.argv_texts[2]);
        }
    }
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
            include_bytes!("../../tests/data/name-consumers/classes.tcl"),
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

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").expect("native Tcl fixture"),
    )
}

/// Evaluate one statement's def typing against empty context pieces and
/// return the lattice a def named `def_name` receives (`DefTyping::PerDef`
/// defs absent from the map widen to `Overdefined`, matching the
/// propagation pass).
fn eval_def(
    stmt: &Statement,
    registry: &CommandRegistry,
    ssa: &SsaFunction,
    def_name: &str,
) -> TypeLattice {
    let uses = HashMap::new();
    let types = HashMap::new();
    let values = HashMap::new();
    let known_classes: HashSet<String> = HashSet::new();
    let ctx = WordTypingCtx {
        preparations: &[],
        math: &[],
        tokens: stmt.tokens(),
        source: crate::ssa::SsaSourceView::unpositioned(ssa),
        uses: &uses,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
        config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
        types: &types,
        values: &values,
        registry,
        known_classes: &known_classes,
        namespace: "::",
        ssa,
        numbers: numbers_of(registry),
        active_writes: &[],
        normal_results: None,
    };
    match evaluate_type_def(stmt, &ctx) {
        DefTyping::Uniform(t) => t,
        DefTyping::PerDef(map) => map
            .get(def_name)
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined),
    }
}

/// A definition the static typing knows nothing of takes the type its
/// evaluation proved — the stated intrep, else the one the route
/// constructed — and a known static type stands, element facts and all.
#[test]
fn a_folded_type_refines_only_what_the_static_typing_leaves_unknown() {
    use crate::value_transfer::FoldedType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let stated = FoldedType {
        intrep: Some(TclType::String),
        shape: None,
        representation: RepresentationEvidence::Unknown,
    };
    let built = FoldedType {
        intrep: None,
        shape: None,
        representation: RepresentationEvidence::Constructed(TclType::ByteArray),
    };
    assert_eq!(
        refined_by_folded_type(TypeLattice::overdefined(), Some(&stated)),
        TypeLattice::of(TclType::String)
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::unknown(), Some(&built)),
        TypeLattice::of(TclType::ByteArray)
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::of(TclType::List), Some(&stated)),
        TypeLattice::of(TclType::List),
        "a known static type stands"
    );
    assert_eq!(
        refined_by_folded_type(TypeLattice::overdefined(), None),
        TypeLattice::overdefined()
    );
}

fn empty_sccp(f: &Function, blocks: &[&str]) -> SccpResult {
    SccpResult {
        required_math_invocations: Vec::new(),
        required_expression_preparations: Vec::new(),
        explanations: Vec::new(),
        route_tally: crate::value_transfer::RouteTally::default(),
        folded_types: HashMap::new(),
        preserved: HashMap::new(),
        raised: HashSet::default(),
        template_plans: Vec::new(),
        selections: Vec::new(),
        existence: HashMap::new(),
        existence_reads: HashMap::new(),
        existence_exits: HashMap::new(),
        existence_entries: HashMap::new(),
        refinements: Vec::new(),
        refinements_at: HashMap::new(),
        value_entries: HashMap::new(),
        query_places: Vec::new(),
        existence_guards: Vec::new(),
        loop_enumerations: Vec::new(),
        reads_module: false,
        completion: crate::sccp::RunCompletion::default(),
        values: HashMap::new(),
        executable_blocks: blocks
            .iter()
            .map(|n| f.block_id(n).expect("interned block"))
            .collect(),
        executable_edges: HashSet::default(),
        constant_branches: Vec::new(),
    }
}

fn assign_const(name: &str, value: &str) -> Statement {
    Statement::AssignConst {
        span: Span::new(0, 0),
        name: name.to_owned(),
        value: value.to_owned(),
        name_braced: false,
        value_span: None,
    }
}

fn make_ssa_stmt(ssa: &mut SsaFunction, stmt: Statement, defs: &[(&str, u32)]) -> SsaStatement {
    SsaStatement {
        statement: stmt,
        uses: HashMap::new(),
        defs: defs.iter().map(|&(n, v)| (ssa.intern_var(n), v)).collect(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn integer_literal_infers_int() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    f.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(assign_const("x", "42"));

    let sccp = empty_sccp(&f, &["entry"]);
    let mut ssa = SsaFunction::trivial("::top", entry, f.block_names().to_vec());
    let stmt = make_ssa_stmt(&mut ssa, assign_const("x", "42"), &[("x", 1)]);
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let x = ssa.var_symbol("x").unwrap();
    let types = propagate_types(
        &f,
        &ssa,
        &sccp,
        &registry(),
        &HashSet::new(),
        &HashSet::new(),
        crate::compilation_unit::ModuleTraceFacts::none(),
    );
    assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
}

#[test]
fn incr_infers_int() {
    let stmt = Statement::Incr {
        span: Span::new(0, 0),
        name: "n".to_owned(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::of(TclType::Int));
}

#[test]
fn lassign_destructure_defs_are_overdefined_not_command_return_type() {
    // TP: `lassign $pipe a b` writes destructured list *elements*;
    // `lassign`'s own `return_type` (List — the *leftover* elements) must
    // not be broadcast onto the targets. Pre-fix, both `a` and `b` were
    // typed LIST, so a later channel-position use (`puts $a ...`) would
    // falsely fire W126 ("has type LIST, not CHANNEL") — see FP-STY-04.
    // The typing now comes from the registry's `VarWriteTyping` for
    // `lassign` (`Destructured`), not a def-count heuristic.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "lassign".to_owned(),
        canonical_command: None,
        args: vec!["$pipe".to_owned(), "a".to_owned(), "b".to_owned()],
        defs: vec!["a".to_owned(), "b".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

#[test]
fn lassign_single_destructure_def_is_overdefined_not_list() {
    // A *single*-target `lassign $l x` is the case a `defs.len() > 1`
    // heuristic misses — one write falls
    // through to `lassign`'s `List` return type, so `x` is typed LIST and
    // `expr {$x + 1}` fired a bogus S100. The registry's `Destructured`
    // typing widens it to OVERDEFINED regardless of target count.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "lassign".to_owned(),
        canonical_command: None,
        args: vec!["$point".to_owned(), "x".to_owned()],
        defs: vec!["x".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

#[test]
fn single_def_command_still_uses_its_declared_return_type() {
    // TN control: a command that writes exactly one variable (`append`)
    // legitimately shares its return value with that variable — its
    // `VarWriteTyping` is the default `ReturnValue`, so it must keep taking
    // the declared return type (`String`), not widen to OVERDEFINED.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "append".to_owned(),
        canonical_command: None,
        args: vec!["result".to_owned(), "x".to_owned()],
        defs: vec!["result".to_owned()],
        reads: Vec::new(),
        reads_own_defs: true,
        safe_on_uninit: true,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::of(TclType::String));
}

#[test]
fn unannotated_multi_def_call_stays_overdefined_not_return_type() {
    // A call that writes SEVERAL variables under the default `ReturnValue`
    // typing must not broadcast its return type onto all of them: a `try`
    // returns what its body returns, not the message and options its
    // handlers bind, and typing `msg`/`result`/`opts` alike would wrongly
    // fire S100/W126. The default arm's multi-def guard keeps them
    // OVERDEFINED.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "try".to_owned(),
        canonical_command: None,
        args: vec![
            "{set msg hello}".to_owned(),
            "on".to_owned(),
            "error".to_owned(),
            "{result opts}".to_owned(),
            "{}".to_owned(),
        ],
        defs: vec!["msg".to_owned(), "result".to_owned(), "opts".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

/// The variables `catch` writes hold the script's result and its options
/// dictionary, not the integer completion code it returns: its registry
/// entry declares them destructured, so a call that writes only the result
/// variable types it as nothing known either.
#[test]
fn a_catch_result_variable_is_overdefined_not_the_completion_code() {
    for (args, defs) in [
        (vec!["{foo}", "msg"], vec!["msg"]),
        (vec!["{foo}", "msg", "opts"], vec!["msg", "opts"]),
    ] {
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "catch".to_owned(),
            canonical_command: None,
            args: args.into_iter().map(str::to_owned).collect(),
            defs: defs.into_iter().map(str::to_owned).collect(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::overdefined(), "{stmt:?}");
    }
}

/// End-to-end lattice checks for the registry-driven `VarWriteTyping`:
/// each destructuring writer types its side-effect target correctly,
/// distinct from its return type.
#[test]
fn var_write_typing_shapes_destructure_target_types() {
    use crate::compilation_unit::CompilationUnit;

    // Helper: does any version of `name` carry a KNOWN type `t`?
    fn any_known(fu: &crate::compilation_unit::FunctionUnit, name: &str, t: TclType) -> bool {
        fu.types.iter().any(|((sym, _), lat)| {
            fu.ssa.var_name(*sym) == name
                && lat.kind() == TypeKind::Known
                && lat.tcl_type() == Some(t)
        })
    }
    // Helper: is every version of `name` non-Known (OVERDEFINED/UNKNOWN)?
    fn none_known(fu: &crate::compilation_unit::FunctionUnit, name: &str) -> bool {
        fu.types
            .iter()
            .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == name)
            .all(|(_, lat)| lat.kind() != TypeKind::Known)
    }

    // `lassign` element target — OVERDEFINED, never List.
    let cu = CompilationUnit::build_for("set p [list 1 2 3]\nlassign $p x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        none_known(fu, "x"),
        "lassign target must not carry a Known type: {:?}",
        fu.types
    );

    // `regexp` capture — OVERDEFINED, never Int (the match count).
    let cu = CompilationUnit::build_for("regexp {(.)} $s c", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(none_known(fu, "c"), "regexp capture must not be Known Int");
    // Over exact operands the route writes the capture: the
    // String it matched, still never the count.
    let cu = CompilationUnit::build_for("regexp {(.)} abc c", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "c", TclType::String) && !any_known(fu, "c", TclType::Int),
        "a written regexp capture is the String it matched: {:?}",
        fu.types
    );

    // `scan` target — OVERDEFINED (format-dependent), never Int.
    let cu = CompilationUnit::build_for("scan $s %s word", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(none_known(fu, "word"), "scan target must not be Known Int");
    // Over exact operands the route writes the conversion: the
    // String `%s` built, still never the count.
    let cu = CompilationUnit::build_for("scan hello %s word", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "word", TclType::String) && !any_known(fu, "word", TclType::Int),
        "a written scan target is the String its conversion built: {:?}",
        fu.types
    );

    // `binary scan` target (subcommand-level typing) — OVERDEFINED.
    let cu = CompilationUnit::build_for("binary scan $d a3 chars", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        none_known(fu, "chars"),
        "binary scan target must not be Known Int"
    );

    // `gets chan line` — Fixed(String): the target is the read line, a
    // String, not the character count the two-arg form returns.
    let cu = CompilationUnit::build_for("set ch stdin; gets $ch ::line", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "::line", TclType::String),
        "gets target must be Known String: {:?}; escaping {:?}; operations {:?}",
        fu.types,
        crate::var_observability::analyse_var_observability(&fu.cfg, &registry())
            .escaping_var_names(),
        fu.ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .map(|statement| (
                statement
                    .defs
                    .keys()
                    .map(|symbol| fu.ssa.var_name(*symbol))
                    .collect::<Vec<_>>(),
                &statement.may_defs,
                crate::registry_invocation::resolved_statement_invocation(
                    &registry(),
                    None,
                    &statement.statement,
                )
                .map(|invocation| invocation.facts.operation),
            ))
            .collect::<Vec<_>>()
    );

    // `lpop listVar` — Fixed(List): the variable is left holding the
    // shortened list, not the popped element (String) it returns.
    let cu = CompilationUnit::build_for("set l [list 1 2 3]\nlpop l", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "l", TclType::List),
        "lpop target must be Known List, not the element return type: {:?}",
        fu.types
    );
}

#[test]
fn float_literal_infers_double() {
    let t = literal_type("3.14", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::Double));
}

#[test]
fn exponent_and_infinite_literals_infer_double_2500() {
    for text in ["1e3", "Inf", "-Inf", "+inf"] {
        assert_eq!(
            literal_type(text, NumberSyntax::Tcl90),
            TypeLattice::of(TclType::Double),
            "{text}"
        );
    }
}

#[test]
fn bool_literal_infers_boolean() {
    let t = literal_type("true", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::Boolean));
    let t2 = literal_type("false", NumberSyntax::Tcl90);
    assert_eq!(t2, TypeLattice::of(TclType::Boolean));
}

#[test]
fn string_literal_infers_string() {
    let t = literal_type("hello", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::String));
}

#[test]
fn hex_and_binary_literals_infer_int() {
    // Hex / binary literals store an INT intrep (`set n 0x80; incr n` is
    // one clean parse, not per-iteration shimmer).
    for lit in ["0x80", "0X1f", "-0xFF", "0b1010", "0B1", "+0x0"] {
        assert_eq!(
            literal_type(lit, NumberSyntax::Tcl90),
            TypeLattice::of(TclType::Int),
            "{lit} should be INT"
        );
    }
    // Octal `0o…` and non-integer hex stay STRING (set-statement classifier).
    assert_eq!(
        literal_type("0o17", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
    assert_eq!(
        literal_type("0xZZ", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
}

/// The literal classifiers read numerals under the *target release's*
/// grammar, and the set/expr divergence on `0o…` survives it.
///
/// The matrix (from the C release sources): `0x` in every release; `0o`/`0b`
/// from 8.5; `0d` and `_` separators from 9.0; a bare leading zero is octal
/// up to 8.6 and decimal from 9.0. A spelling the release does not have is
/// not a numeral — a bareword in expr context (`Numeric`, since an `expr`
/// still yields a number) and a plain `String` after `set`.
#[test]
fn literal_classifiers_follow_the_release_numeral_matrix() {
    use NumberSyntax::{Tcl84, Tcl85, Tcl90};
    let set_type = |lit: &str, n| literal_type(lit, n).tcl_type();
    let expr_type = |lit: &str, n| expr_literal_type(lit, n).tcl_type();

    // `0o17`: the deliberate divergence. From 8.5 it is an `Int` in expr
    // context but stays `String` after `set` (its canonical intrep, `15`,
    // differs from the source text). Under 8.4 there is no `0o` prefix at
    // all, so it is a bareword either way.
    for syntax in [Tcl85, Tcl90] {
        assert_eq!(
            set_type("0o17", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(expr_type("0o17", syntax), Some(TclType::Int), "{syntax:?}");
    }
    assert_eq!(set_type("0o17", Tcl84), Some(TclType::String));
    assert_eq!(expr_type("0o17", Tcl84), Some(TclType::Numeric));

    // `0d99` — the 9.0-only decimal prefix.
    assert_eq!(set_type("0d99", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("0d99", Tcl90), Some(TclType::Int));
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(
            set_type("0d99", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(
            expr_type("0d99", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }

    // `1_000` — 9.0-only digit separators.
    assert_eq!(set_type("1_000", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("1_000", Tcl90), Some(TclType::Int));
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(
            set_type("1_000", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(
            expr_type("1_000", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }

    // `0b101` — 8.5 onward.
    assert_eq!(set_type("0b101", Tcl84), Some(TclType::String));
    assert_eq!(expr_type("0b101", Tcl84), Some(TclType::Numeric));
    for syntax in [Tcl85, Tcl90] {
        assert_eq!(set_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
        assert_eq!(expr_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
    }

    // A radix-invalid digit is never a numeral, in any release.
    for syntax in [Tcl84, Tcl85, Tcl90] {
        for bad in ["0o8", "0xZZ"] {
            assert_eq!(
                set_type(bad, syntax),
                Some(TclType::String),
                "{bad} under {syntax:?}"
            );
            assert_eq!(
                expr_type(bad, syntax),
                Some(TclType::Numeric),
                "{bad} under {syntax:?}"
            );
        }
    }

    // A bare leading zero is an integer in every release (only its *value*
    // differs: 493 up to 8.6, 755 from 9.0 — see `intervals`), while `08`
    // is a broken octal up to 8.6 and plain decimal 8 from 9.0.
    for syntax in [Tcl84, Tcl85, Tcl90] {
        assert_eq!(set_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
        assert_eq!(expr_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
    }
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(set_type("08", syntax), Some(TclType::String), "{syntax:?}");
        assert_eq!(
            expr_type("08", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }
    assert_eq!(set_type("08", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("08", Tcl90), Some(TclType::Int));
}

#[test]
fn phi_joins_types_from_executable_preds() {
    // A minimal two-block CFG: entry → exit.
    // The phi in exit merges version 1 (INT) from entry.
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(crate::cfg::Terminator::Goto {
        target: exit,
        span: None,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let phi = Phi {
        name: x,
        version: 2,
        incoming: [(entry, 1u32)].into_iter().collect(),
    };
    let entry_stmt = make_ssa_stmt(&mut ssa, assign_const("x", "10"), &[("x", 1)]);
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![entry_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    ssa.blocks.insert(
        exit,
        SsaBlock {
            name: "exit".into(),
            phis: vec![phi],
            statements: Vec::new(),
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let mut sccp = empty_sccp(&cfg, &["entry", "exit"]);
    sccp.executable_edges.insert((entry, exit));

    let types = propagate_types(
        &cfg,
        &ssa,
        &sccp,
        &registry(),
        &HashSet::new(),
        &HashSet::new(),
        crate::compilation_unit::ModuleTraceFacts::none(),
    );
    // x@1 (entry) should be Int.
    assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
    // x@2 (phi in exit) should propagate Int from entry.
    assert_eq!(types.get(&(x, 2)), Some(&TypeLattice::of(TclType::Int)));
}

/// `AssignValue` with a pure variable reference inherits the source type.
#[test]
fn assign_value_pure_var_ref_inherits_type() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for("set x 42\nset y $x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    // x should be Int; y (which copies x) should also be Int.
    let x_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int));
    let y_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "y" && t.tcl_type() == Some(TclType::Int));
    assert!(x_is_int, "expected x to be Int");
    assert!(y_is_int, "expected y to inherit Int type from x");
}

/// An aliased `set` (`interp alias {} myset {} set`) keeps its runtime
/// `Call` shape, but its single def takes the *value word's* intrep — the
/// canonical-command value-passthrough. `myset x 5` types x as Int, exactly
/// as `set x 5` would, rather than an opaque OVERDEFINED `Call`.
#[test]
fn aliased_set_call_types_def_from_value() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "interp alias {} myset {} set\nproc ::g {} { myset x 5\n return $x }",
        &registry(),
        false,
    );
    let fu = cu.function("::g").unwrap();
    let x_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int));
    assert!(
        x_is_int,
        "aliased `myset x 5` should type x as Int (value passthrough): {:?}",
        fu.types
            .iter()
            .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
            .collect::<Vec<_>>()
    );
}

/// `AssignValue` with a command substitution uses the command's return type.
#[test]
fn assign_value_command_sub_uses_return_type() {
    use crate::compilation_unit::CompilationUnit;
    // `llength` returns Int per the registry.
    let cu =
        CompilationUnit::build_for("set lst {a b c}\nset n [llength $lst]", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let n_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "n" && t.tcl_type() == Some(TclType::Int));
    assert!(n_is_int, "expected n to be Int (llength return type)");
}

#[test]
fn normal_result_representation_does_not_require_an_opcode() {
    use crate::compilation_unit::CompilationUnit;
    let registry =
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules());
    let source = "when HTTP_REQUEST {set input {a b}; set size [llength $input]}";
    let cu = CompilationUnit::build_for_profile(
        source,
        &registry,
        false,
        tcl_dialect::DialectProfile::irules(),
    );
    let function = cu.function("::when::HTTP_REQUEST").expect("handler");
    assert!(
        function.types.iter().any(|((symbol, _), value)| {
            function.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
        }),
        "successful producer: {:?}",
        function.types
    );
    let shadow = CompilationUnit::build_for_profile(
        "proc llength {args} {return arbitrary}; when HTTP_REQUEST {set input {a b}; set size [llength $input]}",
        &registry,
        false,
        tcl_dialect::DialectProfile::irules(),
    );
    let shadow = shadow.function("::when::HTTP_REQUEST").expect("handler");
    assert!(
        !shadow.types.iter().any(|((symbol, _), value)| {
            shadow.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
        }),
        "custom producer: {:?}",
        shadow.types
    );
}

/// A `dict set VAR k [Class new]` collection retrieved by `dict get` types
/// the element as the class (the collection-of-objects shape).
#[test]
fn dict_of_objects_retrieval_types_element() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   dict set pins a [Pin new]\n\
                   set p [dict get $pins a]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    let pins_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "pins" && t.element_class() == Some("::Pin")
    });
    assert!(
        pins_ok,
        "pins should be Dict<OBJECT(::Pin)>; got {:?}",
        fu.types
            .iter()
            .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
            .collect::<Vec<_>>()
    );
    let p_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "p"
            && t.tcl_type() == Some(TclType::Object)
            && t.class_name() == Some("::Pin")
    });
    assert!(p_ok, "p (dict get) should be OBJECT(::Pin)");
}

/// A `lappend VAR [Class new]` list retrieved by `lindex` types the element.
#[test]
fn list_of_objects_lindex_types_element() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create Pin {}\n\
                   lappend pins [Pin new]\n\
                   set p [lindex $pins 0]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    let p_ok = fu.types.iter().any(|((name, _), t)| {
        fu.ssa.var_name(*name) == "p"
            && t.tcl_type() == Some(TclType::Object)
            && t.class_name() == Some("::Pin")
    });
    assert!(p_ok, "p (lindex) should be OBJECT(::Pin)");
}

/// A collection written with two *different* object classes is not
/// homogeneous, so the element class widens away (retrieval stays untyped).
#[test]
fn heterogeneous_object_collection_drops_element_class() {
    use crate::compilation_unit::CompilationUnit;
    let src = "oo::class create A {}\noo::class create B {}\n\
                   dict set d k1 [A new]\n\
                   dict set d k2 [B new]\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").expect("top level");
    // The latest `d` version must not claim a single element class.
    let widened = fu
        .types
        .iter()
        .filter(|((name, _), _)| fu.ssa.var_name(*name) == "d")
        .all(|((_, ver), t)| *ver < 2 || t.element_class().is_none());
    assert!(
        widened,
        "mixed-class dict must drop its element class; types {:?}; operations {:?}",
        fu.types,
        fu.ssa
            .blocks
            .iter()
            .map(|(id, block)| (
                id,
                block
                    .statements
                    .iter()
                    .enumerate()
                    .map(|(index, statement)| (
                        &statement.defs,
                        &statement.uses,
                        crate::ssa::SsaSourceView::at_statement(&fu.ssa, *id, index).symbol("d"),
                    ))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
}

// expr type-inference precision

/// Infer the type of a standalone expression string (no SSA
/// context — variable refs stay `Unknown`), under the Tcl 9.0 numeral
/// grammar.
fn infer_str(src: &str) -> TypeLattice {
    infer_str_dialect(src, None)
}

/// As [`infer_str`] but parses under `dialect` (the iRules string
/// predicates only tokenise as operators in the iRules dialect) and reads
/// numerals under that dialect's grammar.
fn infer_str_dialect(
    src: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> TypeLattice {
    let node = crate::parse_expr_for_profile(src, dialect);
    infer_expr_type(
        &node,
        &HashMap::new(),
        0,
        crate::intervals::numbers_for_dialect(dialect),
    )
}

#[test]
fn decoded_expression_names_keep_literal_sigils_and_element_identity() {
    let profile = tcl_registry::model::ingress::static_context_for("tcl8.6")
        .commands()
        .profile();
    let types = HashMap::from([
        ("b".to_owned(), TypeLattice::of(TclType::List)),
        ("$b".to_owned(), TypeLattice::of(TclType::Int)),
        ("a".to_owned(), TypeLattice::of(TclType::Dict)),
        ("a(k)".to_owned(), TypeLattice::of(TclType::Double)),
    ]);
    for (source, expected) in [("${$b}", TclType::Int), ("${a(k)}", TclType::Double)] {
        let expression = crate::parse_expr_for_profile(source, profile);
        let reference = expression
            .variable_reference(tcl_lexer::LexerConfig::from_grammar(
                profile.unwrap().grammar,
            ))
            .unwrap()
            .unwrap();
        assert!(reference.index.is_none());
        let ExprNode::Var { start, end, .. } = &expression else {
            panic!("variable");
        };
        let occurrence_types = HashMap::from([(
            (source.to_owned(), *start, *end),
            types[std::str::from_utf8(reference.name).unwrap()].clone(),
        )]);
        assert_eq!(
            infer_expr_type(
                &expression,
                &occurrence_types,
                0,
                NumberSyntax::of_profile(profile)
            )
            .tcl_type(),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn expression_contents_types_use_original_element_and_literal_name_reads() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        (
            "proc f {} {set {$b} [expr {7<<1}]; set b [list x]; return [expr {${$b}}]}; f",
            TclType::Int,
        ),
        (
            "proc f {} {set a(k) [expr {7.5}]; return [expr {${a(k)}}]}; f",
            TclType::Double,
        ),
        (
            "proc f {} {set k k; set a(k) [expr {7.5}]; return [expr {$a($k)}]}; f",
            TclType::Double,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        assert_eq!(
            unit.function("::f").unwrap().return_type.tcl_type(),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn written_container_reads_keep_raw_and_braced_variable_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for word in ["$items", "${items}"] {
        let source = format!(
            "proc f {{raw}} {{set items [list [expr {{$raw<<1}}]]; return [lindex {word} 0]}}; f 7"
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
        assert_eq!(
            unit.function("::f").unwrap().return_type.tcl_type(),
            Some(TclType::Int),
            "{source}"
        );
    }
}

#[test]
fn math_function_calls_infer_their_return_type() {
    // (a) sqrt → Double, int → Int, bool → Boolean.
    assert_eq!(infer_str("sqrt(2.0)").tcl_type(), Some(TclType::Double));
    assert_eq!(infer_str("sin($x)").tcl_type(), Some(TclType::Double));
    assert_eq!(infer_str("int($x)").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("wide($x)").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("isnan($x)").tcl_type(), Some(TclType::Boolean));
    // abs is identity: abs(2) keeps the operand's Int type.
    assert_eq!(infer_str("abs(2)").tcl_type(), Some(TclType::Int));
    // max/min join operands: max(1, 2) stays Int.
    assert_eq!(infer_str("max(1, 2)").tcl_type(), Some(TclType::Int));
    // Tcl 9.1 C99 functions (TIP 745): double-valued, except `signbit`.
    for f in [
        "acosh", "cbrt", "exp2", "log2", "trunc", "erf", "expm1", "logb",
    ] {
        assert_eq!(
            infer_str(&format!("{f}($x)")).tcl_type(),
            Some(TclType::Double),
            "{f} should infer Double",
        );
    }
    assert_eq!(infer_str("signbit($x)").tcl_type(), Some(TclType::Boolean));
    // The rest of TIP 521's classification family answers 0 or 1 like
    // `isnan` (tclsh 9.0 and 9.1: `isfinite(1.0)` is 1, `isunordered(1,
    // 2)` is 0); the table this reads classes them as `isnan` is.
    for f in ["isfinite", "isnormal", "issubnormal"] {
        assert_eq!(
            infer_str(&format!("{f}($x)")).tcl_type(),
            Some(TclType::Boolean),
            "{f} should infer Boolean",
        );
    }
    assert_eq!(
        infer_str("isunordered($x, 1)").tcl_type(),
        Some(TclType::Boolean)
    );
    // Unknown function → Numeric (conservative).
    assert_eq!(infer_str("nope($x)").tcl_type(), Some(TclType::Numeric));
}

/// `infer_expr_type` (and its
/// mutually-recursive helper `expr_call_type`) recurse once per
/// `ExprNode` operator-tree level, so both need a depth cap.
/// A long left-associative `1+1+1+…` chain parses *iteratively* (the
/// Pratt parser's own `MAX_EXPR_DEPTH` never trips — it builds the
/// left-nested `Binary` spine in a loop), so it hands this walker an
/// arbitrarily deep tree the parser cap does not bound. Empirically that
/// unguarded walk overflowed the native stack (SIGABRT) in the low
/// thousands of levels on a 2 MiB thread (`cargo test`'s per-test
/// default). 3000 is comfortably past both that crash range and
/// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that inference returns
/// at all, not what it returns. Also exercises `expr_call_type` via the
/// nested-`min(…)` variant.
#[test]
fn deeply_nested_expr_type_inference_survives() {
    let chain = format!("1{}", "+1".repeat(3000));
    // Returns without overflowing the stack; the value is unspecified
    // past the cap (a conservative `overdefined`/`numeric`).
    let _ = infer_str(&chain);

    // Drive the `expr_call_type` side of the same recursion cluster with
    // a tree built directly — nested `min(min(…))` *source* would trip
    // the Pratt parser's own `MAX_EXPR_DEPTH` first (function-call
    // nesting recurses the parser, unlike the iterative binary chain
    // above), so build the deep `Call` spine by hand to reach this
    // walker unbounded.
    let mut node = ExprNode::Literal {
        text: "1".to_owned(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Call {
            function: "abs".to_owned(),
            args: vec![node],
            start: 0,
            end: 1,
        };
    }
    let _ = infer_expr_type(&node, &HashMap::new(), 0, NumberSyntax::Tcl90);
}

/// Companion to `deeply_nested_expr_type_inference_survives`: moderate
/// nesting (well under `MAX_EXPR_NODE_DEPTH`) must still infer exactly as
/// before — the cap changes nothing for realistic input. A 100-deep
/// all-integer additive chain is unambiguously `Int`.
#[test]
fn moderate_depth_expr_type_inference_unchanged() {
    let chain = format!("1{}", "+1".repeat(100));
    assert_eq!(infer_str(&chain).tcl_type(), Some(TclType::Int));
}

#[test]
fn bitwise_and_shift_ops_infer_int() {
    // (c) bitwise / shift force Int even with untyped operands.
    for src in ["$x & $y", "$x | $y", "$x ^ $y", "$x << 2", "$x >> 2"] {
        assert_eq!(
            infer_str(src).tcl_type(),
            Some(TclType::Int),
            "expected Int for `{src}`",
        );
    }
}

#[test]
fn irules_string_predicates_infer_boolean() {
    // (b) the iRules string predicates were falling through to
    // overdefined; they are Boolean.
    for src in [
        "$s contains \"x\"",
        "$s starts_with \"x\"",
        "$s ends_with \"x\"",
        "$s equals \"x\"",
        "$s matches_glob \"x*\"",
        "$s matches_regex \"x.\"",
    ] {
        let t = infer_str_dialect(src, Some(tcl_dialect::DialectProfile::irules()));
        assert_eq!(
            t.tcl_type(),
            Some(TclType::Boolean),
            "expected Boolean for `{src}`, got {t:?}",
        );
    }
}

#[test]
fn arithmetic_promotes_double() {
    // `_arithmetic_result`: int + double → Double (was Numeric).
    assert_eq!(infer_str("3 + 2.0").tcl_type(), Some(TclType::Double));
    // int + int → Int.
    assert_eq!(infer_str("3 + 4").tcl_type(), Some(TclType::Int));
}

#[test]
fn expr_context_literal_typing() {
    // Every integer spelling tokenises to Int in expr context — including
    // octal `0o…`, which the set-statement classifier keeps as String.
    assert_eq!(infer_str("0o17").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("0xff").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("0b1010").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("42").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("3.14").tcl_type(), Some(TclType::Double));
    // The set-statement classifier still keeps octal as String.
    assert_eq!(
        literal_type("0o17", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
    // An unrecognised literal degrades to Numeric in expr context
    // (the set-statement classifier would say String).
    assert_eq!(
        expr_literal_type("nope", NumberSyntax::Tcl90).tcl_type(),
        Some(TclType::Numeric)
    );
}

#[test]
fn bitnot_coerces_to_int() {
    // `~$x` always yields Int, regardless of the operand's type
    // (was leaking the operand type via the identity arm).
    assert_eq!(infer_str("~$x").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("~3.5").tcl_type(), Some(TclType::Int));
}

#[test]
fn scope_alias_commands_detected() {
    let reg = registry();
    let one = |s: &str| vec![s.to_owned()];
    assert!(is_scope_alias_call(&reg, "global", &one("g")));
    assert!(is_scope_alias_call(&reg, "variable", &one("v")));
    assert!(is_scope_alias_call(
        &reg,
        "upvar",
        &["0".into(), "x".into(), "y".into()]
    ));
    assert!(is_scope_alias_call(
        &reg,
        "namespace",
        &["upvar".into(), "ns".into(), "x".into(), "y".into()]
    ));
    assert!(is_scope_alias_call(
        &reg,
        "my",
        &["variable".into(), "count".into()]
    ));
    // A plain command (and `namespace eval`) is not a scope alias.
    assert!(!is_scope_alias_call(&reg, "set", &["x".into(), "1".into()]));
    assert!(!is_scope_alias_call(
        &reg,
        "namespace",
        &["eval".into(), "ns".into(), "body".into()]
    ));
    // `my`'s other subcommands (an arbitrary method name) are not scope
    // aliases — only the reserved `variable` word is.
    assert!(!is_scope_alias_call(&reg, "my", &["touch".into()]));
}

#[test]
fn scope_alias_declaration_preserves_unknown_contents() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc ::f {} { variable counter; set observed $counter; return $observed }",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    assert!(
        fu.types
            .iter()
            .all(|((name, _), _)| fu.ssa.var_name(*name) != "::counter"),
        "a declaration alone cannot write namespace contents"
    );
    assert_eq!(
        fu.return_type.kind(),
        TypeKind::Overdefined,
        "an external cell's unknown contents are not the declaration's return value; actual return {}",
        fu.return_type
    );
}

#[test]
fn actual_traced_alias_store_preserves_only_a_closed_unchanged_value() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (callback, preserves_dictionary) in [
        ("", true),
        // A callback can coerce the same value without changing its bytes
        // or dictionary contents. Current representation is a separate
        // read receipt consumed by representation diagnostics.
        ("upvar 1 dictionary destination; llength $destination", true),
        (
            "upvar 1 dictionary destination; set destination altered",
            false,
        ),
    ] {
        let source = format!(
            "proc observe args {{{callback}}}; proc f {{}} {{set dictionary [dict create k 1]; upvar 0 dictionary view; trace add variable view write observe; set dictionary [dict create k 2]; llength $view}}"
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::f").unwrap();
        let mut writes = Vec::new();
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if matches!(&statement.statement, Statement::AssignValue { name, .. } if name == "dictionary")
                {
                    for (&symbol, &version) in &statement.defs {
                        writes.push((
                            statement.statement.span().start(),
                            function.types.get(&(symbol, version)),
                            crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index)
                                .normal_store_contents_preserved(symbol, registry),
                        ));
                    }
                }
            }
        }
        writes.sort_by_key(|(offset, _, _)| *offset);
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[1].2, preserves_dictionary, "callback {callback:?}");
        assert_eq!(
            writes[1].1.unwrap().tcl_type() == Some(TclType::Dict),
            preserves_dictionary,
            "callback {callback:?}: {:?}",
            writes[1].1,
        );
    }
}

#[test]
fn expression_types_follow_actual_alias_spellings_at_each_read() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} {set original [expr {3.5}]; upvar 0 original view; set result [expr {$original}]; return [expr {$view}]}",
        registry,
        false,
    );
    let function = unit.function("::f").unwrap();
    assert_eq!(function.return_type.tcl_type(), Some(TclType::Double));
    let writes = function
        .ssa
        .blocks
        .values()
        .flat_map(|body| &body.statements);
    let result = writes
            .filter(|statement| matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result"))
            .flat_map(|statement| &statement.defs)
            .filter_map(|(&symbol, &version)| function.types.get(&(symbol, version)))
            .collect::<Vec<_>>();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].tcl_type(), Some(TclType::Double));
}

#[test]
fn conditionally_assigned_param_merges_to_overdefined() {
    use crate::compilation_unit::CompilationUnit;
    // A parameter assigned in only one arm must NOT be
    // typed from that arm alone. The merge phi joins the live-in
    // (caller-supplied, unknown → OVERDEFINED) with the assigned-arm Int,
    // so the merged type is OVERDEFINED — never Known Int.
    let cu = CompilationUnit::build_for(
        "proc ::p {c x} { if {$c} { set x 5 }\n return $x }",
        &registry(),
        false,
    );
    let fu = cu.function("::p").unwrap();
    // (TP) The merge phi for `x` is OVERDEFINED.
    let has_overdefined = fu
        .types
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "x" && t.kind() == TypeKind::Overdefined);
    assert!(
        has_overdefined,
        "conditionally-assigned param 'x' should merge to OVERDEFINED: {:?}",
        fu.types
    );
    // (FP guard) The assigned arm (`set x 5`) is *still* typed Known Int —
    // the fix widens only the merge, not the definite assignment. Exactly
    // one `x` version is Known Int (the assigned arm) and one is
    // OVERDEFINED (the phi); pre-fix, the phi would also be Known Int.
    let known_int_count = fu
        .types
        .iter()
        .filter(|((n, _), t)| {
            fu.ssa.var_name(*n) == "x" && matches!(t.tcl_type(), Some(TclType::Int))
        })
        .count();
    assert_eq!(
        known_int_count, 1,
        "only the assigned arm of 'x' should be Known Int (not the phi): {:?}",
        fu.types
    );
}

#[test]
fn unconditionally_assigned_param_still_typed() {
    use crate::compilation_unit::CompilationUnit;
    // (FP guard): an *unconditionally* reassigned local is
    // still typed from its definition — the version-0 join only applies
    // when a live-in genuinely reaches the merge. Here `y` is set on every
    // path, so its post-assignment type stays Known Int.
    let cu = CompilationUnit::build_for(
        "proc ::q {c} { set y 5\n if {$c} { set y 7 } else { set y 9 }\n return $y }",
        &registry(),
        false,
    );
    let fu = cu.function("::q").unwrap();
    let all_int = fu
        .types
        .iter()
        .filter(|((n, _), _)| fu.ssa.var_name(*n) == "y")
        .all(|(_, t)| matches!(t.tcl_type(), Some(TclType::Int)) || t.kind() == TypeKind::Unknown);
    assert!(
        all_int,
        "unconditionally-assigned 'y' should stay Int on every path: {:?}",
        fu.types
    );
}

// registry-driven container element inference (type-tracking.md).

/// Helper: the joined lattice of every version of `var` in `func`.
fn type_of(cu: &crate::compilation_unit::CompilationUnit, func: &str, var: &str) -> TypeLattice {
    let fu = cu.function(func).unwrap();
    let mut symbols = HashSet::new();
    symbols.extend(fu.ssa.var_symbol(var));
    for (&block, cfg_block) in &fu.cfg.blocks {
        for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
            symbols
                .extend(crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var));
        }
    }
    let mut acc = TypeLattice::unknown();
    for ((sym, ver), t) in fu.types.iter() {
        if *ver > 0 && symbols.contains(sym) {
            acc = type_join(&acc, t);
        }
    }
    acc
}

/// Elements are shared objects — a committed computed element
/// keeps its intrep inside the list, and `lindex` retrieves it.
#[test]
fn list_builder_tracks_committed_element_and_lindex_retrieves_it() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set l [list [expr {2**20}] other]\nset first [lindex $l 0]",
        &registry(),
        false,
    );
    let l = type_of(&cu, "::top", "l");
    assert_eq!(l.tcl_type(), Some(TclType::List), "{l}");
    let elements = l.elements().expect("tracked elements").clone();
    assert_eq!(elements.known_len(), Some(2), "{l}");
    assert!(
        matches!(elements.shape_at(0), Some(s) if s.is_numeric_family()),
        "committed element 0 keeps its numeric intrep: {l}"
    );
    assert_eq!(
        elements.shape_at(1),
        None,
        "a literal word stays agnostic: {l}"
    );
    let first = type_of(&cu, "::top", "first");
    assert!(
        matches!(first.single_shape(), Some(s) if s.is_numeric_family()),
        "lindex retrieves the element's shape: {first}"
    );
}

/// Arity is a fact even with unknown element values: `[list $a $b]`
/// provably has two elements.
#[test]
fn list_builder_tracks_arity_of_unknown_words() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "proc f {a b} { set l [list $a $b]\n return $l }",
        &registry(),
        false,
    );
    let l = type_of(&cu, "::f", "l");
    assert_eq!(
        l.elements().and_then(Elements::known_len),
        Some(2),
        "arity of [list $a $b] is exactly 2: {l}"
    );
}

/// The old object-collection behaviour, now through general elements:
/// `lappend`ing constructed objects yields `List<OBJECT(C)*>` and a
/// `lassign` / `lindex` retrieval resolves the class.
#[test]
fn lassign_of_committed_object_list_types_targets() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nlassign $l a b",
        &registry(),
        false,
    );
    let a = type_of(&cu, "::top", "a");
    assert_eq!(a.tcl_type(), Some(TclType::Object), "{a}");
    assert_eq!(a.class_name(), Some("::Foo"), "{a}");
    let b = type_of(&cu, "::top", "b");
    assert_eq!(b.class_name(), Some("::Foo"), "{b}");
}

/// FP-SH-17 parity: `lassign` targets from a *pure-literal* list stay
/// Overdefined — no shape claim is defensible for value-dependent
/// conversion checks.
#[test]
fn lassign_of_literal_list_targets_stay_overdefined() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set point [list 1 2 3]\nlassign $point x y z",
        &registry(),
        false,
    );
    let x = type_of(&cu, "::top", "x");
    assert_eq!(x.kind(), TypeKind::Overdefined, "{x}");
}

/// A `lassign` target past the container's arity receives the empty
/// string the command pads with.
#[test]
fn lassign_past_arity_is_empty_string() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "set l [list [expr {1+1}]]\nlassign $l a b",
        &registry(),
        false,
    );
    let b = type_of(&cu, "::top", "b");
    assert_eq!(b.tcl_type(), Some(TclType::String), "{b}");
}

/// `dict create` of constructed objects → `Dict<OBJECT(C)*>`; `dict get`
/// resolves the element class (the old object retrieval, generalised).
#[test]
fn dict_create_and_get_track_object_values() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Pin {}\nset pins [dict create p1 [Pin new] p2 [Pin new]]\nset p [dict get $pins p1]",
        &registry(),
        false,
    );
    let pins = type_of(&cu, "::top", "pins");
    assert_eq!(pins.element_class(), Some("::Pin"), "{pins}");
    let p = type_of(&cu, "::top", "p");
    assert_eq!(p.class_name(), Some("::Pin"), "{p}");
}

/// `lappend var [C new]` evolves the variable's elements — the old
/// `collection_element_class` behaviour through the registry's
/// `VarElementsEffect`, aliased spellings included.
#[test]
fn lappend_object_evolves_element_class_through_rename() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset acc [list]\nlappend acc [Foo new]\nlappend acc [Foo new]",
        &registry(),
        false,
    );
    let acc = type_of(&cu, "::top", "acc");
    assert_eq!(acc.tcl_type(), Some(TclType::List), "{acc}");
    assert_eq!(acc.element_class(), Some("::Foo"), "{acc}");
}

/// A `foreach` element var over a tracked homogeneous list takes the
/// element shape at its def (the loop-header phi separately joins the
/// live-in as Overdefined, by the conditionally-assigned rule).
#[test]
fn foreach_var_takes_uniform_element_shape() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nforeach o $l { puts $o }",
        &registry(),
        false,
    );
    let fu = cu.function("::top").unwrap();
    let has_object_def = fu.types.iter().any(|((sym, ver), t)| {
        *ver > 0 && fu.ssa.var_name(*sym) == "o" && t.class_name() == Some("::Foo")
    });
    assert!(
        has_object_def,
        "some version of 'o' must carry the element class: {:?}",
        fu.types
    );
}

/// constant-keyed array elements are independent variables — each
/// carries its own type ("array elements behave as independent
/// scalars"), and the conflated base claims nothing.
#[test]
fn array_elements_type_independently() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} {\n    set arr(a) 5\n    set arr(b) \"hello world\"\n    return $arr(a)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let type_of = |name: &str| -> Vec<Option<TclType>> {
        fu.types
            .iter()
            .filter(|((sym, ver), _)| *ver > 0 && fu.ssa.var_name(*sym) == name)
            .map(|(_, t)| t.tcl_type())
            .collect()
    };
    assert!(
        type_of("arr(a)").iter().all(|t| *t == Some(TclType::Int)),
        "arr(a) is INT in every version: {:?}",
        fu.types
    );
    assert!(
        type_of("arr(b)")
            .iter()
            .all(|t| *t == Some(TclType::String)),
        "arr(b) is STRING independently: {:?}",
        fu.types
    );
    assert!(
        type_of("arr").iter().all(Option::is_none),
        "the base symbol claims no value type: {:?}",
        fu.types
    );
}

/// a dynamic-key write is a may-write over every known element —
/// the element's type JOINS with the written type (INT ⊔ INT stays
/// INT; INT ⊔ STRING widens) instead of trusting either side.
#[test]
fn dynamic_key_write_joins_element_types() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {k} {\n    set a(x) 5\n    set a($k) 9\n    return $a(x)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let final_is_int = fu.return_type.tcl_type() == Some(TclType::Int);
    assert!(
        final_is_int,
        "INT ⊔ INT across the may-write stays INT: {:?}",
        fu.types
    );

    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc g {k} {\n    set a(x) 5\n    set a($k) \"s t\"\n    return $a(x)\n}",
        &tcl_registry::CommandRegistry::build_default(),
        false,
    );
    let fu = cu.function("::g").unwrap();
    let widened = fu.return_type.tcl_type() == Some(TclType::Int);
    assert!(
        !widened,
        "INT ⊔ STRING must not stay a single INT claim: {:?}",
        fu.types
    );
}

/// Dict-value semantics, verified on tclsh 8.6/9.0:
/// a multi-key `dict set` stores a *dict* under the first key; `dict
/// lappend` stores a *list*; `dict append` concatenates (intreps do
/// not survive), with only object dispatch-identity flowing.
#[test]
fn dict_value_effects_match_oracle() {
    for profile in ["tcl8.6", "tcl9.0"] {
        assert_dict_value_effects(
            tcl_dialect::DialectProfile::find(profile).expect("native Tcl fixture"),
        );
    }
}

fn assert_dict_value_effects(profile: &'static tcl_dialect::DialectProfile) {
    let reg = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let elements_of = |src: &str, qname: &str, var: &str| -> Vec<Option<TclType>> {
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
        let fu = cu.function(qname).unwrap();
        let mut symbols = HashSet::new();
        symbols.extend(fu.ssa.var_symbol(var));
        for (&block, cfg_block) in &fu.cfg.blocks {
            for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
                symbols.extend(
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var),
                );
            }
        }
        fu.types
            .iter()
            .filter(|((sym, ver), _)| *ver > 0 && symbols.contains(sym))
            .map(|(_, t)| {
                t.elements()
                    .and_then(Elements::uniform_shape)
                    .map(|s| s.coarse())
            })
            .collect()
    };
    // Multi-key set: the top-level value is a DICT, never the leaf shape.
    let multi = elements_of(
        "proc f {} {\n    set d {}\n    dict set d outer inner [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        multi.iter().flatten().all(|t| *t == TclType::Dict),
        "multi-key dict set stores a nested dict: {multi:?}"
    );
    // Single-key set keeps the leaf's shape.
    let single = elements_of(
        "proc f {} {\n    set d {}\n    dict set d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        single
            .iter()
            .any(|t| matches!(t, Some(TclType::Int | TclType::Numeric))),
        "single-key dict set keeps the leaf shape: {single:?}"
    );
    // lappend: the value is a LIST (elements unknown — the prior scalar
    // becomes element 0).
    let lap = elements_of(
        "proc f {} {\n    set d {}\n    dict lappend d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        lap.iter().flatten().all(|t| *t == TclType::List),
        "dict lappend listifies the value: {lap:?}"
    );
    // append: concatenation — a numeric argument's intrep must NOT
    // become the value's element fact.
    let app = elements_of(
        "proc f {} {\n    set d {}\n    dict append d k [expr {1}]\n    return $d\n}",
        "::f",
        "d",
    );
    assert!(
        !app.iter()
            .flatten()
            .any(|t| matches!(t, TclType::Int | TclType::Numeric)),
        "dict append must not claim a numeric value intrep: {app:?}"
    );
}

/// Bignum literals classify on the tower: a beyond-wide decimal is
/// `Bignum` (coarse `Int`), never wrapped and never `String`.
#[test]
fn bignum_literal_classifies_on_the_tower() {
    assert_eq!(
        literal_type("18446744073709551616", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Bignum)
    );
    assert_eq!(
        literal_type("0xFFFFFFFFFFFFFFFF", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Bignum)
    );
    assert_eq!(
        literal_type("5", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Int)
    );
    assert_eq!(
        literal_type("-9223372036854775808", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Int),
        "i64::MIN still fits a wide"
    );
}
