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

//! Effective and registry words: alias composition, literals, compiled local names and dynamics.

use super::*;

#[test]
fn decoded_native_byte_words_preserve_cardinality_and_checked_text() {
    let bytes = EffectiveInvocationWord::from_bytes(&[0xff]);
    assert_eq!(bytes.literal_bytes(), Some([0xff].as_slice()));
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    assert_eq!(
        bytes.as_registry_word(),
        InvocationWord::KnownBytes(&[0xff])
    );
    assert_eq!(bytes.as_registry_word().literal(), None);
    assert!(bytes.as_registry_word().has_exactly_one_argv_entry());
    let unicode = EffectiveInvocationWord::from_bytes("ÿ".as_bytes());
    assert_eq!(unicode.as_registry_word(), InvocationWord::Literal("ÿ"));
    assert_ne!(bytes, unicode);
}

#[test]
fn static_command_word_decodes_only_original_static_values() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let rules = WordValueRules::from_config(&config);
    for (source, expected) in [
        (r"se\x74 x 1", Some("set")),
        (r"{se\x74} x 1", Some(r"se\x74")),
        ("\"set\" x 1", Some("set")),
        ("$head x 1", None),
        ("[gethead] x 1", None),
        ("{*}$head x 1", None),
    ] {
        let segment =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .pop()
                .expect("one invocation");
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        assert_eq!(
            static_command_word(&tokens, config.escapes, rules).as_deref(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn alias_composition_preserves_original_evaluation_and_inserted_origins() {
    let registry = CommandRegistry::build_default();
    let module = crate::lowering::lower_to_ir(
        "interp alias {} inner {} list second\ninterp alias {} outer {} inner first\nouter $value",
        &registry,
    );
    let tokens = module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let effective = effective_command_words(tokens).expect("alias has a unique target");
    assert_eq!(tokens.argv_texts, ["outer", "${value}"]);
    assert_eq!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap(),
        ["second", "first", "${value}"]
    );
    assert_eq!(
        effective.origins,
        [
            InvocationWordOrigin::ResolvedHead,
            InvocationWordOrigin::BindingPrefix(0),
            InvocationWordOrigin::BindingPrefix(1),
            InvocationWordOrigin::Written(1)
        ]
    );
    assert_eq!(effective.written_argument(0), None);
    assert_eq!(effective.written_argument(2), Some(0));
    assert!(matches!(
        effective.words.last(),
        Some(WordExpr::Variable { .. })
    ));
    assert!(matches!(
        effective.words[1].source().provenance,
        tcl_source::words::Provenance::Opaque
    ));
}

#[test]
fn document_target_composes_arguments_without_registry_semantics() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc custom {prefix value} {return $value}\ninterp alias {} invoke {} custom FIXED\ninvoke $value";
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("invoke", segment.span.start()));
    let effective =
        effective_command_words(&tokens).expect("unique document procedure through alias");
    assert_eq!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_target()
            .unwrap()
            .command,
        "::custom"
    );
    assert_eq!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap(),
        ["FIXED", "${value}"]
    );
    assert_eq!(effective.written_argument(0), None);
    assert_eq!(effective.written_argument(1), Some(0));
    assert!(
        resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none(),
        "composing a document callee is not registry implementation proof"
    );
}

#[test]
fn captured_unknown_prefix_preserves_count_without_literal_presentation() {
    let source = "alias written";
    let config = tcl_lexer::LexerConfig::default();
    let segment = segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    let effective = compose_effective_words(
        &tokens,
        "list",
        &[
            EffectiveInvocationWord::Dynamic,
            EffectiveInvocationWord::Literal("known".to_owned()),
        ],
    )
    .unwrap();
    assert_eq!(
        effective.argument_presentations(&tokens.argv_texts[1..]),
        Some(vec![
            None,
            Some("known".to_owned()),
            Some("written".to_owned())
        ])
    );
    assert!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .is_none()
    );
    let words = frozen_argument_words(&tokens, &effective);
    assert_eq!(words[0], EffectiveInvocationWord::Dynamic);
    assert_eq!(words.len(), 3);
}

#[test]
fn effective_argument_projection_declines_inconsistent_source_ownership() {
    let projection = EffectiveCommandWords {
        words: vec![literal("list"), literal("arg")],
        origins: vec![InvocationWordOrigin::Written(0)],
        binding_prefix: Vec::new(),
    };
    assert_eq!(projection.argument_spellings(&["arg".into()]), None);
    let projection = EffectiveCommandWords {
        words: vec![literal("list"), literal("arg")],
        origins: vec![
            InvocationWordOrigin::Written(0),
            InvocationWordOrigin::Written(1),
        ],
        binding_prefix: Vec::new(),
    };
    assert_eq!(projection.argument_spellings(&[]), None);
    assert_eq!(
        projection.argument_spellings(&["arg".into()]),
        Some(vec!["arg".into()])
    );
}

#[test]
fn literal_words_materialise_registry_transition_facts() {
    let words = [literal("global"), literal("::shared")];
    let resolution = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &words,
    )
    .expect("literal command has a registry outcome");

    let RegistryInvocationResolution::Resolved(facts) = resolution else {
        panic!("literal global must resolve");
    };
    let transitions = facts
        .state_transitions
        .declared()
        .expect("global declares transition facts");
    assert!(matches!(
        transitions.facts(),
        [fact]
            if matches!(
                &fact.transition,
                StateTransition::VariableCellAlias(alias)
                    if alias.local.literal() == Some("shared")
                        && alias.local.argument_index() == Some(0)
            )
    ));
}

#[test]
fn segmented_arguments_preserve_a_static_non_option_template_prefix() {
    let command = segment_commands("regexp \"abc$part.*\" $subject\n")
        .into_iter()
        .next()
        .expect("one command");
    assert_eq!(
        segmented_command_arguments(&command),
        vec![InvocationWord::DynamicNonOption, InvocationWord::Dynamic],
        "the first template remains dynamic but cannot be a leading option"
    );
}

#[test]
fn compiler_word_shape_preserves_quoted_simple_words_without_decoding_escapes() {
    use tcl_registry::native_compilation::NativeCompilationWordShape as Shape;

    for (source, expected) in [
        ("list plain", Shape::Literal),
        ("list \"plain\"", Shape::QuotedLiteral),
        ("list \"\"", Shape::QuotedLiteral),
        ("list {plain}", Shape::BracedLiteral),
        (r"list \p\lain", Shape::BackslashLiteral),
        (r#"list "\160lain""#, Shape::BackslashLiteral),
        ("list \"$value\"", Shape::Substituted),
    ] {
        let segment = segment_commands(source).pop().unwrap();
        let tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            tcl_lexer::LexerConfig::default(),
            &segment,
        );
        assert_eq!(
            native_compilation_word_shape(&tokens.words()[1]),
            expected,
            "{source}"
        );
    }
}

#[test]
fn effective_words_keep_braces_and_decode_bare_escapes() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    assert_eq!(
        effective_invocation_word(
            &WordExpr::Template {
                parts: vec![WordPart::Text {
                    text: r"\x32".to_owned(),
                    source: source.clone()
                }],
                source: source.clone(),
                rejected: None,
            },
            EscapeSyntax::Tcl86,
            WordValueRules::TCL,
        ),
        EffectiveInvocationWord::Literal("2".to_owned())
    );
    assert_eq!(
        effective_invocation_word(
            &WordExpr::BracedLiteral {
                text: r"\x32".to_owned(),
                source
            },
            EscapeSyntax::Tcl86,
            WordValueRules::TCL,
        ),
        EffectiveInvocationWord::Literal(r"\x32".to_owned())
    );
}

#[test]
fn compound_array_word_facts_require_the_original_configured_operand() {
    let policy =
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let config = tcl_lexer::LexerConfig::default();
    for (source, expected) in [
        ("set array($index) VALUE", Some("array")),
        ("set \"array($index)\" VALUE", Some("array")),
        (r"set array\($index) VALUE", None),
        ("set $root($index) VALUE", None),
        ("set array($index)suffix VALUE", None),
    ] {
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let original = &plan.commands[0].words[1];
        assert_eq!(
            original_array_element_word(original, policy),
            expected.map(|root| EffectiveInvocationWord::ArrayElementName {
                root: root.to_owned()
            }),
            "{source}"
        );
        let segment = segment_commands(source).pop().unwrap();
        let tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            &segment,
        );
        assert_eq!(
            effective_invocation_word(
                &tokens.words()[1],
                config.escapes,
                WordValueRules::from_config(&config)
            ),
            EffectiveInvocationWord::Dynamic,
            "a representative template has no original operand owner: {source}"
        );
    }
    let source = "set array($index) VALUE";
    let image = tcl_lexer::SourceImage::document(source);
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar,
    );
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    assert!(
        original_array_element_word(&plan.commands[0].words[1], policy).is_none(),
        "mismatching original grammar cannot borrow C8.6 operand semantics"
    );
}

#[test]
fn compiled_local_name_requires_a_direct_source_word() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    let raw_braced = WordExpr::BracedLiteral {
        text: r"p\x75b".to_owned(),
        source: source.clone(),
    };
    let bare_escape = WordExpr::Template {
        parts: vec![WordPart::Text {
            text: r"p\x75b".to_owned(),
            source: source.clone(),
        }],
        source: source.clone(),
        rejected: None,
    };
    let quoted_plain = WordExpr::Template {
        parts: vec![WordPart::Text {
            text: "pub".to_owned(),
            source: source.clone(),
        }],
        source: source.clone(),
        rejected: None,
    };
    let dynamic = WordExpr::Variable {
        spelling: "$name".to_owned(),
        source: source.clone(),
    };

    assert_eq!(
        compiled_local_name_word(&raw_braced),
        CompiledLocalNameWord::Direct,
        "braces preserve the backslash as part of the variable name"
    );
    assert_eq!(
        compiled_local_name_value(
            &WordExpr::BracedLiteral {
                text: "{zz}".to_owned(),
                source: source.clone(),
            },
            EscapeSyntax::Tcl90,
            WordValueRules::TCL,
        ),
        Some("{zz}".to_owned()),
        "the source owner removes only the grouping brace pair"
    );
    assert_eq!(
        compiled_local_name_word(&bare_escape),
        CompiledLocalNameWord::Stack,
        "bare escapes decode before variable lookup"
    );
    assert_eq!(
        compiled_local_name_word(&quoted_plain),
        CompiledLocalNameWord::Direct,
        "quotes group a direct name without backslash processing"
    );
    assert_eq!(
        compiled_local_name_word(&dynamic),
        CompiledLocalNameWord::Stack
    );
}

#[test]
fn compiled_local_name_collapses_only_braced_continuations() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    let continuation = WordExpr::BracedLiteral {
        text: "p\\\n  ub".to_owned(),
        source: source.clone(),
    };
    let raw_escape = WordExpr::BracedLiteral {
        text: r"p\x75b".to_owned(),
        source,
    };

    assert_eq!(
        compiled_local_name_value(&continuation, EscapeSyntax::Tcl90, WordValueRules::TCL),
        Some("p ub".to_owned())
    );
    assert_eq!(
        compiled_local_name_value(&raw_escape, EscapeSyntax::Tcl90, WordValueRules::TCL),
        Some(r"p\x75b".to_owned())
    );
}

#[test]
fn dynamic_and_expanded_words_never_become_literal_registry_values() {
    let dynamic = [
        literal("global"),
        WordExpr::Variable {
            spelling: "$name".to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        },
    ];
    let RegistryInvocationResolution::Resolved(dynamic_facts) = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &dynamic,
    )
    .expect("dynamic global has a registry outcome") else {
        panic!("literal global head resolves");
    };
    assert!(
        dynamic_facts
            .state_transitions
            .declared()
            .expect("global declares transition facts")
            .facts()
            .iter()
            .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
    );

    let expanded = [
        literal("proc"),
        literal("p"),
        WordExpr::Expand {
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
            word: Box::new(WordExpr::Opaque {
                text: "args".to_owned(),
                source: SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
                reason: WordOpacity::LossySnapshot,
            }),
        },
        literal("body"),
    ];
    let RegistryInvocationResolution::Resolved(expanded_facts) = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &expanded,
    )
    .expect("expanded proc has a registry outcome") else {
        panic!("literal proc head resolves");
    };
    assert!(
        expanded_facts
            .state_transitions
            .declared()
            .expect("proc declares transition facts")
            .facts()
            .iter()
            .all(|fact| !matches!(fact.transition, StateTransition::CommandBinding(_)))
    );
}

#[test]
fn dynamic_head_stays_typed_unresolved() {
    let words = [WordExpr::Variable {
        spelling: "$command".to_owned(),
        source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
    }];
    assert!(matches!(
        resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &words,
        ),
        Ok(RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::ComputedHead {
                word_kind: InvocationWordKind::Dynamic,
            }
        ))
    ));
}
