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

//! The original source operands kept for a member: receiver declarations, method-name words, loop-installed names and vendor class declarations.

use super::*;

#[test]
fn original_receiver_declaration_keeps_its_whole_parent_bom_configuration() {
    // naming.tcloo.original-source-receiver-body-declaration
    // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
    // Source declaration body only; no allocated class or entered method.
    fn bodies(
        scope: &Scope,
    ) -> Vec<&super::super::super::types::OriginalSourceReceiverBodyDeclaration> {
        scope
            .original_receiver_body_declaration
            .iter()
            .map(AsRef::as_ref)
            .chain(scope.children.iter().flat_map(bodies))
            .collect()
    }
    let body =
        "interp create -safe s; interp eval s {oo::class create C {method m {} {source a.tcl}}}";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let source = format!("\u{feff}{body}");
    let result = Analyser::new().analyse(&source, "tcl9.1");
    let declarations = bodies(&result.global_scope);
    assert_eq!(declarations.len(), 1, "{:?}", result.diagnostics);
    assert!(declarations[0].matches_source(&tcl_lexer::SourceImage::document(&source), &input));
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        let result = Analyser::new()
            .with_resolved_input(supplied.clone())
            .analyse(body, "display-only");
        let declarations = bodies(&result.global_scope);
        assert_eq!(declarations.len(), 1, "{bom:?}: {:?}", result.diagnostics);
        let original = declarations[0];
        let image = tcl_lexer::SourceImage::document(body);
        assert!(original.matches_source(&image, &supplied));
        assert_eq!(original.body_word().config(), supplied.lexer_config());
        assert_eq!(original.parameters_word().config(), supplied.lexer_config());
        let commands = tcl_lexer::native_script_words_in(
            image.clone(),
            original.body_word().content_span().unwrap(),
            supplied.lexer_config(),
        )
        .unwrap();
        assert!(original.owns_invocation(&commands.commands[0].words, &supplied));
        assert!(!original.matches_source(
            &tcl_lexer::SourceImage::document(&(body.to_owned() + "\n")),
            &supplied
        ));
        let mut stale = supplied.clone();
        stale.config.strict_quoting = !stale.config.strict_quoting;
        assert!(!original.matches_source(&image, &stale));
        let mut foreign = supplied.clone();
        foreign.context = std::sync::Arc::new(
            supplied
                .context
                .with_command_store(std::sync::Arc::clone(supplied.context.commands())),
        );
        assert!(!original.matches_source(&image, &foreign));
    }
}

#[test]
fn logical_method_name_operands_require_selected_original_named_members() {
    let source = "oo::class create C {method {$literal} {} {}; self method {with space} {} {}; method known $formals {}; constructor {} {}; foreach n {generated} {method $n {} {}}; method {*}{expanded} {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let class = &analysis.all_classes["::C"];
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    for (side, method) in [
        (MemberSide::Instance, &class.methods["$literal"]),
        (MemberSide::Instance, &class.methods["known"]),
        (MemberSide::ClassObject, &class.class_methods["with space"]),
    ] {
        let word = class
            .original_members
            .logical_name_operand(side, method, input, image)
            .unwrap();
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_ascii_presentation(word).unwrap(),
            method.name.as_bytes(),
        );
        assert!(
            class
                .original_members
                .logical_name_operand(
                    match side {
                        MemberSide::Instance => MemberSide::ClassObject,
                        MemberSide::ClassObject => MemberSide::Instance,
                    },
                    method,
                    input,
                    image,
                )
                .is_none()
        );
        let mut changed = method.clone();
        changed.name_span = tcl_lexer::Span::new(0, 1);
        assert!(
            class
                .original_members
                .logical_name_operand(side, &changed, input, image)
                .is_none()
        );
        let foreign = tcl_lexer::SourceImage::document("different source");
        assert!(
            class
                .original_members
                .logical_name_operand(side, method, input, &foreign)
                .is_none()
        );
        let mut changed_input = input.clone();
        changed_input.unit_profile = Analyser::new()
            .analyse(source, "tcl8.6")
            .resolved_input
            .unwrap()
            .unit_profile;
        assert!(
            class
                .original_members
                .logical_name_operand(side, method, &changed_input, image)
                .is_none()
        );
    }
    assert!(
        class
            .original_members
            .logical_name_operand(MemberSide::Instance, &class.constructors[0], input, image)
            .is_none()
    );
    for name in ["generated", "expanded"] {
        let method = &class.methods[name];
        assert!(
            class
                .original_members
                .logical_name_operand(MemberSide::Instance, method, input, image)
                .is_none()
        );
    }
}

#[test]
fn logical_method_name_roles_follow_the_selected_family_grammar() {
    for source in [
        "snit::type C {method {two words} {} {}}",
        "itcl::class C {method {two words} {} {}}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::C"];
        let method = &class.methods["two words"];
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = analysis
            .retained_command_realm()
            .unwrap()
            .original_source_image()
            .unwrap();
        let word = class
            .original_members
            .logical_name_operand(MemberSide::Instance, method, input, image)
            .unwrap();
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_ascii_presentation(word).unwrap(),
            b"two words"
        );
    }
}

#[test]
fn logical_member_opacity_uses_selected_name_words_instead_of_literal_sigils() {
    let source = "oo::class create C {method {$literal} {} {}; method {${literal}} {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let class = &analysis.all_classes["::C"];
    assert!(class.methods.contains_key("$literal"));
    assert!(class.methods.contains_key("${literal}"));
    assert!(!class.member_set_incomplete);
    let unsupported =
        Analyser::new().analyse("oo::class create C {method {nul\0name} {} {}}", "tcl");
    let class = &unsupported.all_classes["::C"];
    assert!(class.methods.contains_key("nul\0name"));
    assert!(class.member_set_incomplete);
    for body in [
        "method $computed {} {}",
        "method {*}$parts",
        "method {*}{expanded} {} {}",
    ] {
        let source = format!("oo::class create C {{{body}}}");
        let analysis = Analyser::new().analyse(&source, "tcl");
        let class = &analysis.all_classes["::C"];
        assert!(class.member_set_incomplete);
        assert!(class.methods.values().all(|method| {
            class
                .original_members
                .logical_name_operand(
                    MemberSide::Instance,
                    method,
                    analysis.resolved_input.as_ref().unwrap(),
                    analysis
                        .retained_command_realm()
                        .unwrap()
                        .original_source_image()
                        .unwrap(),
                )
                .is_none()
        }));
    }
}

fn assert_logical_instance_member_present(
    class: &crate::analyser::types::ClassDef,
    analysis: &crate::analyser::AnalysisResult,
) {
    let actual_input = analysis.resolved_input.as_ref().unwrap();
    let actual_image = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    assert!(
        class
            .original_members
            .logical_name_operand(
                MemberSide::Instance,
                &class.methods["m"],
                actual_input,
                actual_image
            )
            .is_some()
    );
}

fn assert_logical_instance_member_absent(
    class: &crate::analyser::types::ClassDef,
    analysis: &crate::analyser::AnalysisResult,
) {
    let actual_input = analysis.resolved_input.as_ref().unwrap();
    let actual_image = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    assert!(
        class
            .original_members
            .logical_name_operand(
                MemberSide::Instance,
                &class.methods["m"],
                actual_input,
                actual_image
            )
            .is_none()
    );
}

#[test]
fn logical_member_name_ingress_refuses_reconstructed_wrapper_vectors() {
    let source = "oo::class create C {self {method m {} {}}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let class = &analysis.all_classes["::C"];
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    assert!(
        class
            .original_members
            .logical_name_operand(
                MemberSide::ClassObject,
                &class.class_methods["m"],
                input,
                image
            )
            .is_none()
    );

    let direct_source = "oo::class create C {method m {} {}}";
    let mut analyser = Analyser::new();
    let analysis = analyser
        .analyse_and_retain_result_for_test(direct_source, "tcl")
        .clone();
    let mut class = analysis.all_classes["::C"].clone();
    class.original_members = crate::analyser::types::OriginalSourceMemberLedger::default();
    let current = &analysis.all_classes["::C"];
    let actual_input = analysis.resolved_input.as_ref().unwrap();
    let actual_image = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    let outer = tcl_source::segment::segment_commands_with_offset_and_config(
        direct_source,
        0,
        actual_input.lexer_config(),
    );
    let [outer] = outer.as_slice() else {
        panic!("one authentic original class command")
    };
    let groups = tcl_lexer::group_commands_bytes(
        &outer.all_tokens,
        actual_image.bytes(),
        actual_input.lexer_config(),
    );
    let body_group = &groups[0].words[3];
    assert_eq!(body_group.span, current.body_span);
    let body = tcl_lexer::NativeWord::from_group(
        actual_image.clone(),
        actual_input.lexer_config(),
        &outer.all_tokens,
        body_group,
    )
    .unwrap();
    let content = crate::registry_invocation::original_written_script_body_span(&body).unwrap();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        &direct_source[content.as_range()],
        content.start(),
        actual_input.lexer_config().nested(),
    );
    let mut command = commands
        .into_iter()
        .find(|command| command.name() == "method")
        .unwrap();
    analyser.retain_logical_member_name_operand(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &command,
        &mut class,
    );
    assert_logical_instance_member_present(&class, &analysis);
    class.original_members = crate::analyser::types::OriginalSourceMemberLedger::default();
    command.argv[2] = command.argv[1];
    assert!(analyser.member_declaration_is_opaque(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &command,
        &command.texts,
    ));
    analyser.retain_logical_member_name_operand(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &command,
        &mut class,
    );
    assert_logical_instance_member_absent(&class, &analysis);
}

#[test]
fn loop_installed_members_require_the_original_variable_word() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.1");
    let dialect = tcl_registry::model::ingress::static_context_for("tcl9.1")
        .context()
        .authoring_query();
    let shape =
        |body, name| loop_installed_member_shape(tcloo(), body, 7, config, Some(dialect), name);
    for body in [
        "method $name {} {}",
        r#"method "$name" {} {}"#,
        "method ${name} {} {}",
        "private method $name {} {}",
        "self method $name {} {}",
    ] {
        assert!(shape(body, "name").is_some(), "{body}");
    }
    for body in [
        "method {$name} {} {}",
        r"method \$name {} {}",
        "method {${name}} {} {}",
        "method prefix$name {} {}",
        "method $name$suffix {} {}",
        "method $name(key) {} {}",
        "method ${name(key)} {} {}",
        "method {*}{$name} {} {}",
        r#"method " $name" {} {}"#,
    ] {
        assert!(shape(body, "name").is_none(), "{body}");
    }
    assert!(shape("method ${literal:::name} {} {}", "literal:::name").is_some());
    assert!(shape("method ${literal:::name} {} {}", "literal::name").is_none());
    let flat = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..config
    };
    assert!(
        loop_installed_member_shape(
            tcloo(),
            "method ${a{b} {} {}",
            7,
            flat,
            Some(dialect),
            "a{b"
        )
        .is_some()
    );
    assert!(shape("method ${a{b} {} {}", "a{b").is_none());
}

#[test]
fn original_member_script_projection_keeps_body_bom_as_keyword_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Exact member grammar syntax; no installed method or Native worker.
    let config = tcl_lexer::LexerConfig::for_file_grammar(
        crate::environment_ingress::resolve_environment("tcl9.1").grammar(),
    );
    let dialect = tcl_registry::model::ingress::static_context_for("tcl9.1")
        .context()
        .authoring_query();
    let positive = "method $name {} {}";
    assert!(
        loop_installed_member_shape(tcloo(), positive, 7, config, Some(dialect), "name").is_some()
    );
    assert!(
        loop_installed_member_shape(
            tcloo(),
            "\u{feff}method $name {} {}",
            7,
            config,
            Some(dialect),
            "name"
        )
        .is_none()
    );
    let source = "private {\u{feff}method ghost {} {}}";
    let command = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let expanded =
        expand_wrapper_block_members(source, tcloo(), &command.texts, &command.argv, config)
            .unwrap();
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].0[1], "\u{feff}method");
    assert!(tcloo().member(&expanded[0].0[1]).is_none());
}

#[test]
fn original_vendor_class_declarations_keep_selected_source_role_and_unavailable_units() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    let source = r"snit::type {Plain} {}
snit::type {C\uD800} {}";
    for profile in ["f5-irules", "f5-iapps", "f5-tmsh"] {
        let result = Analyser::new().analyse(source, profile);
        let classes = result
            .original_vendor_class_declarations()
            .collect::<Vec<_>>();
        assert_eq!(classes.len(), 2, "{profile}");
        for class in &classes {
            assert_eq!(
                class.purpose(),
                tcl_syntax::naming::VendorSourceNamePurpose::SourceName
            );
            assert_eq!(
                class.name_input().source_image(),
                &tcl_lexer::SourceImage::document(source)
            );
            assert!(
                class.metadata().source_name.is_none(),
                "hosted source cards have no C/Jim slot"
            );
            assert_eq!(
                result.original_vendor_source_name_in_source(
                    class.name_input().source_image(),
                    class.name_input().lexer_config(),
                    class.metadata().name_span,
                ),
                Some(class.original_occurrence())
            );
        }
        assert_eq!(
            classes[0].name_input().literal_units(classes[0].purpose()),
            Some(b"Plain".as_slice())
        );
        assert!(
            classes[1]
                .name_input()
                .literal_units(classes[1].purpose())
                .is_none()
        );
        assert_eq!(result.original_class_declarations().count(), 0);
    }
    let result = Analyser::new().analyse("snit::type Plain {}", "tcl8.6");
    assert_eq!(result.original_vendor_class_declarations().count(), 0);
}
