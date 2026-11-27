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
fn original_parameter_source_scope_keeps_allocated_and_declared_owners_separate() {
    fn check(
        bindings: &super::SourceCommandBindings,
        registry: &tcl_registry::CommandRegistry,
        source: &str,
        config: tcl_lexer::LexerConfig,
        expected_allocated: Option<bool>,
    ) {
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
            "upvar 1 $name cell",
            offset,
            config,
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        let scope = binding
            .original_body_parameter_source_scope(&tokens)
            .expect("actual source row and its own body/formal owner");
        let allocated = matches!(
            scope.entry.as_ref(),
            super::OriginalDiagnosticFrameEntry::Body(_)
        );
        let declared = matches!(
            scope.entry.as_ref(),
            super::OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
        );
        assert!(allocated || declared);
        if let Some(expected) = expected_allocated {
            assert_eq!(allocated, expected);
        }
        assert!(scope.matches_parameters(&["name", "spare"]));
        assert!(!scope.matches_parameters(&["spare", "name"]));
        assert_eq!(
            scope
                .original_words(&tokens, config, registry)
                .unwrap()
                .len(),
            4
        );
        if declared {
            assert!(
                bindings
                    .conditional_body_entry_at(bindings.source_origin().unwrap(), offset)
                    .is_none()
            );
        } else {
            assert!(
                bindings
                    .conditional_body_entry_at(bindings.source_origin().unwrap(), offset)
                    .is_some()
            );
            let second_offset = u32::try_from(source.rfind("upvar").unwrap()).unwrap();
            assert_ne!(second_offset, offset);
            let second = bindings
                .conditional_body_entry_at(bindings.source_origin().unwrap(), second_offset)
                .unwrap();
            assert!(
                scope
                    .layout
                    .original_body_parameter_source_words(&tokens, &second, config, registry,)
                    .is_none()
            );
        }
        let mut config = config;
        config.strict_quoting = !config.strict_quoting;
        assert!(scope.original_words(&tokens, config, registry).is_none());
        let mut withdrawn = binding.clone();
        withdrawn.declaration_layout_observations = None;
        assert!(
            withdrawn
                .original_body_parameter_source_scope(&tokens)
                .is_none()
        );
    }
    let source = "proc p {name spare} {upvar 1 $name cell; set cell VALUE}\nproc q {name spare} {upvar 1 $name cell; set cell VALUE}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let bindings = super::SourceCommandBindings::analyse(source, config, registry);
    check(&bindings, registry, source, config, Some(true));
    let analysis = crate::analyser::Analyser::new().analyse(source, "jimtcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    check(
        analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref(),
        input.borrowed_context_registry().commands(),
        source,
        input.lexer_config(),
        None,
    );
}

#[test]
fn logical_receiver_catalogues_retain_parent_member_and_vector_without_native_context() {
    let source = "oo::class create C {method m {arg} {set local $arg}}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
    let cursor = offset + 5;
    let binding = bindings.invocation_at_source("", offset);
    let rows = binding
        .declaration_layout_observations
        .as_deref()
        .expect("genuine retained Logical receiver catalogue rows");
    let original = super::original_source_catalogue_layouts(rows)
        .unwrap()
        .next()
        .unwrap();
    let super::OriginalDiagnosticFrameEntry::LogicalReceiver(body) = original.entry.as_ref() else {
        panic!("distinct Logical member-body issuer");
    };
    assert_eq!(
        original.issuer,
        super::DeclarationLayoutIssuer::ConditionalReceiverSource
    );
    assert_eq!(body.input, *input);
    assert_eq!(body.body.original_words()[0].try_text().unwrap(), "method");
    assert_eq!(body.source.try_text().unwrap(), "set local $arg");
    assert!(
        !original
            .entry
            .owns_original_context(&original.snapshot.state.source_variables)
    );
    assert!(!original.entry.owns_frame(bindings, original.entry.frame()));
    assert!(original.entry.original_formal_topology().is_none());
    assert!(super::original_declaration_layouts(rows).is_none());
    assert!(binding.original_recorded_command().is_none());
    let frame = bindings
        .logical_source_member_frame_at_offset(input, &image, cursor, input.lexer_config())
        .expect("actual source-only receiver frame");
    assert!(matches!(
        frame.frame().layout(),
        tcl_names::variable::VariableExecutionFrame::ReceiverMethod { .. }
    ));
    assert_eq!(frame.source().try_text().unwrap(), "set local $arg");
    assert!(
        bindings
            .original_variable_frame_at_offset(cursor, input.lexer_config())
            .is_none()
    );
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        parent,
    ) = body.body.original_parent_body().source_words().source()
    else {
        panic!("actual source parent declaration");
    };
    let mut missing = bindings.clone();
    assert!(missing.declaration_layouts.remove(parent.site()).is_some());
    assert!(
        missing
            .logical_source_member_frame_at_offset(input, &image, cursor, input.lexer_config(),)
            .is_none()
    );
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    assert!(
        bindings
            .logical_source_member_frame_at_offset(input, &image, cursor, changed_config,)
            .is_none()
    );
    let changed_image = tcl_lexer::SourceImage::document(&source.replace("local", "other"));
    assert!(
        bindings
            .logical_source_member_frame_at_offset(
                input,
                &changed_image,
                cursor,
                input.lexer_config(),
            )
            .is_none()
    );
}

fn original_tokens(
    source: &str,
    body: &str,
    offset: usize,
) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
    original_tokens_in("tcl8.6", source, body, offset)
}

fn original_tokens_in(
    profile: &str,
    source: &str,
    body: &str,
    offset: usize,
) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
    original_tokens_in_image(
        profile,
        &tcl_lexer::SourceImage::document(source),
        body,
        offset,
    )
}

fn original_tokens_in_image(
    profile: &str,
    image: &tcl_lexer::SourceImage,
    body: &str,
    offset: usize,
) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
    );
    original_tokens_in_image_with_config(
        profile,
        image,
        body,
        offset,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
}

fn original_tokens_in_image_with_config(
    profile: &str,
    image: &tcl_lexer::SourceImage,
    body: &str,
    offset: usize,
    config: tcl_lexer::LexerConfig,
) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
    let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
    );
    let bindings = super::SourceCommandBindings::analyse_image_in_frame_with_options(
        image,
        &tcl_names::variable::VariableExecutionFrame::Global,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..super::super::SourceAnalysisOptions::default()
        },
    )
    .unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        body,
        u32::try_from(offset).unwrap(),
        config,
    )
    .remove(0);
    let mut tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    (bindings, tokens)
}

fn issuer_guard_summary(observations: &[super::DeclarationLayoutObservation]) -> Vec<[bool; 7]> {
    let original = observations.iter().find(|observation| {
        observation.issuer == super::DeclarationLayoutIssuer::OriginalDeclaration
    });
    observations
        .iter()
        .map(|observation| {
            [
                observation.issuer == super::DeclarationLayoutIssuer::OriginalDeclaration,
                original.is_some_and(|first| observation.entry == first.entry),
                original.is_some_and(|first| observation.config == first.config),
                original.is_some_and(|first| observation.words == first.words),
                observation.snapshot.state.variable_frame == *observation.entry.frame(),
                observation.snapshot.state.current_source_origin.as_ref()
                    == Some(&observation.entry.source().origin),
                observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables),
            ]
        })
        .collect()
}

#[test]
fn original_variable_cursor_at_body_end_keeps_occurrence_membership_half_open() {
    // Implementation contract: naming.variable.original-body-cursor-frame
    // docs/design/analysis/name-resolution-proofs/original-body-cursor-frame.md
    let source = concat!(
        "set shared 1\n",
        "proc first {argument} {set local 1; global shared; upvar #0 shared link; puts $}\n",
        "proc second {argument} {set foreign 1; puts $}\n",
    );
    let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    analysis.global_scope.variables.clear();
    analysis.all_procs.clear();
    let first_end = u32::try_from(source.find("puts $").unwrap() + "puts $".len()).unwrap();
    let second_end = u32::try_from(source.rfind("puts $").unwrap() + "puts $".len()).unwrap();
    let frame = |offset| analysis.original_variable_frame_in_source(&image, config, offset);
    let first = frame(first_end).expect("the cursor precedes the first authentic body closer");
    let second = frame(second_end).expect("the cursor precedes the second authentic body closer");
    assert_eq!(frame(first_end - 1), Some(first.clone()));
    assert_eq!(frame(second_end - 1), Some(second.clone()));
    assert_ne!(first, second);
    assert_eq!(frame(first_end + 1), None);
    assert_eq!(frame(second_end + 1), None);
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let origin = bindings.root_origin.as_ref().unwrap();
    let entry = bindings
        .declaration_layouts
        .values()
        .find_map(|rows| {
            super::original_declaration_layouts(rows)?.find_map(|row| {
                (row.entry.frame() == first.frame() && row.entry.source() == first.source())
                    .then(|| row.entry.clone())
            })
        })
        .expect("the frame has its independent original declaration entry");
    assert!(entry.owns_source(origin, first_end - 1));
    assert!(!entry.owns_source(origin, first_end));
    assert!(entry.owns_source_cursor(origin, first_end));
    assert!(!entry.owns_source_cursor(origin, first_end + 1));
    assert!(
        analysis
            .original_variable_frame_in_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                config,
                first_end,
            )
            .is_none()
    );
    assert!(
        analysis
            .original_variable_frame_in_source(
                &image,
                tcl_lexer::LexerConfig {
                    expand_syntax: !config.expand_syntax,
                    ..config
                },
                first_end,
            )
            .is_none()
    );
}

#[test]
fn declaration_layout_issuer_preserves_two_actual_caller_activations() {
    let body = "expr {$n + 1}";
    let source = "proc p {n} {expr {$n + 1}}; p 1; p 2";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let (bindings, tokens) =
            original_tokens_in(dialect, source, body, source.find("expr").unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let unchanged = binding.clone();
        let observations = binding.declaration_layout_observations.as_deref().unwrap();
        let originals = super::original_declaration_layouts(observations).unwrap_or_else(|| {
            panic!(
                "{dialect}: original issuer/entry/config/words/frame/source/context: {:?}",
                issuer_guard_summary(observations)
            )
        });
        let original = originals.clone().next().unwrap();
        let entered = observations
            .iter()
            .filter(|observation| {
                observation.issuer == super::DeclarationLayoutIssuer::EnteredActivation
            })
            .map(|observation| observation.snapshot.state.variable_frame.clone())
            .collect::<std::collections::HashSet<_>>();
        assert!(entered.len() >= 2, "{dialect}: distinct actual calls");
        assert!(
            originals
                .clone()
                .all(|observation| observation.entry == original.entry)
        );
        let advice = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("original layout survives actual-frame joins");
        assert_eq!(advice.frame(), original.entry.frame(), "{dialect}");
        assert!(advice.closed_lookup(), "{dialect}");
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        assert!(
            binding.declaration_flow_report(registry).is_some(),
            "{dialect}"
        );
        assert_eq!(binding, &unchanged, "advice never edits actual receipts");
        let site = binding.invocation_site().unwrap();
        assert!(bindings.declaration_layouts[site].len() > entered.len());
    }
}

#[test]
fn declaration_layout_issuer_requires_original_source_and_unanimity() {
    use std::sync::Arc;
    let source = "proc p {n} {expr {$n + 1}}; p 1; p 2";
    let (_, tokens) = original_tokens(source, "expr {$n + 1}", source.find("expr").unwrap());
    let binding = tokens.source_binding.as_ref().unwrap();
    let observations = binding.declaration_layout_observations.as_deref().unwrap();
    let original = super::original_declaration_layouts(observations)
        .unwrap_or_else(|| {
            panic!(
                "original issuer/entry/config/words/frame/source/context: {:?}",
                issuer_guard_summary(observations)
            )
        })
        .next()
        .unwrap();
    let entered = observations
        .iter()
        .find(|observation| observation.issuer == super::DeclarationLayoutIssuer::EnteredActivation)
        .unwrap();
    let with_observations = |observations| {
        let mut changed = binding.clone();
        changed.declaration_layout_observations = Some(Arc::from(observations));
        changed
    };
    let missing = with_observations(vec![entered.clone()]);
    assert!(missing.declaration_operand_layout_advice(&tokens).is_none());
    let mut wrong_frame = entered.clone();
    wrong_frame.issuer = super::DeclarationLayoutIssuer::OriginalDeclaration;
    let wrong = with_observations(vec![original.clone(), wrong_frame]);
    assert!(wrong.declaration_operand_layout_advice(&tokens).is_none());

    let mut unknown = original.clone();
    let mut state = unknown.snapshot.state.clone();
    state.mark_opaque_binding_mutation();
    unknown.snapshot = Arc::new(super::SourceLookupSnapshot::in_realm(
        state,
        unknown.snapshot.realm,
    ));
    let ambiguous = with_observations(vec![original.clone(), unknown]);
    assert!(
        ambiguous
            .declaration_operand_layout_advice(&tokens)
            .is_none()
    );

    // Equal authored images intentionally share semantic source identity.
    // A different original input channel supplies genuinely foreign
    // provenance even when its bytes and command geometry are identical.
    let (_, foreign_tokens) = original_tokens_in_image(
        "tcl8.6",
        &tcl_lexer::SourceImage::native(source.as_bytes()),
        "expr {$n + 1}",
        source.find("expr").unwrap(),
    );
    let foreign = super::original_declaration_layouts(
        foreign_tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_layout_observations
            .as_deref()
            .unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let foreign = with_observations(vec![original.clone(), foreign.clone()]);
    assert!(foreign.declaration_operand_layout_advice(&tokens).is_none());

    let mut changed_tokens = tokens.clone();
    changed_tokens.word_exprs[1] = tcl_source::words::WordExpr::BracedLiteral {
        text: "$n + 99".into(),
        source: tokens.words()[1].source().clone(),
    };
    assert!(
        binding
            .declaration_operand_layout_advice(&changed_tokens)
            .is_none()
    );
}

#[test]
fn repeated_declarations_keep_original_compilers_separate_from_body_allocations() {
    let body = "expr {$n + 1}";
    let source = "foreach generation {A B} {proc p {n} {expr {$n + 1}}}; p 1; p 2";
    let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
    let binding = tokens.source_binding.as_ref().unwrap();
    let advice = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("same original lexical declaration can retain operand geometry across repetitions");
    assert_ne!(advice.targets(), []);
    let original = binding
        .admitted_inline_invocation()
        .expect("unchanged original expression compiler registration");
    assert!(original.target.registry_backed);
    assert!(original.target.implementation_allocation.is_none());
    assert_eq!(
        original.operation,
        tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    );
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation: original.operation,
            guard: tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments,
        }
    );
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .declaration_operand_layout_advice(&changed)
            .is_none()
    );
}

#[test]
fn replaced_declaration_handler_withdraws_original_layout_advice() {
    let body = "expr {$n + 1}";
    let source = "proc expr args {return CUSTOM}; proc p {n} {expr {$n + 1}}; p 1; p 2";
    let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.declaration_operand_layout_advice(&tokens).is_none());
    let replacement = binding
        .proved_execution_target()
        .expect("known replacement procedure");
    assert!(!replacement.registry_backed);
    assert!(replacement.matches_authored_implementation(source, 0));
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Generic
    );
}

#[test]
fn original_lookup_completeness_keeps_unknown_absent_and_namespace_alternatives() {
    use tcl_names::variable::VariableExecutionFrame;
    let source = "proc p {} {expr {3.5}}";
    let (_, tokens) = original_tokens(source, "expr {3.5}", source.find("expr").unwrap());
    let binding = tokens.source_binding.as_ref().unwrap();
    let mut advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
    assert!(advice.closed_lookup());
    advice.unknown = true;
    assert!(!advice.closed_lookup());
    assert!(
        !advice.targets().is_empty(),
        "May candidates remain available"
    );
    advice.unknown = false;
    advice.may_be_absent = true;
    assert!(!advice.closed_lookup());
    advice.may_be_absent = false;
    let original = advice.snapshot.clone();
    for frame in [
        VariableExecutionFrame::ReceiverMethod {
            identity: "receiver-preview".into(),
        },
        VariableExecutionFrame::Selected {
            selector: tcl_registry::FrameLevel::Relative(1),
            namespace: None,
        },
        VariableExecutionFrame::Unknown,
    ] {
        let mut state = original.state.clone();
        state.variable_frame = frame;
        advice.snapshot = std::sync::Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            original.realm,
        ));
        assert!(
            !advice.closed_lookup(),
            "declaration namespace cannot supply an unknown activation namespace"
        );
        assert_ne!(advice.targets(), []);
    }
    let mut state = original.state.clone();
    state.variable_frame = VariableExecutionFrame::NamespaceIdentity {
        namespace: tcl_names::namespace::SourceNamespaceKey::Authored("::other".into()),
        frame: Box::new(state.variable_frame),
    };
    advice.snapshot = std::sync::Arc::new(super::super::SourceLookupSnapshot::in_realm(
        state,
        original.realm,
    ));
    assert!(
        !advice.closed_lookup(),
        "foreign typed namespace cannot complete the original lookup"
    );
    advice.snapshot = original;
    assert!(advice.closed_lookup());
}

#[test]
fn declaration_variable_operands_preserve_decoded_name_and_decline_aliases() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Alias exclusion requires complete original body ownership, not an
    // installed alias, entered variable frame or successful read.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, body, expected) in [
        (
            "proc p {} {set l {a b c}; lset l $j $v}",
            "lset l $j $v",
            Some("l"),
        ),
        ("proc p {} {incr {$n}}", "incr {$n}", Some("$n")),
        ("proc p {} {global l; lset l $j $v}", "lset l $j $v", None),
        ("proc p {l} {lset l $j $v}", "lset l $j $v", None),
        ("proc p {} {lset a(k) $j $v}", "lset a(k) $j $v", None),
    ] {
        let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let operand =
            binding.declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
        assert_eq!(
            operand
                .as_ref()
                .map(super::DeclarationVariableOperandAdvice::name),
            expected,
            "{source}"
        );
        let mut changed = tokens.words()[1].clone();
        match &mut changed {
            tcl_source::words::WordExpr::Literal { text, .. }
            | tcl_source::words::WordExpr::BracedLiteral { text, .. } => *text = "different".into(),
            _ => panic!("fixed literal operand"),
        }
        assert!(
            binding
                .declaration_variable_operand_advice(registry, &tokens, &changed)
                .is_none()
        );
    }
}

#[test]
fn declaration_variable_operands_use_the_original_selected_word_configuration() {
    let name = concat!("local", "\\", "\n", "name");
    let source = format!("proc p {{}} {{incr {{{name}}}}}");
    let body = format!("incr {{{name}}}");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
    );
    for brace in [
        tcl_dialect::BraceBackslashNewline::Folds,
        tcl_dialect::BraceBackslashNewline::Literal,
    ] {
        let mut config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        config.brace_backslash_newline = brace;
        config.strict_quoting = true;
        let (bindings, tokens) = original_tokens_in_image_with_config(
            "tcl8.6",
            &tcl_lexer::SourceImage::document(&source),
            &body,
            source.find(&body).unwrap(),
            config,
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_source_operand_layout_advice(registry, &tokens)
            .unwrap();
        let original_layout = binding.declaration_operand_layout_advice(&tokens);
        let original_operand =
            binding.declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
        match brace {
            // A fold only rewrites a separator, so the body keeps its
            // authored coordinates and its Native operand reads the folded
            // name (#2505).
            tcl_dialect::BraceBackslashNewline::Folds => {
                assert!(original_layout.is_some());
                assert_eq!(
                    original_operand
                        .as_ref()
                        .map(super::DeclarationVariableOperandAdvice::name),
                    Some("local name")
                );
            }
            tcl_dialect::BraceBackslashNewline::Literal => {
                assert!(original_layout.is_some());
                assert_eq!(
                    original_operand
                        .as_ref()
                        .map(super::DeclarationVariableOperandAdvice::name),
                    Some(name)
                );
            }
        }
        assert_eq!(layout.config, config);
        assert_eq!(layout.dialect().word_values.brace, brace);
        let operand = binding
            .declaration_source_variable_operand_advice(registry, &tokens, &tokens.words()[1])
            .expect("genuine original scalar operand under its selected configuration");
        let expected = match brace {
            tcl_dialect::BraceBackslashNewline::Folds => "local name",
            tcl_dialect::BraceBackslashNewline::Literal => name,
        };
        assert_eq!(operand.name(), expected, "{brace:?}");
        assert!(
            crate::registry_invocation::declaration_increment_advice(registry, &tokens).is_some()
        );
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "other".into();
        assert!(
            binding
                .declaration_source_variable_operand_advice(registry, &changed, &tokens.words()[1])
                .is_none()
        );
        drop(bindings);
    }
}

fn assert_mapped_source_cache_conflict_refused(
    advice: &super::DeclarationSourceOperandLayoutAdvice,
    tokens: &crate::ir::CommandTokens,
    binding: &crate::command_binding::SourceInvocationBinding,
    registry: &tcl_registry::CommandRegistry,
) {
    let super::DeclarationSourceOperandLayoutKind::Configured { layout, cache, .. } = &advice.kind
    else {
        panic!("distinct source-only mapped carrier");
    };
    cache
        .upgrade()
        .unwrap()
        .entries
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(layout.site.offset, None);
    assert!(advice.current_advice(tokens).is_none());
    assert!(
        binding
            .declaration_source_operand_layout_advice(registry, tokens)
            .is_none()
    );
}

#[test]
fn declaration_mapped_source_operands_keep_full_vector_owner_and_conflict_refusals() {
    let name = concat!("local", "\\", "\n", "name");
    // The `${…}` comment keeps the folded body a mapped source.
    let source = format!("proc mapped {{}} {{incr {{{name}}};#${{kept}}}}");
    let body = format!("incr {{{name}}}");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let (bindings, tokens) = original_tokens_in_image_with_config(
        "tcl8.6",
        &tcl_lexer::SourceImage::document(&source),
        &body,
        source.find(&body).unwrap(),
        config,
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    let advice = binding
        .declaration_source_operand_layout_advice(registry, &tokens)
        .expect("actual mapped original body source vector");
    assert!(advice.current_advice(&tokens).is_some());
    assert!(advice.current_metadata_context(registry, &tokens).is_some());
    assert!(
        advice
            .current_advice(&tokens)
            .unwrap()
            .original_declaration_metadata_context(registry, &tokens)
            .is_err(),
        "the old Native declaration facade still requires its own invocation owner"
    );
    assert!(binding.declaration_operand_layout_advice(&tokens).is_none());

    let mut withdrawn = bindings.clone();
    withdrawn.declaration_layouts.clear();
    withdrawn.retain_original_declaration_source_operand_layouts(registry);
    let mut restamped = tokens.clone();
    withdrawn.stamp_original_tokens(&mut restamped);
    assert!(advice.current_advice(&restamped).is_none());
    assert!(
        restamped
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_source_operand_layout_advice(registry, &restamped)
            .is_none()
    );
    // Joining source alternatives must retain the attachment only when
    // both operands have the same actual original inventory receipt.
    for (left, right) in [(&tokens, &restamped), (&restamped, &tokens)] {
        let mut joined = left.clone();
        joined
            .source_binding
            .as_mut()
            .unwrap()
            .join(right.source_binding.as_ref().unwrap());
        assert!(advice.current_advice(&joined).is_none());
        assert!(
            joined
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_source_operand_layout_advice(registry, &joined)
                .is_none()
        );
    }
    let mut equal_join = tokens.clone();
    equal_join.source_binding.as_mut().unwrap().join(binding);
    assert!(advice.current_advice(&equal_join).is_some());
    // The retained original tokens still describe their actual old immutable
    // source analysis, independently of the modified clone's inventory.
    assert!(advice.current_advice(&tokens).is_some());

    let mut changed_world = bindings.clone();
    std::sync::Arc::make_mut(&mut changed_world.final_state).mark_opaque_binding_mutation();
    let mut changed_world_tokens = tokens.clone();
    changed_world.stamp_original_tokens(&mut changed_world_tokens);
    assert!(advice.current_advice(&changed_world_tokens).is_none());

    let mut lost_owner = tokens.clone();
    lost_owner
        .source_binding
        .as_mut()
        .unwrap()
        .original_analysis_owner = None;
    assert!(advice.current_advice(&lost_owner).is_none());
    let mut lost_word = tokens.clone();
    lost_word.argv_texts[1] = "other".into();
    assert!(advice.current_advice(&lost_word).is_none());
    let foreign = tcl_registry::model::ingress::static_context_for("jimtcl").commands();
    assert!(
        binding
            .declaration_source_operand_layout_advice(foreign, &tokens)
            .is_none()
    );

    // A genuine new analysis of identical source and a different complete
    // configuration cannot donate its binding to this retained receipt.
    let mut literal = config;
    literal.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Literal;
    literal.strict_quoting = !config.strict_quoting;
    let (_other_owner, other_tokens) = original_tokens_in_image_with_config(
        "tcl8.6",
        &tcl_lexer::SourceImage::document(&source),
        &body,
        source.find(&body).unwrap(),
        literal,
    );
    assert!(advice.current_advice(&other_tokens).is_none());

    assert_mapped_source_cache_conflict_refused(&advice, &tokens, binding, registry);
    drop(bindings);
}

#[test]
fn declaration_configured_scalar_operands_keep_existing_receiver_and_scope_refusals() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut config = tcl_lexer::LexerConfig::from_grammar(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap().grammar,
    );
    config.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Literal;
    for (source, body, expected) in [
        ("proc p {} {incr {$n}}", "incr {$n}", Some("$n")),
        ("proc p {} {incr {π}}", "incr {π}", Some("π")),
        ("proc p {} {incr {arr(key)}}", "incr {arr(key)}", None),
        (
            r"proc p {} {incr local\x3a\x3aname}",
            r"incr local\x3a\x3aname",
            None,
        ),
        ("proc p {l} {incr l}", "incr l", None),
        ("proc p {} {global l; incr l}", "incr l", None),
        ("proc p {} {upvar #0 original l; incr l}", "incr l", None),
        (
            "proc p {} {trace add variable l write callback; incr l}",
            "incr l",
            None,
        ),
    ] {
        let (_, tokens) = original_tokens_in_image_with_config(
            "tcl8.6",
            &tcl_lexer::SourceImage::document(source),
            body,
            source.find(body).unwrap(),
            config,
        );
        let operand = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
        assert_eq!(
            operand
                .as_ref()
                .map(super::DeclarationVariableOperandAdvice::name),
            expected,
            "{source}"
        );
    }
}

fn assert_scalar_original_layout_kind(
    brace: tcl_dialect::BraceBackslashNewline,
    source_layout: &super::DeclarationSourceOperandLayoutAdvice,
    original_operand: Option<&super::DeclarationVariableOperandAdvice>,
    expected: Option<&str>,
    source: &str,
) {
    match brace {
        tcl_dialect::BraceBackslashNewline::Folds => {
            assert!(matches!(
                source_layout.kind,
                super::DeclarationSourceOperandLayoutKind::Configured { .. }
            ));
            assert!(original_operand.is_none());
        }
        tcl_dialect::BraceBackslashNewline::Literal => {
            // No body transform occurred: its genuine original
            // owner keeps the ordinary trace-name veto, just as in
            // the unchanged-body controls above.
            assert!(matches!(
                source_layout.kind,
                super::DeclarationSourceOperandLayoutKind::Original(_)
            ));
            assert_eq!(
                original_operand.map(super::DeclarationVariableOperandAdvice::name),
                expected,
                "unchanged original body: {source}"
            );
        }
    }
}

#[test]
fn declaration_scalar_trace_exclusions_retain_other_names_and_mapped_source_configurations() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        (
            "proc trace_local {} {trace add variable l write callback; incr l}",
            None,
        ),
        (
            "proc trace_other {} {trace add variable other write callback; incr l}",
            Some("l"),
        ),
        (
            "proc trace_dynamic {} {trace add variable $target write callback; incr l}",
            None,
        ),
    ] {
        let (_owner, tokens) = original_tokens(source, "incr l", source.rfind("incr l").unwrap());
        let advice = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
        assert_eq!(
            advice
                .as_ref()
                .map(super::DeclarationVariableOperandAdvice::name),
            expected,
            "{source}"
        );
    }
    let name = concat!("local", "\\", "\n", "name");
    for same_name in [false, true] {
        let target = if same_name {
            format!("{{{name}}}")
        } else {
            "other".to_owned()
        };
        let body = format!("trace add variable {target} write callback; incr {{{name}}}");
        // The `${…}` comment keeps the folded body a mapped source.
        let source = format!("proc trace_mapped {{}} {{{body};#${{kept}}}}");
        let command = format!("incr {{{name}}}");
        for brace in [
            tcl_dialect::BraceBackslashNewline::Folds,
            tcl_dialect::BraceBackslashNewline::Literal,
        ] {
            let mut config =
                tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            config.brace_backslash_newline = brace;
            config.strict_quoting = true;
            let (_owner, tokens) = original_tokens_in_image_with_config(
                "tcl8.6",
                &tcl_lexer::SourceImage::document(&source),
                &command,
                source.rfind(&command).unwrap(),
                config,
            );
            let binding = tokens.source_binding.as_ref().unwrap();
            let advice = binding.declaration_source_variable_operand_advice(
                registry,
                &tokens,
                &tokens.words()[1],
            );
            let expected = if same_name {
                None
            } else {
                Some(match brace {
                    tcl_dialect::BraceBackslashNewline::Folds => "local name",
                    tcl_dialect::BraceBackslashNewline::Literal => name,
                })
            };
            assert_eq!(
                advice
                    .as_ref()
                    .map(super::DeclarationSourceVariableOperandAdvice::name),
                expected,
                "{brace:?}: {source}"
            );
            let source_layout = binding
                .declaration_source_operand_layout_advice(registry, &tokens)
                .expect("actual selected source owner");
            let original_operand =
                binding.declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
            assert_scalar_original_layout_kind(
                brace,
                &source_layout,
                original_operand.as_ref(),
                expected,
                &source,
            );
        }
    }
}

#[test]
fn mapped_source_continuation_keeps_original_prefix_and_observer_debt() {
    let name = concat!("local", "\\", "\n", "name");
    let body = format!("trace add variable other write callback; incr {{{name}}}");
    // The `${…}` comment keeps the folded body a mapped source.
    let source = format!("proc source_prefix {{}} {{{body};#${{kept}}}}");
    let command = format!("incr {{{name}}}");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let (_owner, tokens) = original_tokens_in_image_with_config(
        "tcl8.6",
        &tcl_lexer::SourceImage::document(&source),
        &command,
        source.rfind(&command).unwrap(),
        config,
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    let advice = binding
        .declaration_source_operand_layout_advice(registry, &tokens)
        .expect("actual conditional source continuation");
    assert!(
        advice.current_advice(&tokens).is_none(),
        "no unchanged Native lookup world follows a result registration"
    );
    assert!(advice.current_conditional_source_advice(&tokens).is_some());
    let super::DeclarationSourceOperandLayoutKind::Configured { layout, .. } = &advice.kind else {
        panic!("mapped readonly source purpose");
    };
    assert_eq!(layout.source_operations.len(), 1);
    let operation = &layout.source_operations[0].operation;
    assert!(operation.obligations().contains(
        &tcl_registry::source_operation::SourceOperationObligation::ObserverApplicability
    ));
    assert!(
        operation
            .effects()
            .accesses()
            .iter()
            .any(|access| access.mode != tcl_registry::EffectAccessMode::Read)
    );
    let operand = binding
        .declaration_source_variable_operand_advice(registry, &tokens, &tokens.words()[1])
        .unwrap();
    assert_eq!(operand.name(), "local name");
    assert_eq!(operand.source_operations, layout.source_operations);
    assert!(
        binding
            .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1])
            .is_none()
    );
}

#[test]
fn original_destruction_operand_keys_follow_the_retained_alias_point() {
    // Implementation contract: naming.variable.original-destruction-operand-keys
    // docs/design/analysis/name-resolution-proofs/original-destruction-operand-keys.md
    let source = "proc f {} {set original 1; upvar 0 original linked; unset linked}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let (_, tokens) = original_tokens_in(
            dialect,
            source,
            "unset linked",
            source.find("unset").unwrap(),
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let keys = binding
            .declaration_variable_argument_keys(&tokens, &[0], registry)
            .unwrap_or_else(|| panic!("{dialect}: original destruction operand"));
        let [tcl_names::variable::VariableCellKey::Activation { simple, .. }] = keys.as_slice()
        else {
            panic!("{dialect}: independently owned activation key");
        };
        assert_eq!(
            simple.as_bytes(),
            b"original",
            "{dialect}: followed original alias, not reporting linked"
        );
        assert!(
            binding
                .declaration_variable_argument_keys(&tokens, &[1], registry)
                .is_none()
        );
    }
}

#[test]
fn declaration_read_occurrences_keep_missing_reads_without_physical_ssa() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Alias exclusion requires complete original body ownership, not an
    // installed alias, entered variable frame or successful read.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc p {} {namespace upvar ::ns a alias; return $missing}";
    let (_, tokens) = original_tokens(source, "return $missing", source.find("return").unwrap());
    let reads = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_read_occurrences(registry, &tokens)
        .expect("original declaration read inventory");
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].name(), "missing");
    assert_eq!(reads[0].spelling(), "$missing");
    assert_eq!(
        reads[0].source(),
        tokens.words()[1].sole_variable_substitution().unwrap().1
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .execution_is_unknown()
    );
    let mut changed = tokens.clone();
    changed.variable_accesses[0].original_spelling = "$different".to_owned();
    assert_eq!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_read_occurrences(registry, &changed)
            .unwrap(),
        [] as [crate::command_binding::declaration_layout::DeclarationReadOccurrenceAdvice; 0]
    );
    for source in [
        "proc p {missing} {return $missing}",
        "proc p {} {global missing; return $missing}",
        "proc p {} {namespace upvar ::ns a missing; return $missing}",
    ] {
        let (_, tokens) =
            original_tokens(source, "return $missing", source.find("return").unwrap());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_read_occurrences(registry, &tokens)
                .unwrap()
                .is_empty(),
            "{source}"
        );
    }
    let replaced = "proc return args {}; proc p {} {return $missing}";
    let (_, tokens) = original_tokens(
        replaced,
        "return $missing",
        replaced.rfind("return").unwrap(),
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_read_occurrences(registry, &tokens)
            .is_none()
    );
}

#[test]
fn conditional_index_layout_keeps_native_compiler_capture_independent() {
    // Native original pair: C8.6 compiles lsetList before the unknown argv
    // substitution; Jim retains its generic script dispatch. Both bodies
    // fail at the unknown child, without entering the outer lset handler.
    // Evidence: .proofs/2286-command-declaration-graph/native-lset-pair.json.
    let source = "proc p {j v} {lset l $j [unknown]}";
    for (profile, normal) in [("jim", false), ("tcl8.6", true)] {
        let (bindings, tokens) = original_tokens_in(
            profile,
            source,
            "lset l $j [unknown]",
            source.find("lset").unwrap(),
        );
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let advice =
            crate::registry_invocation::conditional_index_access_advice(registry, None, &tokens)
                .unwrap_or_else(|| {
                    panic!(
                        "{profile}: original single-index layout is conditional diagnostic advice"
                    )
                });
        assert_eq!(
            advice.kind,
            crate::registry_invocation::NormalIndexAccessKind::ListWrite
        );
        assert_eq!(advice.container, "l");
        assert_eq!(advice.index, "$j");
        assert!(
            bindings
                .declaration_operand_layout_advice(&tokens)
                .is_some()
        );
        assert_eq!(
            crate::registry_invocation::normal_representation_invocation(registry, None, &tokens)
                .is_some(),
            normal,
            "{profile}"
        );
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let replaced = "proc lset args {}; proc p {j v} {lset l $j $v}";
    let (_, tokens) = original_tokens(replaced, "lset l $j $v", replaced.rfind("lset").unwrap());
    assert!(
        crate::registry_invocation::conditional_index_access_advice(registry, None, &tokens)
            .is_none()
    );
}

#[test]
fn source_callee_layout_is_distinct_from_registry_metadata() {
    let source = "proc run {cmd} {$cmd 5}; proc d {} {run notacommand}";
    let (bindings, tokens) =
        original_tokens(source, "run notacommand", source.rfind("run").unwrap());
    assert!(
        bindings
            .declaration_operand_layout_advice(&tokens)
            .is_none()
    );
    let call = bindings
        .declaration_call_layout_advice(&tokens)
        .expect("original source-backed call layout");
    let [callee] = call.targets() else {
        panic!("one original source declaration");
    };
    assert!(!callee.registry_backed);
    assert_eq!(callee.kind, tcl_names::command::BindingKind::Proc);
    assert!(callee.implementation_allocation.is_some());
    let mut changed = tokens;
    changed.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
    assert!(bindings.declaration_call_layout_advice(&changed).is_none());
    let source = "proc d {} {notacommand VALUE}";
    let (bindings, tokens) = original_tokens(
        source,
        "notacommand VALUE",
        source.find("notacommand").unwrap(),
    );
    assert!(bindings.declaration_call_layout_advice(&tokens).is_none());
}

#[test]
fn declaration_layout_keeps_its_original_table_without_a_compiler_visit() {
    let source = "proc p {mode} {switch -- $mode {a {puts A} default {puts D}}}";
    let body = "switch -- $mode {a {puts A} default {puts D}}";
    let (bindings, tokens) = original_tokens(source, body, source.find("switch").unwrap());
    crate::command_binding::trace_original_g4_owner("declaration-layout", &bindings, &tokens);
    let advice = bindings.declaration_operand_layout_advice(&tokens);
    if advice.is_none() {
        let binding = tokens.source_binding.as_ref().unwrap();
        eprintln!(
            "declaration layout: site={:?} frame={:?} observations={} entry={:?}",
            binding.invocation_site().map(|site| site.offset),
            binding.variable_frame,
            bindings.declaration_layouts.len(),
            binding
                .invocation_site()
                .and_then(|site| bindings.conditional_body_entry_at(&site.source, site.offset))
        );
        for (site, observations) in &bindings.declaration_layouts {
            for observation in observations {
                eprintln!(
                    "declaration observation: site={} origin_matches={} frame_matches={} owns_source={} owns_invocation={} dialect={:?} targets={:?}",
                    site.offset,
                    observation.snapshot.state.current_source_origin.as_ref() == Some(&site.source),
                    observation.snapshot.state.variable_frame == binding.variable_frame,
                    observation.entry.owns_source(&site.source, site.offset),
                    observation.entry.owns_invocation(binding),
                    observation
                        .snapshot
                        .state
                        .source_variables
                        .invocation_dialect,
                    observation
                        .snapshot
                        .state
                        .targets("switch", &observation.namespace)
                        .iter()
                        .map(|target| (&target.command, target.registry_backed))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    let advice = advice.expect("original declaration-owned operand layout");
    assert_ne!(advice.targets(), []);
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiler_lookup_state
            .is_none()
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(!binding.execution_is_unknown());
    assert_eq!(
        binding.runtime_reachability(),
        super::super::SourceRuntimeReachability::Reached
    );
    assert_eq!(
        binding
            .proved_execution_target()
            .map(|target| target.command.as_str()),
        Some("::switch")
    );
    assert!(bindings.final_state.baseline.native_entry.is_none());
    let mut changed = tokens.clone();
    changed.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
    assert!(
        bindings
            .declaration_operand_layout_advice(&changed)
            .is_none()
    );
    let source = "proc switch args {}; proc p {mode} {switch -- $mode {a {puts A}}}";
    let (bindings, tokens) = original_tokens(
        source,
        "switch -- $mode {a {puts A}}",
        source.rfind("switch").unwrap(),
    );
    assert!(
        bindings
            .declaration_operand_layout_advice(&tokens)
            .is_none()
    );
}
