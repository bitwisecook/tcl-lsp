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

#[test]
fn source_free_effects_keep_point_contexts_without_lexical_restoration() {
    use crate::ir::{CommandTokens, Statement, SyntheticMarker, WordEffectsHost};
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::subject", "entry");
    let effect = Statement::word_effects(
        tcl_lexer::Span::new(0, 1),
        (vec!["changed".to_owned()], vec!["read".to_owned()]),
        WordEffectsHost::Statement,
    );
    cfg.blocks
        .get_mut(&cfg.entry)
        .unwrap()
        .statements
        .push(effect.clone());
    assert!(cfg.source_input_tokens_at(cfg.entry, 0).is_none());
    let mut contexts = build_point_resolve_contexts(&cfg, "::subject", &registry);
    assert!(contexts.statements.contains_key(&(cfg.entry, 0)));
    assert!(contexts.after.contains_key(&(cfg.entry, 0)));
    assert!(contexts.restore_source_tokens(&BTreeMap::new()));
    assert_eq!(cfg.blocks[&cfg.entry].statements, [effect]);

    for marker in [
        SyntheticMarker::EvaluatedArguments,
        SyntheticMarker::WordEffects(WordEffectsHost::Statement),
    ] {
        let tokens = cfg.blocks.get_mut(&cfg.entry).unwrap().statements[0]
            .tokens_mut()
            .unwrap();
        *tokens = CommandTokens::marker(marker);
        if matches!(marker, SyntheticMarker::WordEffects(_)) {
            tokens.argv_texts.push("partial carrier".to_owned());
        }
        assert!(cfg.source_input_tokens_at(cfg.entry, 0).is_some());
        let mut contexts = build_point_resolve_contexts(&cfg, "::subject", &registry);
        assert!(!contexts.restore_source_tokens(&BTreeMap::new()));
    }
}

#[test]
fn original_logical_store_point_refuses_changed_words_context_and_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let unit = CompilationUnit::build_with_analysis_input(
        "interp alias {} store {} set; proc f {} {store p literal.txt; open $p}",
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let function = unit.function("::f").unwrap();
    let original = function.ssa.point_contexts.as_ref().unwrap();
    let (&(block, index), _) = original
        .logical_scalar_stores
        .iter()
        .next()
        .expect("the genuine source alias write owns a conditional point");
    assert_eq!(
        original
            .conditional_logical_scalar_store_at(block, index, context.commands())
            .and_then(ConditionalLogicalScalarStore::value),
        Some("literal.txt")
    );
    for axis in 0..5 {
        let mut changed = original.clone();
        match axis {
            0 => {
                changed
                    .statements
                    .get_mut(&(block, index))
                    .unwrap()
                    .dynamic_bindings = true;
            }
            1 => changed
                .after
                .get_mut(&(block, index))
                .unwrap()
                .define_literal("p", "CHANGED", context.commands()),
            2 => changed
                .source_tokens
                .get_mut(&(block, index))
                .unwrap()
                .word_exprs
                .swap(1, 2),
            3 => {
                changed.metadata_context =
                    crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
            }
            _ => {
                changed
                    .source_tokens
                    .get_mut(&(block, index))
                    .unwrap()
                    .source_binding = None;
            }
        }
        assert!(
            changed
                .conditional_logical_scalar_store_at(block, index, context.commands())
                .is_none(),
            "changed original point axis {axis}"
        );
    }
}

fn original_trace_inputs(
    profile: &str,
    prefix: &[u8],
    label: &[u8],
) -> (ResolveContext, OriginalVariableInvocation) {
    original_trace_subject_inputs(profile, b"v", prefix, label)
}

fn original_trace_subject_inputs(
    profile: &str,
    subject: &[u8],
    prefix: &[u8],
    label: &[u8],
) -> (ResolveContext, OriginalVariableInvocation) {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
    );
    let policy = dialect.authored_name_policy().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let mut source = label.to_vec();
    source.extend_from_slice(b"\ntrace add variable {");
    source.extend_from_slice(subject);
    source.extend_from_slice(b"} read {");
    source.extend_from_slice(prefix);
    source.push(b'}');
    let image = tcl_lexer::SourceImage::native(source);
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
        config,
    )
    .unwrap();
    let inputs = plan
        .commands
        .last()
        .unwrap()
        .words
        .iter()
        .skip(1)
        .map(|word| {
            Some(SignatureSourceNameInput::OriginalWord(
                SignatureSourceNameKey::from_original_native_word(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    policy,
                )
                .unwrap(),
            ))
        })
        .collect();
    let mut state = ResolveContext::for_function("::p");
    state.invocation_dialect = Some(dialect);
    state.execution_name_policy = Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
        policy,
    ));
    let root = crate::command_binding::SourceNamespaceKey::authored("::");
    state.retain_namespace_world(root.clone(), [root], Some(policy.recipe()));
    (
        state,
        OriginalVariableInvocation::from_original_inputs(inputs, Vec::new()),
    )
}

fn selected_variable_trace(profile: &str, remove: bool) -> TraceTransition {
    let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
    );
    let facts = registry
        .resolve_invocation(
            "trace",
            &[
                if remove { "remove" } else { "add" },
                "variable",
                "MISLEADING_TARGET",
                "read",
                "MISLEADING_PREFIX",
            ],
            dialect.authoring_query(),
        )
        .unwrap()
        .facts();
    let StateTransition::Trace(trace) =
        &facts.state_transitions.declared().unwrap().facts()[0].transition
    else {
        panic!("selected variable trace");
    };
    trace.clone()
}

// Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
// docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
#[test]
fn original_variable_trace_copies_producers_and_removes_by_selected_native_purpose() {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let (mut state, mut operands) =
            original_trace_inputs(profile, b"watch\0tail extra", b"# first");
        let producer = crate::command_binding::original_name_value::OriginalProducedNameValue::from_source_input(
                operands.input(4, &state).unwrap(), &state,
            ).unwrap();
        operands.inputs[4] = Some(SignatureSourceNameInput::OriginalValue(
            SignatureSourceNameValue::from_original_produced_value(&producer),
        ));
        apply_trace_with_original(
            &mut state,
            &selected_variable_trace(profile, false),
            registry,
            Some(&operands),
        );
        let receiver = operands.access(
            2,
            &state,
            registry,
            tcl_registry::TraceOperation::Read,
            false,
        );
        let key = trace_key(&receiver);
        let registration = &state.trace_registrations.get(&key).unwrap()[0];
        let crate::var_resolve::VariableTracePrefix::Original(prefix) = &registration.1 else {
            panic!("original copied data");
        };
        assert_eq!(prefix.input().bytes(), b"watch\0tail extra");
        assert!(prefix.input().original_word_key().is_none());
        let stored = prefix.clone();
        state.invalidate_original_contents();
        assert!(!producer.is_current(&state));
        assert!(stored.input().is_current(&state));
        assert!(operands.input(4, &state).is_none());
        let (_, short) = original_trace_inputs(profile, b"watch", b"# removal short");
        apply_trace_with_original(
            &mut state,
            &selected_variable_trace(profile, true),
            registry,
            Some(&short),
        );
        assert_eq!(state.trace_registrations.get(&key).unwrap().len(), 1);
        let (_, other_tail) =
            original_trace_inputs(profile, b"watch\0fail extra", b"# removal equal count");
        apply_trace_with_original(
            &mut state,
            &selected_variable_trace(profile, true),
            registry,
            Some(&other_tail),
        );
        assert_eq!(state.trace_registrations.get(&key).unwrap().len(), 0);
        assert!(!state.traced.contains(&key));
        assert!(stored.input().is_current(&state));
        let mut missing = state.clone();
        missing.execution_name_policy = None;
        assert!(!stored.input().is_current(&missing));
    }
}

#[test]
// Native proof: naming.variable.trace-subject-counted-zero-address
// docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
fn original_trace_subject_keeps_cstring_purpose_separate_from_counted_runtime_lookup() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let (mut state, operands) = original_trace_subject_inputs(
            profile,
            b"v\0tail(k)",
            b"watch\0tail extra",
            b"# subject purpose",
        );
        let subject = operands.trace_subject_access(2, &state, registry);
        let ordinary = operands.access(
            2,
            &state,
            registry,
            tcl_registry::TraceOperation::Read,
            false,
        );
        assert_eq!(subject.cell.as_ref().unwrap().name.as_bytes(), b"v");
        assert!(subject.index.is_none());
        if matches!(profile, "tcl9.0" | "tcl9.1") {
            // The combined object parser cannot see a suffix beyond the
            // raw zero; its retained root remains the whole object.
            assert_eq!(
                ordinary.cell.as_ref().unwrap().name.as_bytes(),
                b"v\0tail(k)"
            );
            assert!(ordinary.index.is_none());
            assert_ne!(trace_key(&ordinary), trace_key(&subject));
        }
        apply_trace_with_original(
            &mut state,
            &selected_variable_trace(profile, false),
            registry,
            Some(&operands),
        );
        let key = trace_key(&subject);
        let registrations = state.trace_registrations.get(&key).unwrap();
        assert_eq!(registrations.len(), 1);
        let crate::var_resolve::VariableTracePrefix::Original(prefix) = &registrations[0].1 else {
            panic!("original copied prefix");
        };
        assert_eq!(prefix.input().bytes(), b"watch\0tail extra");
        assert_eq!(operands.input(2, &state).unwrap().bytes(), b"v\0tail(k)");
    }
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let (state, operands) = original_trace_subject_inputs("jim", b"v", b"watch", b"# unavailable");
    assert_eq!(
        operands.trace_subject_access(2, &state, registry).kind,
        PlaceKind::Unknown
    );
}

#[test]
fn original_trace_subject_roles_do_not_read_or_overwrite_value_contents() {
    // Implementation contract: naming.variable.trace-source-receiver-purpose (docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md).
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let (mut state, _) = original_trace_inputs(profile, b"watch", b"# value effects");
        let dialect = state.invocation_dialect.unwrap();
        let policy = dialect.authored_name_policy().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let value = resolve_literal_access(
            "v",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        state.traced.insert(trace_key(&value));
        assert!(project_access(value, &state, tcl_registry::TraceOperation::Read).observed);
        let mut cases: Vec<(&str, &[&str], bool)> = vec![
            ("trace", &["info", "variable", "v"], false),
            ("trace", &["add", "variable", "v", "read", "watch"], false),
            (
                "trace",
                &["remove", "variable", "v", "read", "watch"],
                false,
            ),
            (
                "trace",
                &["add", "variable", "v", "INVALID", "watch"],
                false,
            ),
            ("set", &["v"], true),
        ];
        if matches!(profile, "tcl8.4" | "tcl8.5" | "tcl8.6") {
            cases.extend([
                ("trace", &["vinfo", "v"][..], false),
                ("trace", &["variable", "v", "r", "watch"][..], false),
                ("trace", &["vdelete", "v", "r", "watch"][..], false),
            ]);
        }
        for (command, words, reads_value) in cases {
            let selected = registry
                .resolve_invocation(command, words, dialect.authoring_query())
                .unwrap();
            let facts = selected.facts();
            let arguments =
                tcl_registry::InvocationArguments::literals(words).with_dialect(dialect);
            let source = format!("{command} {}", words.join(" "));
            let image = tcl_lexer::SourceImage::document(&source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let inputs = plan.commands[0]
                .words
                .iter()
                .skip(1)
                .map(|word| {
                    Some(SignatureSourceNameInput::OriginalWord(
                        SignatureSourceNameKey::from_original_native_word(
                            word,
                            tcl_syntax::word_rules::WordValueRules::from_config(&config),
                            policy,
                        )
                        .unwrap(),
                    ))
                })
                .collect();
            let operands = OriginalVariableInvocation::from_original_inputs(inputs, Vec::new());
            for inputs in [None, Some(&operands)] {
                let reads = source_variable_read_places_with_original_operands_impl(
                    &facts, arguments, &state, registry, inputs,
                );
                assert_eq!(reads.len(), usize::from(reads_value), "{profile}/{words:?}");
                assert!(reads.iter().all(|place| place.observed));
                let writes =
                    source_variable_write_places_with_output_order_and_original_operands_impl(
                        &facts, arguments, &state, registry, None, inputs,
                    );
                assert!(writes.is_empty(), "{profile}/{words:?}");
            }
            if !reads_value {
                assert!(facts.arg_roles.iter().any(|&(_, role)| matches!(
                    role,
                    tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                )));
            }
        }
    }
}

#[test]
fn original_variable_trace_join_preserves_all_same_valued_producers() {
    let profile = "tcl8.6";
    let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
    let (mut left, left_inputs) = original_trace_inputs(profile, b"watch\0tail", b"# left");
    let (mut right, right_inputs) = original_trace_inputs(profile, b"watch\0tail", b"# right");
    apply_trace_with_original(
        &mut left,
        &selected_variable_trace(profile, false),
        registry,
        Some(&left_inputs),
    );
    apply_trace_with_original(
        &mut right,
        &selected_variable_trace(profile, false),
        registry,
        Some(&right_inputs),
    );
    let key = trace_key(&left_inputs.access(
        2,
        &left,
        registry,
        tcl_registry::TraceOperation::Read,
        false,
    ));
    let first = left.trace_registrations.get(&key).unwrap()[0].1.clone();
    let second = right.trace_registrations.get(&key).unwrap()[0].1.clone();
    assert!(first.same_data(&second));
    assert_ne!(first, second);
    left.join(&right);
    let reaching = left.trace_registrations.get(&key).unwrap();
    assert_eq!(reaching.len(), 1);
    let joined = &reaching[0].1;
    assert_ne!(*joined, first);
    assert_ne!(*joined, second);
    assert_eq!(first.joined(&second).as_ref(), Some(joined));
    assert!(!left.untracked_traces.contains(&key));
}

#[test]
fn original_variable_trace_copy_cannot_reset_the_complete_lineage_bound() {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
    let (context, operands) = original_trace_inputs("tcl8.6", b"watch", b"# original");
    let mut input = operands.input(4, &context).unwrap().clone();
    let mut copied = 0;
    while let Some(value) = SignatureSourceNameValue::copied_variable_trace_prefix(&input, &context)
    {
        input = SignatureSourceNameInput::OriginalValue(value);
        copied += 1;
        assert!(input.original_word_key().is_none());
        assert!(copied < crate::command_binding::original_name_value::MAX_ORIGINS);
    }
    assert_eq!(
        copied + 1,
        crate::command_binding::original_name_value::MAX_ORIGINS
    );
    assert!(input.is_current(&context));
}

#[test]
fn valid_trace_registration_closes_only_the_selected_native_address_protocol() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let mut state = ResolveContext::for_frame(
        crate::var_resolve::VariableFrameKind::Global,
        crate::var_resolve::BindingIdentity::Bound,
        Some(dialect),
    );
    state.namespace_cells = fresh_namespace_cells(registry);
    state.define_literal("scalar", "VALUE", registry);
    state.define_literal("array(k)", "MEMBER", registry);
    for (words, expected) in [
        (["add", "variable", "x", "read", "{"], true),
        (["add", "variable", "x", "", "watch"], false),
        (["add", "variable", "scalar(k)", "read", "watch"], false),
        (["add", "variable", "array", "unset", "watch"], true),
        (["add", "variable", "array(k)", "unset", "watch"], true),
        (["add", "variable", "::missing::x", "read", "watch"], false),
        (["remove", "variable", "scalar(k)", "read", "watch"], true),
        (
            ["remove", "variable", "::missing::x", "read", "watch"],
            true,
        ),
    ] {
        let facts = registry
            .resolve_invocation("trace", &words, dialect.authoring_query())
            .unwrap()
            .facts();
        assert_eq!(
            variable_trace_registration_is_closed(&state, &facts, registry),
            expected,
            "{words:?}"
        );
    }
    let facts = registry
        .resolve_invocation(
            "trace",
            &["add", "variable", "x", "read", "watch", "extra"],
            dialect.authoring_query(),
        )
        .unwrap()
        .facts();
    assert!(!variable_trace_registration_is_closed(
        &state, &facts, registry
    ));
    state.invocation_dialect = None;
    let facts = registry
        .resolve_invocation(
            "trace",
            &["add", "variable", "x", "read", "watch"],
            dialect.authoring_query(),
        )
        .unwrap()
        .facts();
    assert!(!variable_trace_registration_is_closed(
        &state, &facts, registry
    ));
}

#[test]
// Implementation contract: naming.variable.byte-cell-correspondence
// docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
fn original_alias_operands_ignore_display_and_preserve_runtime_units() {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::naming::{ExecutionNamePolicy, NamePolicyProtocol};
    use tcl_syntax::word_rules::WordValueRules;
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_native_model::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let policy: NamePolicyProtocol = dialect.authored_name_policy().unwrap();
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        for source in [b"v\0tail".as_slice(), br"v\u0000tail", br"v\uD800"] {
            let image = SourceImage::native(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let input = SignatureSourceNameInput::OriginalWord(
                SignatureSourceNameKey::from_original_native_word(
                    &plan.commands[0].words[0],
                    WordValueRules::from_config(&config),
                    policy,
                )
                .unwrap(),
            );
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(dialect);
            state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
            let root = crate::command_binding::SourceNamespaceKey::authored("::");
            state.retain_namespace_world(root.clone(), [root.clone()], Some(policy.recipe()));
            // The logical transition facet cannot select either cell.
            let subject = tcl_registry::TransitionSubject::LocatedLiteral {
                value: "DIFFERENT_DISPLAY".into(),
                argument_index: 0,
            };
            let alias = VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
                local: subject.clone(),
                target: VariableAliasTarget::Global { variable: subject },
                writes_value: false,
                words: tcl_registry::AliasWords::same(0),
            };
            let operands =
                OriginalVariableInvocation::from_original_inputs(vec![Some(input.clone())], vec![]);
            let expected = crate::var_resolve::resolve_original_namespace_variable_bytes(
                input.bytes(),
                &root,
                &state,
                registry,
                false,
            );
            assert_ne!(expected.kind, PlaceKind::Unknown, "{profile}/{source:?}");
            bind_alias_with_compiled_local(&mut state, &alias, registry, None, Some(&operands));
            let local = tcl_syntax::naming::global_local_name_bytes(policy.recipe(), input.bytes())
                .unwrap();
            let read = crate::var_resolve::resolve_evaluated_variable_input(
                tcl_syntax::naming::NativeVariableInputForm::Combined(&local),
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(cell_key(&read), cell_key(&expected), "{profile}/{source:?}");
            assert_eq!(
                read.cell.as_ref().unwrap().name,
                expected.cell.as_ref().unwrap().name
            );
        }
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        let subject = tcl_registry::TransitionSubject::LocatedLiteral {
            value: "v".into(),
            argument_index: 0,
        };
        let alias = VariableCellAliasTransition {
            destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
            local: subject.clone(),
            target: VariableAliasTarget::Global { variable: subject },
            writes_value: false,
            words: tcl_registry::AliasWords::same(0),
        };
        let missing = OriginalVariableInvocation::from_original_inputs(vec![None], vec![]);
        bind_alias_with_compiled_local(&mut state, &alias, registry, None, Some(&missing));
        assert!(
            state.dynamic_bindings,
            "{profile}: missing owner must not use displayed v"
        );
    }
}

fn original_absolute_alias_operands(
    source: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> OriginalVariableInvocation {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    use tcl_lexer::Span;
    use tcl_syntax::word_rules::WordValueRules;
    let parsed = tcl_lexer::native_script_words_in(
        source.clone(),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let inputs = parsed.commands[0].words[1..]
        .iter()
        .map(|word| {
            Some(SignatureSourceNameInput::OriginalWord(
                SignatureSourceNameKey::from_original_native_word(
                    word,
                    WordValueRules::from_config(&config),
                    policy,
                )
                .unwrap(),
            ))
        })
        .collect();
    OriginalVariableInvocation::from_original_inputs(inputs, vec![])
}

#[test]
fn original_absolute_zero_alias_uses_retained_root_without_a_caller_frame() {
    use tcl_lexer::{LexerConfig, SourceImage};
    use tcl_syntax::naming::ExecutionNamePolicy;
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_native_model::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let policy = dialect.authored_name_policy().unwrap();
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let source = SourceImage::native(br"upvar #0 root\uD800 local".as_slice());
        let operands = original_absolute_alias_operands(&source, config, policy);
        let facts = registry
            .resolve_invocation(
                "upvar",
                &["#0", "REPORT_ONLY", "local"],
                dialect.authoring_query(),
            )
            .unwrap()
            .facts();
        let alias = facts
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .find_map(|fact| {
                if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                    Some(alias)
                } else {
                    None
                }
            })
            .unwrap();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        let root = crate::command_binding::SourceNamespaceKey::authored("::");
        state.retain_namespace_world(root.clone(), [root.clone()], Some(policy.recipe()));
        assert!(state.caller.is_none());
        assert!(
            state
                .selected_frame_context(FrameLevel::Absolute(0))
                .is_none()
        );
        let expected = crate::var_resolve::resolve_original_namespace_variable_bytes(
            b"root\xed\xa0\x80",
            &root,
            &state,
            registry,
            false,
        );
        let selected = original_alias_target(&state, &alias.target, &operands, registry, false);
        assert_ne!(selected.kind, PlaceKind::Unknown, "{profile}");
        assert_eq!(cell_key(&selected), cell_key(&expected), "{profile}");
        bind_alias_with_compiled_local(&mut state, alias, registry, None, Some(&operands));
        let read = crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Combined(b"local"),
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(cell_key(&read), cell_key(&expected), "{profile}");
        let mut missing = state.clone();
        missing.namespace_identities.remove(&root);
        assert_eq!(
            original_alias_target(&missing, &alias.target, &operands, registry, false).kind,
            PlaceKind::Unknown
        );
        let relative = VariableAliasTarget::CallerSelectedFrame {
            frame: CallerFrameSelection::DefaultCaller,
            variable: tcl_registry::TransitionSubject::LocatedLiteral {
                value: "REPORT_ONLY".into(),
                argument_index: 1,
            },
        };
        assert_eq!(
            original_alias_target(&state, &relative, &operands, registry, false).kind,
            PlaceKind::Unknown
        );
    }
}

#[test]
fn normal_alias_completion_requires_the_actual_destination_and_selected_frame() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let mut state = ResolveContext::for_frame(
        crate::var_resolve::VariableFrameKind::Global,
        crate::var_resolve::BindingIdentity::Bound,
        Some(dialect),
    );
    state.namespace_cells = fresh_namespace_cells(registry);
    state.define_literal("b", "100", registry);
    let words = ["0", "b", "link"];
    let arguments = tcl_registry::InvocationArguments::literals(&words).with_dialect(dialect);
    let facts = registry
        .resolve_invocation("upvar", &["0", "b", "link"], dialect.authoring_query())
        .unwrap()
        .facts();
    let before = state.clone();
    assert!(alias_registration_is_closed(
        &state, &facts, arguments, registry
    ));
    state.define_literal("link", "DIRECT", registry);
    assert!(!alias_registration_is_closed(
        &state, &facts, arguments, registry
    ));
    state.contents_world = crate::var_resolve::ContentsWorld::Unknown;
    state.contents_presence.remove("::link");
    assert!(!alias_registration_is_closed(
        &state, &facts, arguments, registry
    ));
    let mut state = before;
    state.caller = None;
    let words = ["1", "b", "link"];
    let arguments = tcl_registry::InvocationArguments::literals(&words).with_dialect(dialect);
    let facts = registry
        .resolve_invocation("upvar", &["1", "b", "link"], dialect.authoring_query())
        .unwrap()
        .facts();
    assert!(!alias_registration_is_closed(
        &state, &facts, arguments, registry
    ));
}

#[test]
fn reached_iteration_assignments_replace_missing_incoming_provenance() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let mut state = ResolveContext::for_function("::p");
    state.invocation_dialect = Some(dialect);
    state.define_literal("unrelated", "KEEP", registry);
    let words = [tcl_registry::InvocationWord::Literal("x y")];
    let arguments = tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
    assert_eq!(
        transfer_source_iteration_bindings(&mut state, &[0], arguments, registry, 77),
        SourceIterationBindingOutcome::Entered
    );
    for name in ["x", "y"] {
        let target = resolve_literal_access(
            name,
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            state.contents_origin(&target),
            crate::var_resolve::ContentsOrigin::WrittenAt(77)
        );
        assert_eq!(
            state.contents_presence(&target),
            crate::var_resolve::ContentsPresence::Defined
        );
        assert_eq!(state.literal_value(name, registry), None);
    }
    assert_eq!(state.literal_value("unrelated", registry), Some("KEEP"));
}

#[test]
fn iteration_kind_error_preserves_prior_stores_and_selected_container_policy() {
    // Actual C 8.4–9.1 writes x before failing on array a. Jim 0.84
    // replaces the dictionary-valued root and enters the body.
    for (profile, dialect, expected) in [
        (
            "tcl8.6",
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            SourceIterationBindingOutcome::StoreError,
        ),
        (
            "jim",
            tcl_native_model::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            ),
            SourceIterationBindingOutcome::Entered,
        ),
    ] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.define_literal("a(k)", "OLD", registry);
        let element = resolve_literal_access(
            "a(k)",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        state.record_contents_write(&element, 1, false);
        let words = [tcl_registry::InvocationWord::Literal("x a")];
        let arguments = tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
        assert_eq!(
            transfer_source_iteration_bindings(&mut state, &[0], arguments, registry, 77),
            expected,
            "{profile}"
        );
        let first = resolve_literal_access(
            "x",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            state.contents_origin(&first),
            crate::var_resolve::ContentsOrigin::WrittenAt(77),
            "{profile}"
        );
    }
}

#[test]
fn native_loop_conditions_retain_source_reads_without_replay_boundaries() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for loop_script in [
        "while {$x} {incr x -1}",
        "for {set i 0} {$x} {incr x -1} {}",
    ] {
        let source = format!("proc p {{}} {{set x 1; {loop_script}}}");
        let unit = crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::p").unwrap();
        let conditions: Vec<_> = function
            .cfg
            .blocks
            .iter()
            .filter_map(|(&block, body)| {
                let Some(crate::cfg::Terminator::Branch {
                    condition,
                    condition_base,
                    ..
                }) = &body.terminator
                else {
                    return None;
                };
                matches!(condition, crate::expr_ast::ExprNode::Var { .. }).then_some((
                    block,
                    condition,
                    *condition_base,
                ))
            })
            .collect();
        assert_eq!(conditions.len(), 1, "{loop_script}");
        for (block, condition, base) in conditions {
            assert!(
                function.cfg.condition_binding_sites.contains_key(&block),
                "{loop_script}"
            );
            assert!(
                !function.cfg.command_boundary_sites.contains_key(&block),
                "{loop_script}"
            );
            let view = crate::ssa::SsaSourceView::at_terminator(&function.ssa, block);
            assert!(view.source_tokens().is_some(), "{loop_script}");
            assert!(
                view.read_expression_variable_contents(condition, base, registry)
                    .is_some(),
                "{loop_script}"
            );
        }
    }
}
use super::*;
use tcl_registry::TransitionSubject;

#[test]
fn ordered_output_addresses_decline_only_after_possible_observers() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let words = [
        tcl_registry::InvocationWord::Literal("error fail"),
        tcl_registry::InvocationWord::Literal("result"),
        tcl_registry::InvocationWord::Literal("options"),
    ];
    let arguments = tcl_registry::InvocationArguments::structured(&words);
    let facts = registry
        .resolve_structured_invocation(
            tcl_registry::InvocationWords::from_arguments(
                tcl_registry::InvocationWord::Literal("catch"),
                arguments,
            ),
            registry.own_surface_query(),
        )
        .resolved()
        .unwrap()
        .facts();
    let mut state = ResolveContext::for_function("::p");
    state.define_literal("result", "OLD", registry);
    state.define_literal("options", "OLD", registry);
    let result = variable_operand_access(
        arguments,
        1,
        &state,
        registry,
        tcl_registry::TraceOperation::Write,
        false,
    );
    state.traced.insert(cell_key(&result));
    assert_eq!(
        uncertain_variable_output_addresses(&facts, arguments, &state, registry, Some(&[1, 2])),
        std::collections::BTreeSet::from([2])
    );
    assert!(
        uncertain_variable_output_addresses(&facts, arguments, &state, registry, Some(&[2, 1]))
            .is_empty()
    );
    assert_eq!(
        uncertain_variable_output_addresses(&facts, arguments, &state, registry, None),
        std::collections::BTreeSet::from([2])
    );
    let writes = source_variable_write_places_with_output_order(
        &facts,
        arguments,
        &state,
        registry,
        Some(&[1, 2]),
    );
    assert!(writes.iter().any(|place| place.kind == PlaceKind::Unknown));
    assert!(!writes.iter().any(|place| place.name == "options"));
    let options = variable_operand_access(
        arguments,
        2,
        &state,
        registry,
        tcl_registry::TraceOperation::Write,
        false,
    );
    state.traced.insert(cell_key(&options));
    assert_eq!(
        uncertain_variable_output_addresses(&facts, arguments, &state, registry, None),
        std::collections::BTreeSet::from([1, 2])
    );
}

#[test]
fn bounded_array_store_preserves_unrelated_namespace_and_value_facts() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let words = [
        tcl_registry::InvocationWord::ArrayElementName { root: "a" },
        tcl_registry::InvocationWord::Literal("NEW"),
    ];
    let arguments = tcl_registry::InvocationArguments::structured(&words);
    let facts = registry
        .resolve_structured_invocation(
            tcl_registry::InvocationWords::from_arguments(
                tcl_registry::InvocationWord::Literal("set"),
                arguments,
            ),
            registry.own_surface_query(),
        )
        .resolved()
        .expect("native set")
        .facts();
    let mut state = ResolveContext::for_function("::top");
    state.define_literal("unrelated", "KEPT", registry);
    let writes = source_variable_write_places(&facts, arguments, &state, registry);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].kind, PlaceKind::ArrayElem);
    assert_eq!(writes[0].name, "a");
    transfer_source_namespace_cells(&mut state, &facts, arguments, registry, false, 10);
    assert_eq!(state.literal_value("unrelated", registry), Some("KEPT"));
    assert!(state.namespace_cells.present.contains("::a"));
    assert!(!state.dynamic_bindings);
}

#[test]
fn caller_reference_distinguishes_defined_unknown_contents_from_absence() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let mut parent = ResolveContext::for_function("::parent");
    parent.invocation_dialect = Some(tcl_native_model::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    ));
    parent.define_unknown_contents("parameter", registry);
    let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "reference-call".to_owned(),
    };
    let mut child = parent.enter_called_frame(&frame);
    assert_eq!(
        bind_caller_reference(&mut child, "v", Some("parameter"), registry),
        CallerReferenceBinding::Bound
    );
    assert_eq!(
        crate::var_resolve::canonical_literal_variable_name("v", &child, registry),
        crate::var_resolve::canonical_literal_variable_name("parameter", &parent, registry)
    );
    assert_eq!(child.literal_value("v", registry), None);
    let mut child = parent.enter_called_frame(&frame);
    assert_eq!(
        bind_caller_reference(&mut child, "v", Some("absent"), registry),
        CallerReferenceBinding::Missing
    );
    parent.widen();
    let mut child = parent.enter_called_frame(&frame);
    assert_eq!(
        bind_caller_reference(&mut child, "v", Some("absent"), registry),
        CallerReferenceBinding::Unknown
    );
}

fn link(state: &mut ResolveContext, target: &str, local: &str) {
    bind_authored_alias_values(
        state,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal(local.to_owned()),
            target: &VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(TransitionSubject::Literal("0".to_owned())),
                variable: TransitionSubject::Literal(target.to_owned()),
            },
        },
        &CommandRegistry::build_default(),
    );
}

#[test]
fn actual_caller_increment_preserves_the_shared_namespace_contents() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut root = ResolveContext::for_function("::top");
    root.invocation_dialect = Some(tcl_native_model::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    root.namespace_cells = fresh_namespace_cells(registry);
    let set_words = [
        tcl_registry::InvocationWord::Literal("n"),
        tcl_registry::InvocationWord::Literal("1"),
    ];
    let set_args = tcl_registry::InvocationArguments::structured(&set_words)
        .with_dialect(root.invocation_dialect.unwrap());
    let set_facts = registry
        .resolve_invocation("set", &["n", "1"], registry.own_surface_query())
        .unwrap()
        .facts();
    transfer_source_namespace_cells(&mut root, &set_facts, set_args, registry, false, 0);
    assert_eq!(root.literal_value("n", registry), Some("1"));
    assert!(
        root.namespace_cells.present.contains("::n"),
        "{set_facts:#?}: {root:#?}"
    );
    let mut child =
        root.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "bump@1".to_owned(),
        });
    bind_authored_alias_values(
        &mut child,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal("v".to_owned()),
            target: &VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(TransitionSubject::Literal("1".to_owned())),
                variable: TransitionSubject::Literal("n".to_owned()),
            },
        },
        registry,
    );
    let incr_words = [tcl_registry::InvocationWord::Literal("v")];
    let incr_args = tcl_registry::InvocationArguments::structured(&incr_words)
        .with_dialect(child.invocation_dialect.unwrap());
    let incr_facts = registry
        .resolve_invocation("incr", &["v"], registry.own_surface_query())
        .unwrap()
        .facts();
    transfer_source_namespace_cells(&mut child, &incr_facts, incr_args, registry, false, 10);
    assert_eq!(
        child.literal_value("v", registry),
        Some("2"),
        "{incr_facts:#?}: {child:#?}"
    );
    assert!(
        child.namespace_cells.present.contains("::n"),
        "{incr_facts:#?}: {child:#?}"
    );
    let restored = crate::var_resolve::restore_execution_frame(&root, &child);
    assert_eq!(restored.literal_value("n", registry), Some("2"));
    assert!(restored.namespace_cells.present.contains("::n"));
}

#[test]
fn alias_chain_retargeting_follows_the_runtime_link_policy() {
    for (dialect, expected) in [
        ("tcl8.4", "OLD"),
        ("tcl8.5", "OLD"),
        ("tcl8.6", "OLD"),
        ("tcl9.0", "OLD"),
        ("tcl9.1", "OLD"),
        ("jim", "NEW"),
    ] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_native_model::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
        ));
        state.define_literal("x", "OLD", registry);
        state.define_literal("z", "NEW", registry);
        link(&mut state, "x", "a");
        link(&mut state, "a", "b");
        link(&mut state, "z", "a");
        assert_eq!(
            state.literal_value("b", registry),
            Some(expected),
            "{dialect}: {state:#?}"
        );
    }
}

#[test]
fn namespace_alias_destinations_cannot_retain_c_procedure_cells() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let mut state = ResolveContext::for_function("::p");
    state.known_namespaces.insert("::N".to_owned());
    state.namespace_cells.closed = true;
    let alias = AliasBindingValues {
        destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
        local: &TransitionSubject::Literal("::N::y".to_owned()),
        target: &VariableAliasTarget::CallerSelectedFrame {
            frame: CallerFrameSelection::Explicit(TransitionSubject::Literal("0".to_owned())),
            variable: TransitionSubject::Literal("x".to_owned()),
        },
    };
    assert_eq!(
        alias_value_binding_validity(&state, alias, registry),
        AliasBindingValidity::NamespaceToLocal
    );
    bind_authored_alias_values(&mut state, alias, registry);
    assert!(!state.namespace_cells.present.contains("::N::y"));
    assert!(!state.namespace_alias_bindings.contains_key("::N::y"));
}

#[test]
fn namespace_deletion_turns_over_scalar_lifetimes_and_scopes_clobbers() {
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::p");
    state.known_namespaces.insert("::N".to_owned());
    state.namespace_cells.present.insert("::N::x".to_owned());
    let before = resolve_place("::N::x", &state, false, &registry);
    apply_namespace(
        &mut state,
        &NamespaceTransition::Delete {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                "::N".to_owned(),
            )),
        },
        20,
    );
    let after = resolve_place("::N::x", &state, false, &registry);
    assert_ne!(
        crate::var_resolve::canonical_place_name(&before),
        crate::var_resolve::canonical_place_name(&after)
    );
    assert_eq!(
        after.cell.as_ref().map(|cell| cell.generation),
        Some(CellGeneration::After(20))
    );
    let destruction = place::unknown_namespace("::N");
    assert!(place::overlap(&destruction, &before));
    assert!(!place::overlap(
        &destruction,
        &resolve_place("local", &state, false, &registry)
    ));
    assert!(!place::overlap(
        &destruction,
        &resolve_place("::Other::x", &state, false, &registry)
    ));
}

#[test]
fn current_frame_alias_retargets_at_the_binding_operation() {
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    link(&mut state, "x", "y");
    let first = resolve_place("y", &state, false, &registry);
    assert_eq!(first, resolve_place("x", &state, false, &registry));
    link(&mut state, "z", "y");
    let second = resolve_place("y", &state, false, &registry);
    assert_eq!(second, resolve_place("z", &state, false, &registry));
    assert!(!place::overlap(&first, &second));
}

#[test]
fn scalar_unset_preserves_alias_but_array_unset_dangles_element_link() {
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    link(&mut state, "x", "y");
    destroy_root(&mut state, "y", 10, &registry);
    assert_eq!(
        resolve_place("y", &state, false, &registry),
        resolve_place("x", &state, false, &registry)
    );
    link(&mut state, "a(k)", "element");
    let original = resolve_place("element", &state, false, &registry);
    destroy_root(&mut state, "a", 20, &registry);
    assert!(state.unknown_bindings.contains("element"));
    assert!(!place::overlap(
        &original,
        &resolve_place("a(k)", &state, false, &registry)
    ));
}

#[test]
fn conditional_binding_join_cannot_claim_either_target() {
    let registry = CommandRegistry::build_default();
    let mut left = ResolveContext::for_function("::f");
    let mut right = left.clone();
    link(&mut left, "x", "y");
    link(&mut right, "z", "y");
    left.join(&right);
    assert!(left.unknown_bindings.contains("y"));
    assert!(crate::var_resolve::canonical_variable_name("y", &left, &registry).is_none());
}
#[test]
fn exact_trace_removal_counts_duplicates_and_tracks_cells_across_alias_retargeting() {
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    link(&mut state, "x", "y");
    let add = TraceTransition::Add {
        target: TraceTarget::Variable(TransitionSubject::Literal("y".to_owned())),
        operations: tcl_registry::TraceOperationSet::Known(vec![
            tcl_registry::TraceOperation::Read,
        ]),
        prefix: TransitionSubject::Literal("callback".to_owned()),
    };
    apply_trace(&mut state, &add, &registry);
    apply_trace(&mut state, &add, &registry);
    link(&mut state, "z", "y");
    assert!(!resolve_place("y", &state, false, &registry).observed);
    assert!(resolve_place("x", &state, false, &registry).observed);
    let remove = TraceTransition::Remove {
        target: TraceTarget::Variable(TransitionSubject::Literal("x".to_owned())),
        operations: tcl_registry::TraceOperationSet::Known(vec![
            tcl_registry::TraceOperation::Read,
        ]),
        prefix: TransitionSubject::Literal("callback".to_owned()),
    };
    apply_trace(&mut state, &remove, &registry);
    assert!(resolve_place("x", &state, false, &registry).observed);
    apply_trace(&mut state, &remove, &registry);
    assert!(!resolve_place("x", &state, false, &registry).observed);
}
fn trace(state: &mut ResolveContext, variable: &str, operation: tcl_registry::TraceOperation) {
    apply_trace(
        state,
        &TraceTransition::Add {
            target: TraceTarget::Variable(TransitionSubject::Literal(variable.to_owned())),
            operations: tcl_registry::TraceOperationSet::Known(vec![operation]),
            prefix: TransitionSubject::Literal("callback".to_owned()),
        },
        &CommandRegistry::build_default(),
    );
}

#[test]
fn trace_callbacks_follow_operation_and_exact_element_address() {
    use tcl_registry::TraceOperation::{Read, Unset, Write};
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    trace(&mut state, "a(k)", Write);
    assert!(resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
    assert!(!resolve_literal_access("a(j)", &state, false, &registry, Write).observed);
    assert!(!resolve_literal_access("a(k)", &state, false, &registry, Read).observed);
    assert!(!resolve_literal_access("a", &state, true, &registry, Unset).observed);
    assert!(resolve_literal_access("a", &state, true, &registry, Write).observed);
    trace(&mut state, "a", Read);
    assert!(resolve_literal_access("a(j)", &state, false, &registry, Read).observed);
    assert!(!resolve_literal_access("unrelated", &state, false, &registry, Write).observed);
    trace(&mut state, "a(j)", Unset);
    assert!(resolve_literal_access("a", &state, false, &registry, Unset).observed);
}

#[test]
fn braced_substitution_preserves_literal_element_and_dollar_name() {
    use tcl_registry::TraceOperation::Read;
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    trace(&mut state, "a($i)", Read);
    trace(&mut state, "$x", Read);
    let literal =
        crate::var_resolve::resolve_substitution_access("${a($i)}", &state, &registry, Read);
    assert_eq!(
        literal.index.as_ref().map(|index| index.kind),
        Some(place::IndexKind::Literal)
    );
    assert!(literal.observed);
    assert!(!resolve_literal_access("a(k)", &state, false, &registry, Read).observed);
    let dynamic =
        crate::var_resolve::resolve_substitution_access("$a($i)", &state, &registry, Read);
    assert_eq!(
        dynamic.index.as_ref().map(|index| index.kind),
        Some(place::IndexKind::Dynamic)
    );
    assert!(dynamic.observed);
    assert!(
        crate::var_resolve::resolve_substitution_access("${$x}", &state, &registry, Read).observed
    );
}

#[test]
fn unset_removes_element_trace_before_recreation() {
    use tcl_registry::TraceOperation::Write;
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    trace(&mut state, "a(k)", Write);
    destroy_root(&mut state, "a(k)", 10, &registry);
    assert!(!resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
    trace(&mut state, "a(k)", Write);
    destroy_root(&mut state, "a", 20, &registry);
    assert!(!resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
}

#[test]
fn namespace_deletion_removes_trace_registrations_from_old_cells() {
    use tcl_registry::TraceOperation::Write;
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_function("::f");
    state.known_namespaces.insert("::N".to_owned());
    trace(&mut state, "::N::x", Write);
    apply_namespace(
        &mut state,
        &NamespaceTransition::Delete {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                "::N".to_owned(),
            )),
        },
        20,
    );
    state.known_namespaces.insert("::N".to_owned());
    assert!(!resolve_literal_access("::N::x", &state, false, &registry, Write).observed);
}

#[test]
fn repeated_root_store_allocates_current_lifetime_without_reviving_dangling_element_alias() {
    let registry = CommandRegistry::build_default();
    let mut initial = ResolveContext::for_function("::f");
    link(&mut initial, "a(k)", "element");
    let mut repeated = initial.clone();
    destroy_root(&mut repeated, "a", 10, &registry);
    initial.join(&repeated);
    let before = resolve_literal_place("a", &initial, false, &registry);
    assert_eq!(
        before.cell.as_ref().map(|cell| cell.generation),
        Some(CellGeneration::Unknown)
    );
    establish_written_lifetime(&mut initial, &before, 20);
    assert_eq!(
        resolve_literal_place("a", &initial, false, &registry)
            .cell
            .as_ref()
            .map(|cell| cell.generation),
        Some(CellGeneration::After(20))
    );
    assert!(initial.unknown_bindings.contains("element"));
}
#[test]
fn jim_aliases_follow_names_after_array_and_namespace_destruction() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let mut state = ResolveContext::for_function("::f");
    state.invocation_dialect = Some(tcl_native_model::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    ));
    link(&mut state, "a(k)", "element");
    destroy_root(&mut state, "a", 10, registry);
    assert!(!state.unknown_bindings.contains("element"));
    assert_eq!(
        resolve_literal_place("element", &state, false, registry).cell,
        resolve_literal_place("a(k)", &state, false, registry).cell
    );
    bind_authored_alias_values(
        &mut state,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal("global_alias".to_owned()),
            target: &VariableAliasTarget::Global {
                variable: TransitionSubject::Literal("::N::x".to_owned()),
            },
        },
        registry,
    );
    apply_namespace(
        &mut state,
        &NamespaceTransition::Delete {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                "::N".to_owned(),
            )),
        },
        20,
    );
    assert!(!state.unknown_bindings.contains("global_alias"));
    assert_eq!(
        resolve_literal_place("global_alias", &state, false, registry).cell,
        resolve_literal_place("::N::x", &state, false, registry).cell
    );
}
#[test]
fn actual_parent_frame_frames_select_bound_cells_and_literal_values() {
    let registry = CommandRegistry::build_default();
    let mut global = ResolveContext::for_function("::top");
    global.define_literal("seed", "GLOBAL", &registry);
    let mut parent_frame =
        global.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::N".to_owned(),
            identity: "parent_frame@1".to_owned(),
        });
    parent_frame.define_literal("seed", "CALLER", &registry);
    let mut child_frame =
        parent_frame.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::N".to_owned(),
            identity: "child_frame@1".to_owned(),
        });
    bind_authored_alias_values(
        &mut child_frame,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal("alias".to_owned()),
            target: &VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::DefaultCaller,
                variable: TransitionSubject::Literal("seed".to_owned()),
            },
        },
        &registry,
    );
    assert_eq!(
        child_frame.literal_value("alias", &registry),
        Some("CALLER")
    );
    assert_eq!(
        child_frame.substitution_literal_value("${alias}", &registry),
        Some("CALLER")
    );
    assert_eq!(
        child_frame
            .selected_frame_context(FrameLevel::Absolute(0))
            .unwrap()
            .literal_value("seed", &registry),
        Some("GLOBAL")
    );
    assert!(
        child_frame
            .selected_frame_context(FrameLevel::Relative(3))
            .is_none()
    );
    let lexical = global.in_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: "::N".to_owned(),
        identity: "definition-only".to_owned(),
    });
    assert!(
        lexical
            .selected_frame_context(FrameLevel::Relative(1))
            .is_none()
    );
}

#[test]
fn namespace_aliases_persist_and_deleted_targets_do_not_rebind_in_c_tcl() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut parent = ResolveContext::for_function("::p");
    parent.namespace_cells.closed = true;
    parent
        .known_namespaces
        .extend(["::N".to_owned(), "::Target".to_owned()]);
    parent.define_literal("::Target::x", "OLD", registry);
    let mut child = parent.enter_called_frame(
        &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace-eval@1".to_owned(),
        },
    );
    bind_authored_alias_values(
        &mut child,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal("link".to_owned()),
            target: &VariableAliasTarget::Global {
                variable: TransitionSubject::Literal("::Target::x".to_owned()),
            },
        },
        registry,
    );
    let restored = crate::var_resolve::restore_execution_frame(&parent, &child);
    assert_eq!(restored.literal_value("::N::link", registry), Some("OLD"));
    let mut child = restored.enter_called_frame(
        &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace-eval@2".to_owned(),
        },
    );
    apply_namespace(
        &mut child,
        &NamespaceTransition::Delete {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                "::Target".to_owned(),
            )),
        },
        20,
    );
    child.known_namespaces.insert("::Target".to_owned());
    child.define_literal("::Target::x", "NEW", registry);
    let restored = crate::var_resolve::restore_execution_frame(&restored, &child);
    assert_eq!(restored.literal_value("::Target::x", registry), Some("NEW"));
    assert!(
        crate::var_resolve::canonical_literal_variable_name("::N::link", &restored, registry)
            .is_none()
    );
}

#[test]
fn actual_namespace_activation_contributes_to_numeric_frame_depth() {
    let global = ResolveContext::for_function("::top");
    let namespace = global.enter_called_frame(
        &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace-eval@1".to_owned(),
        },
    );
    let procedure =
        namespace.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::N".to_owned(),
            identity: "call@1".to_owned(),
        });
    assert!(
        procedure
            .selected_frame_context(FrameLevel::Absolute(1))
            .unwrap()
            .namespace_scope()
    );
    assert!(
        procedure
            .selected_frame_context(FrameLevel::Relative(2))
            .unwrap()
            .global_frame()
    );
    assert!(
        procedure
            .selected_frame_context(FrameLevel::Absolute(3))
            .is_none()
    );
}
#[test]
fn scalar_alias_join_preserves_binding_contents_without_faking_a_root_lifetime() {
    let registry = CommandRegistry::build_default();
    let mut initial = ResolveContext::for_function("::p");
    link(&mut initial, "x", "alias");
    let mut repeated = initial.clone();
    destroy_root(&mut repeated, "x", 10, &registry);
    initial.join(&repeated);
    assert!(!initial.unknown_bindings.contains("alias"));
    let alias = resolve_literal_place("alias", &initial, false, &registry);
    assert_eq!(
        alias.cell.as_ref().map(|cell| cell.generation),
        Some(CellGeneration::Unknown)
    );
    assert_eq!(
        crate::var_resolve::canonical_binding_value_name(&alias),
        crate::var_resolve::canonical_literal_variable_name("x", &initial, &registry)
    );
    assert!(crate::var_resolve::canonical_place_name(&alias).is_none());
    establish_written_lifetime(&mut initial, &alias, 20);
    initial.define_literal("alias", "NEW", &registry);
    assert_eq!(initial.literal_value("x", &registry), Some("NEW"));
}

#[test]
fn actual_parent_frame_retains_outward_alias_and_contents_effects() {
    let registry = CommandRegistry::build_default();
    let mut parent_frame = ResolveContext::for_function("::p");
    parent_frame.define_literal("x", "OLD", &registry);
    link(&mut parent_frame, "x", "alias");
    let child_frame =
        parent_frame.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "call@1".to_owned(),
        });
    let mut selected = child_frame
        .selected_frame_context(FrameLevel::Relative(1))
        .unwrap();
    link(&mut selected, "z", "alias");
    selected.define_literal("alias", "NEW", &registry);
    let restored_child_frame = crate::var_resolve::restore_execution_frame(&child_frame, &selected);
    let restored_parent_frame =
        crate::var_resolve::restore_execution_frame(&parent_frame, &restored_child_frame);
    let selected_parent_frame = restored_child_frame
        .selected_frame_context(FrameLevel::Relative(1))
        .unwrap();
    assert_eq!(
        selected_parent_frame.literal_value("alias", &registry),
        Some("NEW")
    );
    assert_eq!(
        selected_parent_frame.literal_value("x", &registry),
        Some("OLD")
    );
    // The return adapter must retain binding retargeting published on the
    // actual selected parent_frame, not copy the child_frame's own local alias map.
    assert_eq!(
        restored_parent_frame.literal_value("alias", &registry),
        Some("NEW")
    );
}
#[test]
fn global_declarations_do_not_override_c_namespace_lookup() {
    let registry = CommandRegistry::build_default();
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut state = ResolveContext::for_namespace("::N");
        state.invocation_dialect = Some(tcl_native_model::InvocationDialect::for_version(version));
        state.namespace_cells.closed = true;
        state
            .namespace_cells
            .present
            .extend(["::x".to_owned(), "::N::x".to_owned()]);
        bind_authored_alias_values(
            &mut state,
            AliasBindingValues {
                destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
                local: &TransitionSubject::Literal("x".to_owned()),
                target: &VariableAliasTarget::Global {
                    variable: TransitionSubject::Literal("x".to_owned()),
                },
            },
            &registry,
        );
        assert!(!state.alias_bindings.contains_key("x"));
        assert_eq!(
            cell_key(&resolve_literal_place("x", &state, false, &registry)),
            crate::var_resolve::VariableCellKey::Authored("::N::x".into())
        );
    }
}

#[test]
fn namespace_alias_destination_does_not_retarget_a_same_named_global() {
    let registry = CommandRegistry::build_default();
    let mut state = ResolveContext::for_namespace("::N");
    state.invocation_dialect = Some(tcl_native_model::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    state.namespace_cells.closed = true;
    state.namespace_cells.present.insert("::link".to_owned());
    bind_authored_alias_values(
        &mut state,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: &TransitionSubject::Literal("link".to_owned()),
            target: &VariableAliasTarget::Global {
                variable: TransitionSubject::Literal("::Target::x".to_owned()),
            },
        },
        &registry,
    );
    assert!(state.namespace_alias_bindings.contains_key("::N::link"));
    assert!(!state.namespace_alias_bindings.contains_key("::link"));
    assert_eq!(
        cell_key(&resolve_literal_place("::link", &state, false, &registry)),
        crate::var_resolve::VariableCellKey::Authored("::link".into())
    );
    assert_eq!(
        cell_key(&resolve_literal_place(
            "::N::link",
            &state,
            false,
            &registry
        )),
        crate::var_resolve::VariableCellKey::Authored("::Target::x".into())
    );
}
#[test]
fn no_value_list_append_preserves_existing_cells_and_initialises_missing_cells() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
        let dialect = tcl_native_model::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
        );
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.define_literal("x", "OLD", registry);
        let arguments = tcl_registry::InvocationArguments::literals(&["x"]).with_dialect(dialect);
        let facts = registry
            .resolve_invocation("lappend", &["x"], dialect.authoring_query())
            .expect("native no-value form")
            .facts();
        let before = resolve_literal_place("x", &state, false, registry);
        let origin = state.read_contents_origin(&before, registry);
        assert_eq!(
            source_variable_write_places(&facts, arguments, &state, registry).len(),
            0
        );
        transfer_source_namespace_cells(&mut state, &facts, arguments, registry, false, 50);
        assert_eq!(state.literal_value("x", registry), Some("OLD"));
        assert_eq!(state.read_contents_origin(&before, registry), origin);
        let missing = tcl_registry::InvocationArguments::literals(&["new"]).with_dialect(dialect);
        let missing_facts = registry
            .resolve_invocation("lappend", &["new"], dialect.authoring_query())
            .expect("native no-value form")
            .facts();
        let target = resolve_literal_place("new", &state, false, registry);
        state.contents_presence.insert(
            crate::var_resolve::cell_key(&target),
            crate::var_resolve::ContentsPresence::Undefined,
        );
        transfer_source_namespace_cells(&mut state, &missing_facts, missing, registry, false, 60);
        assert_eq!(
            state.literal_value("new", registry),
            Some(""),
            "{environment}"
        );
    }
}

/// The interpreter creates its startup variables before the document
/// runs, so reading one at file level does not raise and what follows is
/// reached.
///
/// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print `1 1 1 1 1 1` for
/// `puts [list [info exists argv] [info exists argv0] [info exists env]
/// [info exists tcl_version] [info exists tcl_platform] [info exists
/// auto_path]]`, and `1` for `catch {set x $never_created}`.
#[test]
fn a_startup_variable_read_at_file_level_reaches_what_follows() {
    let unresolved = |source: &str, release: &str| {
        crate::analyser::Analyser::new()
            .analyse(source, release)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W123)
            .count()
    };
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for read in [
            "$argv",
            "$argv0",
            "$env(HOME)",
            "$tcl_version",
            "$tcl_platform(os)",
            "$auto_path",
        ] {
            let source = format!("set seen {read}\nnosuch\n");
            assert_eq!(unresolved(&source, release), 1, "{release}: {read}");
        }
        // A name the interpreter never created still ends the script.
        assert_eq!(
            unresolved("set seen $never_created\nnosuch\n", release),
            0,
            "{release}"
        );
    }
}
