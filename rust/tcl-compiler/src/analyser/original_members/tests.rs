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
fn original_property_source_provenance_refuses_foreign_profile() {
    let source = "oo::configurable create C {property size}";
    let mut current = Analyser::new().analyse(source, "tcl9.0");
    let original = current.original_class_declarations().next().unwrap();
    let property = original
        .metadata()
        .original_properties
        .declarations()
        .next()
        .unwrap();
    assert!(property.matches_original_declaration_source(source, &current, original));
    let foreign = Analyser::new().analyse(source, "tcl9.1");
    assert_ne!(current.resolved_input, foreign.resolved_input);
    current.original_class_metadata = foreign.original_class_metadata;
    let transplanted = current.original_class_declarations().next().unwrap();
    let property = transplanted
        .metadata()
        .original_properties
        .declarations()
        .next()
        .unwrap();
    assert!(!property.matches_original_declaration_source(source, &current, transplanted));
}

#[test]
fn counted_initial_methods_do_not_include_later_configurations() {
    let source = "oo::class create C\\uD800 {method original {} {}}; oo::define C\\uD800 method later {} {}; rename C\\uD800 {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("one original class");
    };
    let methods = class
        .metadata()
        .initial_method_declarations()
        .expect("original body inventory");
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].original_name_input().bytes(), b"original");
}

#[test]
fn counted_initial_methods_keep_class_side_and_duplicate_sites() {
    let source =
        "oo::class create C\\uD800 {method m {} {}; method m {} {}; self method create {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let class = analysis.original_counted_class_metadata.first().unwrap();
    let methods = class
        .metadata()
        .initial_method_declarations()
        .expect("complete source inventory");
    assert_eq!(methods.len(), 3);
    assert_eq!(methods[0].original_name_input().bytes(), b"m");
    assert_eq!(methods[1].original_name_input().bytes(), b"m");
    assert_ne!(methods[0].site(), methods[1].site());
    assert_eq!(methods[0].side(), MemberSide::Instance);
    assert_eq!(methods[2].original_name_input().bytes(), b"create");
    assert_eq!(methods[2].side(), MemberSide::ClassObject);
}

#[test]
fn counted_initial_methods_decline_malformed_bodies() {
    let source = "oo::class create C\\uD800 {method m {} {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        analysis.original_counted_class_metadata,
        [] as [crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::types::OriginalCountedClass,
        >; 0]
    );
    let inner = "oo::class create C\\uD800 {method m {} \"unterminated}";
    let analysis = Analyser::new().analyse(inner, "tcl8.6");
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("outer declaration and body remain present");
    };
    assert!(class.metadata().initial_method_declarations().is_none());
}

#[test]
fn counted_initial_methods_preserve_original_byte_names_and_role_words() {
    let source =
        "oo::class create C\\uD800 {method m\\uD800 {x} {return $x}; method m\\uD801 {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("one counted class");
    };
    let methods = class
        .metadata()
        .initial_method_declarations()
        .expect("retained source method declarations");
    assert_eq!(methods.len(), 2);
    assert_eq!(methods[0].original_name_input().bytes(), b"m\xed\xa0\x80");
    assert_eq!(methods[1].original_name_input().bytes(), b"m\xed\xa0\x81");
    assert_eq!(
        methods[0].parameters_word().unwrap().try_text().unwrap(),
        "{x}"
    );
    assert_eq!(
        methods[0].body_word().unwrap().try_text().unwrap(),
        "{return $x}"
    );
    assert_eq!(methods[0].side(), MemberSide::Instance);
    assert_eq!(
        methods[0].original_word().image().bytes(),
        source.as_bytes()
    );
    assert_eq!(
        methods[0].site().source.source_image().bytes(),
        source.as_bytes()
    );
    assert_ne!(methods[0].site(), methods[1].site());
    assert!(analysis.all_classes.is_empty());
    assert!(analysis.global_scope.classes.is_empty());
}

#[test]
fn counted_initial_methods_decline_unknown_dynamic_and_loop_sources() {
    for body in [
        "{unknown_member; method m {} {}}",
        "{method $name {} {}}",
        "{foreach name {a b} {method $name {} {}}}",
    ] {
        let source = format!("oo::class create C\\uD800 {body}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let [class] = analysis.original_counted_class_metadata.as_slice() else {
            panic!("{source}");
        };
        assert!(
            class.metadata().initial_method_declarations().is_none(),
            "{source}"
        );
    }
}

#[test]
fn counted_initial_body_owner_rejects_swapped_body_and_stale_image() {
    let source =
        "oo::class create C\\uD800 {method c {} {}}; oo::class create D\\uD801 {method d {} {}}";
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, "tcl8.6");
    assert_eq!(analysis.original_counted_class_metadata.len(), 2);
    let records = &analysis.original_counted_class_metadata;
    assert!(
        records
            .iter()
            .all(|record| record.metadata().initial_method_declarations().is_some())
    );
    let declaration = crate::registry_invocation::source_structure::source_class_declaration_at(
        source, &analysis, 0,
    )
    .unwrap();
    assert!(
        analyser
            .counted_initial_body_methods(&declaration, records[1].metadata().body(), &[])
            .is_none()
    );
    let unchanged = std::sync::Arc::clone(&analyser.source);
    analyser.source = format!("#{unchanged}").into();
    assert!(
        analyser
            .counted_initial_body_methods(&declaration, records[0].metadata().body(), &[])
            .is_none()
    );
    analyser.source = unchanged;
    analyser.result = Analyser::new().analyse(source, "tcl9.0");
    assert!(
        analyser
            .counted_initial_body_methods(&declaration, records[0].metadata().body(), &[])
            .is_none()
    );
    analyser.result = analysis.clone();
    let mut foreign = Analyser::new();
    foreign.analyse(source, "tcl9.0");
    analyser.registry = foreign.registry.clone();
    assert!(
        analyser
            .counted_initial_body_methods(&declaration, records[0].metadata().body(), &[])
            .is_none()
    );
}

#[test]
fn logical_inline_member_metadata_keeps_actual_original_name_and_body_words() {
    let source = "oo::class create C {}; oo::define C method added {arg} {return $arg}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let class = &analysis.all_classes["::C"];
    let methods = class
        .original_members
        .methods(MemberSide::Instance)
        .expect("genuine Logical inline source member ledger");
    let [method] = methods.as_slice() else {
        panic!("one actual inline declaration");
    };
    assert_eq!(method.original_name_input().bytes(), b"added");
    let key = method.original_name_input().original_word_key().unwrap();
    assert_eq!(key.original_word().image(), &SourceImage::document(source));
    assert!(
        SignatureSourceNameKey::from_original_native_word(
            key.original_word(),
            tcl_syntax::word_rules::WordValueRules::from_config(&key.original_word().config()),
            key.policy()
        )
        .is_none()
    );
    assert_eq!(
        method.parameters_word().unwrap().try_text().unwrap(),
        "{arg}"
    );
    assert_eq!(
        method.body_word().unwrap().try_text().unwrap(),
        "{return $arg}"
    );
}

#[test]
fn logical_inline_member_words_require_one_complete_retained_source_suffix() {
    let source = "oo::class create C {method setup {} {oo::define C method added {arg} {return $arg}}}; C create object; object setup";
    let mut analyser = Analyser::new();
    analyser.result = analyser.analyse(source, "tcl");
    let input = analyser.result.resolved_input.as_ref().unwrap().clone();
    let offset = u32::try_from(source.find("oo::define").unwrap()).unwrap();
    let original = analyser
        .head_identities
        .source_bindings_ref()
        .original_logical_source_command_at_in_source(
            &input,
            &SourceImage::document(source),
            offset,
            input.lexer_config(),
        )
        .expect("genuine retained receiver-source inline command");
    let texts = &original.command().texts[2..];
    let tokens = &original.command().argv[2..];
    let words = analyser
        .original_logical_inline_member_words(texts, tokens)
        .expect("the actual original inline member suffix");
    assert_eq!(words.len(), 4);
    assert_eq!(words[0].try_text().unwrap(), "method");
    assert_eq!(words[1].try_text().unwrap(), "added");
    assert_eq!(words[2].try_text().unwrap(), "{arg}");
    assert_eq!(words[3].try_text().unwrap(), "{return $arg}");
    let mut changed = texts.to_vec();
    changed[1] = "renamed".into();
    assert!(
        analyser
            .original_logical_inline_member_words(&changed, tokens)
            .is_none()
    );
    let mut changed = tokens.to_vec();
    changed.swap(2, 3);
    assert!(
        analyser
            .original_logical_inline_member_words(texts, &changed)
            .is_none()
    );
    assert!(
        analyser
            .original_logical_inline_member_words(&texts[..3], &tokens[..3])
            .is_none(),
        "a prefix cannot masquerade as the whole member suffix"
    );
    analyser.result.resolved_input = None;
    assert!(
        analyser
            .original_logical_inline_member_words(texts, tokens)
            .is_none()
    );
    analyser.result.resolved_input = Some(input.clone());
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    analyser.result.body_lexer_config = Some(config);
    assert!(
        analyser
            .original_logical_inline_member_words(texts, tokens)
            .is_none()
    );
    analyser.result.body_lexer_config = Some(input.lexer_config());
    analyser.source = format!("{}{}", analyser.source, ' ').into();
    assert!(
        analyser
            .original_logical_inline_member_words(texts, tokens)
            .is_none()
    );
}

#[test]
fn logical_member_names_retain_source_ledgers_without_native_word_admission() {
    let source = "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_retained_logical_declaration_advice());
    let class = &analysis.all_classes["::Meta"];
    let methods = class
        .original_members
        .methods(MemberSide::ClassObject)
        .expect("the genuine Logical original member inventory");
    let [method] = methods.as_slice() else {
        panic!("one actual ClassObject member");
    };
    assert_eq!(method.original_name_input().bytes(), b"create");
    let roles = method
        .original_parameter_names_and_body(&analysis)
        .expect("the same genuine Logical formal/body role owner");
    assert_eq!(
        roles.parameter_names().collect::<Vec<_>>(),
        vec!["name", "body"]
    );
    assert_eq!(roles.body_word(), method.body_word().unwrap());
    let key = method.original_name_input().original_word_key().unwrap();
    assert!(
        SignatureSourceNameKey::from_original_native_word(
            key.original_word(),
            tcl_syntax::word_rules::WordValueRules::from_config(&key.original_word().config()),
            key.policy(),
        )
        .is_none(),
        "source geometry cannot satisfy the Native word purpose"
    );
    let effects = class.original_relations.effects().collect::<Vec<_>>();
    let [effect] = effects.as_slice() else {
        panic!("the actual superclass source effect");
    };
    assert_eq!(effect.kind(), OriginalSourceClassRelationKind::Superclass);
    assert_eq!(
        effect.values().unwrap()[0].name_input().bytes(),
        b"oo::class"
    );
}

#[test]
fn original_readonly_method_names_keep_the_selected_option_position() {
    // naming.tcloo.original-member-body-source-roles
    // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
    let source = r"oo::class create C {foreach m {p\uD800 p\uD801} {method $m -private {arg} {return $arg}}; method zero\x00tail {} {return zero}}";
    for dialect in ["tcl9.0", "tcl9.1"] {
        let analysis = super::super::Analyser::new().analyse(source, dialect);
        let class = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata();
        let methods = class
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        assert_eq!(methods.len(), 3, "{dialect}");
        for bytes in [b"p\xed\xa0\x80".as_slice(), b"p\xed\xa0\x81".as_slice()] {
            let method = methods
                .iter()
                .find(|method| method.original_name_input().bytes() == bytes)
                .unwrap();
            assert!(method.declaration().static_occurrence().is_none());
            assert!(method.declaration().source_installer().is_some());
            assert!(!method.exported());
            let parameters = method.parameters_word().unwrap();
            assert_eq!(
                parameters
                    .image()
                    .bytes()
                    .get(parameters.content_span().unwrap().as_range()),
                Some(b"arg".as_slice())
            );
            assert!(method.body_word().is_some());
        }
        let zero = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"zero\xc0\x80tail")
            .unwrap();
        assert!(zero.declaration().static_occurrence().is_some());
        assert!(zero.parameters_word().is_some());
        assert!(zero.body_word().is_some());
    }
}

fn foreign_member_body_word() -> tcl_lexer::NativeWord {
    let foreign = super::super::Analyser::new().analyse(
        "oo::class create Foreign {method pick {arg} {return $arg}}",
        "tcl8.6",
    );
    foreign
        .original_class_declarations()
        .next()
        .unwrap()
        .metadata()
        .original_members
        .declarations()
        .next()
        .unwrap()
        .body_word()
        .unwrap()
        .clone()
}

#[test]
fn original_method_source_roles_retain_complete_words_across_routes_and_readonly_names() {
    // Implementation contract: naming.tcloo.original-member-body-source-roles
    // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
    let source = "oo::class create C {method pick {arg} {return $arg}; renamemethod pick moved; foreach m {one two} {method $m {} {return body}}; forward proxy target fixed}";
    let analysis = super::super::Analyser::new().analyse(source, "tcl8.6");
    let class = analysis
        .original_class_declarations()
        .next()
        .unwrap()
        .metadata();
    let methods = class
        .original_members
        .methods(MemberSide::Instance)
        .unwrap();
    let moved = methods
        .iter()
        .find(|method| method.original_name_input().bytes() == b"moved")
        .unwrap();
    assert_eq!(moved.declaration().original_name_input().bytes(), b"pick");
    for method in methods.iter().filter(|method| method.body_word().is_some()) {
        let body = method.body_word().unwrap();
        assert_eq!(body.image(), &SourceImage::document(source));
        assert_eq!(body.config(), method.declaration().original_word().config());
        assert!(method.parameters_word().is_some());
        assert!(!body.group().expand);
        let dialect = method.source_dialect().unwrap();
        assert_eq!(dialect.tcl_version, Some(tcl_dialect::TclVersion::V8_6));
        assert_eq!(
            dialect.parameter_grammar(),
            Some(tcl_dialect::ParameterGrammar::Tcl)
        );
        assert_eq!(
            body.config().grammar_over(dialect.lexer_grammar),
            dialect.lexer_grammar
        );
    }
    let readonly = methods
        .iter()
        .filter(|method| method.declaration().static_occurrence().is_none())
        .collect::<Vec<_>>();
    assert_eq!(readonly.len(), 2);
    assert!(
        readonly
            .iter()
            .all(|method| method.parameters_word().is_some() && method.body_word().is_some())
    );
    for method in &readonly {
        let installer = method.declaration().source_installer().unwrap();
        assert!(installer.matches_source(
            &SourceImage::document(source),
            method.declaration().original_word().config()
        ));
        assert_eq!(installer.candidate().name, "foreach");
        assert_ne!(installer.obligations(), []);
        assert!(installer.site().offset < method.declaration().site().offset);
    }
    assert!(moved.declaration().source_installer().is_none());
    let forward = methods
        .iter()
        .find(|method| method.original_name_input().bytes() == b"proxy")
        .unwrap();
    assert!(forward.parameters_word().is_none());
    assert!(forward.body_word().is_none());
    let foreign_word = foreign_member_body_word();
    assert!(
        moved
            .clone()
            .with_body_role_words(None, Some(foreign_word), moved.source_dialect())
            .is_none()
    );
    let unknown = moved
        .clone()
        .with_body_role_words(
            moved.parameters_word().cloned(),
            moved.body_word().cloned(),
            None,
        )
        .unwrap();
    assert!(unknown.parameters_word().is_some());
    assert!(unknown.body_word().is_some());
    assert!(unknown.source_dialect().is_none());
    let jim = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let foreign_dialect = moved
        .clone()
        .with_body_role_words(
            moved.parameters_word().cloned(),
            moved.body_word().cloned(),
            Some(jim),
        )
        .unwrap();
    assert!(foreign_dialect.source_dialect().is_none());
}

#[test]
fn original_literal_member_installer_keeps_known_shadow_and_shape_barriers() {
    // Implementation contract: naming.tcloo.original-member-body-source-roles
    // docs/design/analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md
    for source in [
        "proc foreach {args} {}; oo::class create C {foreach m {one two} {method $m {} {return body}}}",
        "oo::class create C {foreach {m n} {one two} {method $m {} {return body}}}",
        "oo::class create C {foreach m $unknown {method $m {} {return body}}}",
    ] {
        let analysis = super::super::Analyser::new().analyse(source, "tcl8.6");
        let class = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata();
        assert!(
            class
                .original_members
                .methods(MemberSide::Instance)
                .is_none(),
            "{source}"
        );
        assert_eq!(class.original_members.declarations().count(), 0, "{source}");
    }
}

#[test]
fn original_special_members_keep_keyword_body_and_definition_owners_separate() {
    let source = "oo::class create C {constructor {arg} {set local $arg}; destructor {puts CLEAN}; method p {} {}}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let class = class(source, dialect);
        let entries = class
            .original_special_members
            .declarations()
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2, "{dialect}");
        assert_eq!(
            entries[0].kind(),
            tcl_registry::definer::DefinitionSpecialMemberKind::Constructor
        );
        assert_eq!(
            entries[1].kind(),
            tcl_registry::definer::DefinitionSpecialMemberKind::Destructor
        );
        for entry in &entries {
            assert_eq!(entry.keyword_word().image(), &SourceImage::document(source));
            assert_eq!(entry.body_word().image(), &SourceImage::document(source));
            assert_eq!(
                entry.site().offset,
                entry.keyword_word().group().span.start()
            );
            assert_eq!(entry.metadata().body_span, entry.body_word().span());
            assert!(entry.site().offset < entry.body_word().span().start());
        }
        assert_eq!(entries[0].parameters_word().unwrap().bytes(), b"{arg}");
        assert!(entries[1].parameters_word().is_none());
        assert_eq!(class.original_members.declarations().count(), 1);
    }
}

#[test]
fn original_special_members_retain_empty_replacements_without_inferring_absence() {
    let source = "oo::class create C {constructor args {}; destructor {}}; oo::define C {constructor {} {}; destructor {}}";
    let result = Analyser::new().analyse(source, "tcl9.0");
    let class = result
        .original_class_declarations()
        .next()
        .unwrap()
        .metadata();
    let declarations = class
        .original_special_members
        .declarations()
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 4);
    assert!(declarations.iter().all(|entry| {
        entry
            .body_word()
            .content_span()
            .unwrap()
            .as_range()
            .is_empty()
    }));
    let delta = result.original_class_configurations().next().unwrap();
    assert_eq!(delta.special_members().declarations().count(), 2);
    let configuration_offset = u32::try_from(source.find("oo::define").unwrap()).unwrap();
    assert!(
        delta
            .special_members()
            .declarations()
            .all(|entry| entry.site().offset > configuration_offset)
    );
}

#[test]
fn original_property_options_retain_exact_declaration_policy_and_source() {
    // Implementation contract: naming.tcloo.original-property-accessor-source-advice
    // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
    let source = r"oo::configurable create C {property p\uD800 p\uD801 zero\x00tail}";
    for dialect in ["tcl9.0", "tcl9.1"] {
        let class = class(source, dialect);
        let properties = class
            .original_properties
            .properties(MemberSide::Instance)
            .unwrap();
        assert_eq!(properties.len(), 3);
        let options = properties
            .iter()
            .map(|property| property.option_name().unwrap().selected().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(
            options,
            vec![
                b"-p\xed\xa0\x80".to_vec(),
                b"-p\xed\xa0\x81".to_vec(),
                b"-zero\xc0\x80tail".to_vec()
            ]
        );
        for property in properties {
            let option = property.option_name().unwrap();
            assert_eq!(option.original(), property.original_name_input().bytes());
            assert_eq!(
                property.declaration().name_input().source_image(),
                &SourceImage::document(source)
            );
        }
    }
}

#[test]
fn original_literal_loop_members_keep_readonly_list_children_and_worker_words() {
    let source = "oo::class create C {foreach m {p\\uD800 p\\uD801} {method $m {args} {return $args}}; method fetch {} {}}";
    let class = class(source, "tcl9.0");
    let methods = class
        .original_members
        .methods(MemberSide::Instance)
        .expect("closed source candidate fold");
    assert_eq!(methods.len(), 3);
    let generated = methods
        .iter()
        .filter(|method| method.declaration().static_occurrence().is_none())
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 2);
    assert_eq!(
        generated
            .iter()
            .map(|method| method.original_name_input().bytes())
            .collect::<Vec<_>>(),
        vec![b"p\xed\xa0\x80".as_slice(), b"p\xed\xa0\x81".as_slice()]
    );
    for method in generated {
        let declaration = method.declaration();
        assert!(matches!(
            declaration.original_name_input(),
            SignatureSourceNameInput::OriginalValue(_)
        ));
        assert_eq!(declaration.original_word().try_text().unwrap(), "$m");
        assert_eq!(
            declaration.original_word().image(),
            &SourceImage::document(source)
        );
        assert!(declaration.site().offset < declaration.original_word().span().start());
        assert!(method.metadata().params_computed && method.metadata().params.is_empty());
        assert!(method.original_name_input().original_word_key().is_none());
    }
    let direct = methods
        .iter()
        .find(|method| method.original_name_input().bytes() == b"fetch")
        .unwrap();
    assert!(direct.declaration().static_occurrence().is_some());
}

#[test]
fn original_literal_loop_member_candidates_require_registry_shape_and_single_variable() {
    for source in [
        "proc foreach args {}; oo::class create C {foreach m {fake} {method $m {} {}}}",
        "oo::class create C {foreach {a b} {fake alternate} {method $a {} {}}}",
        "oo::class create C {foreach m $names {method $m {} {}}}",
        "oo::class create C {foreach m {fake} {method prefix$m {} {}}}",
    ] {
        let class = class(source, "tcl9.0");
        assert!(
            class
                .original_members
                .declarations()
                .all(|method| method.declaration().static_occurrence().is_some()),
            "{source}"
        );
    }
}

fn class(source: &str, dialect: &str) -> ClassDef {
    let result = Analyser::new().analyse(source, dialect);
    result
        .original_class_declarations()
        .next()
        .expect("genuine original class publication")
        .metadata()
        .clone()
}

#[test]
fn original_opaque_member_names_survive_reporting_collisions_and_moves() {
    let source = r"oo::class create C {method p\uD800 {} {return A}; method p\uD801 {} {return B}; renamemethod p\uD800 q\uD800; unexport p\uD801}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let class = class(source, dialect);
        let methods = class
            .original_members
            .methods(MemberSide::Instance)
            .expect("complete static original metadata");
        assert_eq!(methods.len(), 2, "{dialect}");
        let moved = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"q\xed\xa0\x80")
            .unwrap();
        assert_eq!(
            moved.declaration().original_name_input().bytes(),
            b"p\xed\xa0\x80"
        );
        assert!(moved.exported());
        let private = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"p\xed\xa0\x81")
            .unwrap();
        assert!(!private.exported());
        assert!(std::str::from_utf8(private.original_name_input().bytes()).is_err());
        assert_eq!(class.original_members.effects().count(), 2);
    }
}

#[test]
fn original_wrapper_tables_visibility_and_forward_inputs_remain_distinct() {
    let source = r"oo::class create C {method ping {} {}; self {method ping {} {}; renamemethod ping pong}; private {method hidden {} {}}; forward go target p\uD800; unexport ping; method ping {} {return RESET}}";
    for dialect in ["tcl9.0", "tcl9.1"] {
        let class = class(source, dialect);
        let own = class
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        let object = class
            .original_members
            .methods(MemberSide::ClassObject)
            .unwrap();
        assert_eq!(object.len(), 1);
        assert_eq!(object[0].original_name_input().bytes(), b"pong");
        assert_eq!(
            object[0].declaration().original_name_input().bytes(),
            b"ping"
        );
        assert!(
            own.iter()
                .find(|method| method.original_name_input().bytes() == b"ping")
                .unwrap()
                .exported()
        );
        assert!(
            !own.iter()
                .find(|method| method.original_name_input().bytes() == b"hidden")
                .unwrap()
                .exported()
        );
        let forward = own
            .iter()
            .find(|method| method.original_name_input().bytes() == b"go")
            .unwrap();
        let inputs = forward.forward_prefix().unwrap();
        assert_eq!(inputs[0].bytes(), b"target");
        assert_eq!(inputs[1].bytes(), b"p\xed\xa0\x80");
    }
}

#[test]
fn original_computed_member_name_withdraws_own_metadata_view() {
    let class = class(
        "oo::class create C {method fixed {} {}; method $name {} {}}",
        "tcl8.6",
    );
    assert!(
        class
            .original_members
            .methods(MemberSide::Instance)
            .is_none()
    );
    assert!(
        class
            .original_members
            .declarations()
            .any(|method| method.original_name_input().bytes() == b"fixed")
    );
}

#[test]
fn original_relations_preserve_opaque_operands_caller_scope_and_receiver_side() {
    // Implementation contract: naming.tcloo.original-relation-provider-fold-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-relation-provider-fold-contract.md
    let source = r"namespace eval N {oo::class create C {superclass B\uD800 B\uD801; mixin B\uD800; self mixin B\uD801}}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let class = class(source, dialect);
        let supers = class
            .original_relations
            .resolve(
                MemberSide::Instance,
                OriginalSourceClassRelationKind::Superclass,
                |relation| Some(relation.name_input().bytes().to_vec()),
            )
            .unwrap();
        assert_eq!(
            supers,
            vec![b"B\xed\xa0\x80".to_vec(), b"B\xed\xa0\x81".to_vec()]
        );
        let mixes = class
            .original_relations
            .resolve(
                MemberSide::ClassObject,
                OriginalSourceClassRelationKind::Mixin,
                |relation| Some(relation.name_input().bytes().to_vec()),
            )
            .unwrap();
        assert_eq!(mixes, vec![b"B\xed\xa0\x81".to_vec()]);
        let relation = class
            .original_relations
            .effects()
            .next()
            .unwrap()
            .values()
            .unwrap()
            .first()
            .unwrap();
        let candidates = relation.lookup().candidates().unwrap();
        assert_eq!(
            candidates[0].namespace,
            tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
        );
        assert_eq!(candidates[0].simple.as_bytes(), b"B\xed\xa0\x80");
    }
}

#[test]
fn original_relation_removal_compares_joined_providers_and_renews_after_unknown() {
    // Implementation contract: naming.tcloo.original-relation-provider-fold-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-relation-provider-fold-contract.md
    let replaced = class(
        "oo::class create C {mixin First; mixin -remove Other; mixin $unknown; mixin -set Last}",
        "tcl9.1",
    );
    let resolved = replaced.original_relations.resolve(
        MemberSide::Instance,
        OriginalSourceClassRelationKind::Mixin,
        |relation| match relation.name_input().bytes() {
            b"First" | b"Other" => Some(1),
            b"Last" => Some(2),
            _ => None,
        },
    );
    assert_eq!(
        resolved,
        Some(vec![2]),
        "an exact replacement renews the relation list after incomplete operand coverage"
    );
    let class = class(
        "oo::class create C {mixin First; mixin -remove Other}",
        "tcl9.1",
    );
    assert_eq!(
        class.original_relations.resolve(
            MemberSide::Instance,
            OriginalSourceClassRelationKind::Mixin,
            |_| Some(1)
        ),
        Some(Vec::<i32>::new()),
        "different spellings can independently join the same actual class provider"
    );
}

#[test]
fn original_class_configuration_joins_opaque_target_allocation_and_own_delta() {
    // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
    let source = r"oo::class create C\uD800 {method p\uD800 {} {}}; oo::class create C\uD801 {method p\uD801 {} {}}; oo::define C\uD800 {renamemethod p\uD800 moved; mixin -set M\uD800}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let declarations = result.original_class_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2, "{dialect}");
        let configuration = result
            .original_class_configurations()
            .next()
            .expect("original target point and full member delta");
        assert_eq!(
            configuration.target().name_input().bytes(),
            b"C\xed\xa0\x80"
        );
        assert_eq!(
            configuration.target().local_allocation().unwrap().site,
            *declarations[0].declaration_site()
        );
        assert!(!configuration.target().is_external_candidate());
        let first = declarations[0]
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        assert_eq!(first[0].original_name_input().bytes(), b"moved");
        assert_eq!(
            first[0].declaration().original_name_input().bytes(),
            b"p\xed\xa0\x80"
        );
        let second = declarations[1]
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        assert_eq!(second[0].original_name_input().bytes(), b"p\xed\xa0\x81");
        assert_eq!(
            configuration
                .relations()
                .effects()
                .next()
                .unwrap()
                .values()
                .unwrap()[0]
                .name_input()
                .bytes(),
            b"M\xed\xa0\x80"
        );
    }
}

#[test]
fn original_class_configuration_preserves_moved_identity_and_external_obligation() {
    // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
    let source = "oo::class create C {method old {} {}}; rename C D; oo::define D {renamemethod old changed}";
    let result = Analyser::new().analyse(source, "tcl9.1");
    let declaration = result.original_class_declarations().next().unwrap();
    let configuration = result.original_class_configurations().next().unwrap();
    assert_eq!(
        configuration.target().local_allocation().unwrap().site,
        *declaration.declaration_site()
    );
    assert_eq!(
        declaration
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap()[0]
            .original_name_input()
            .bytes(),
        b"changed"
    );
    let result = Analyser::new().analyse(r"oo::define Foreign {method p\uD800 {} {}}", "tcl9.1");
    assert_eq!(result.original_class_declarations().count(), 0);
    let configuration = result
        .original_class_configurations()
        .next()
        .expect("independent external target obligation");
    assert!(configuration.target().is_external_candidate());
    assert!(configuration.target().local_allocation().is_none());
    assert_eq!(
        configuration
            .members()
            .methods(MemberSide::Instance)
            .unwrap()[0]
            .original_name_input()
            .bytes(),
        b"p\xed\xa0\x80"
    );
}

#[test]
fn original_class_configuration_does_not_promote_alias_or_computed_target() {
    // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
    for source in [
        "oo::class create C {method old {} {}}; interp alias {} A {} C; oo::define A {method extra {} {}}",
        "oo::class create C {method old {} {}}; oo::define $unknown {method extra {} {}}",
    ] {
        let result = Analyser::new().analyse(source, "tcl9.1");
        assert_eq!(result.original_class_configurations().count(), 0);
        let class = result
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata();
        assert!(
            class
                .original_members
                .methods(MemberSide::Instance)
                .is_none()
        );
        assert!(
            class
                .original_members
                .declarations()
                .any(|method| method.original_name_input().bytes() == b"old")
        );
    }
}

#[test]
fn original_object_configuration_keeps_class_object_and_instance_tables_separate() {
    // Implementation contract: naming.tcloo.original-configuration-allocation-join-contract
    // docs/design/analysis/name-resolution-proofs/tcloo-original-configuration-allocation-join-contract.md
    let source = r"oo::class create C {method same {} {}; self method same {} {}}; oo::objdefine C {renamemethod same own; mixin -set ObjectMixin}";
    let result = Analyser::new().analyse(source, "tcl9.1");
    let declaration = result.original_class_declarations().next().unwrap();
    let configuration = result.original_class_configurations().next().unwrap();
    assert_eq!(
        configuration.target().layer(),
        tcl_registry::ObjectDispatchLayer::Object
    );
    assert_eq!(
        configuration.target().local_allocation().unwrap().site,
        *declaration.declaration_site()
    );
    let class = declaration.metadata();
    assert_eq!(
        class
            .original_members
            .methods(MemberSide::Instance)
            .unwrap()[0]
            .original_name_input()
            .bytes(),
        b"same"
    );
    assert_eq!(
        class
            .original_members
            .methods(MemberSide::ClassObject)
            .unwrap()[0]
            .original_name_input()
            .bytes(),
        b"own"
    );
    assert_eq!(
        class.original_relations.resolve(
            MemberSide::Instance,
            OriginalSourceClassRelationKind::Mixin,
            |_| Some(1)
        ),
        Some(Vec::<i32>::new())
    );
    assert_eq!(
        class.original_relations.resolve(
            MemberSide::ClassObject,
            OriginalSourceClassRelationKind::Mixin,
            |_| Some(1)
        ),
        Some(vec![1])
    );
}

#[test]
fn original_properties_retain_counted_names_and_per_declaration_options() {
    let source = r"oo::configurable create C {property p\uD800 -kind writable p\uD801 -get {return custom} q -kind readable -set {set value 1}}";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl9.1");
    let declaration = result
        .original_class_declarations()
        .next()
        .expect("genuine original class declaration");
    let properties = declaration
        .metadata()
        .original_properties
        .properties(MemberSide::Instance)
        .expect("complete own original property words");
    assert_eq!(properties.len(), 3);
    assert!(
        properties
            .iter()
            .all(|property| property.accessor_methods() == ["configure"])
    );
    assert_eq!(
        properties[0].original_name_input().bytes(),
        b"p\xed\xa0\x80"
    );
    assert_eq!(
        properties[0].kind(),
        tcl_registry::commands::tcl::TclOoPropertyKind::Writable
    );
    assert!(properties[0].getter().is_none());
    assert_eq!(
        properties[1].original_name_input().bytes(),
        b"p\xed\xa0\x81"
    );
    assert_eq!(
        properties[1].kind(),
        tcl_registry::commands::tcl::TclOoPropertyKind::ReadWrite
    );
    assert!(properties[1].getter().is_some());
    assert_eq!(
        properties[2].kind(),
        tcl_registry::commands::tcl::TclOoPropertyKind::Readable
    );
    assert!(properties[2].setter().is_some());
    assert_eq!(
        declaration
            .metadata()
            .original_properties
            .properties(MemberSide::ClassObject)
            .unwrap(),
        [] as [&crate::analyser::types::OriginalSourcePropertyMetadata; 0]
    );
    assert!(properties.iter().all(
        |property| property.declaration().name_input().source_image()
            == &tcl_lexer::SourceImage::document(source)
    ));
}

#[test]
fn original_object_configuration_retains_instance_allocation_without_class_table_donation() {
    let source = r"oo::class create C {method base {} {}}; C create d; oo::objdefine d {method own\uD800 {} {}; method own\uD801 {} {}}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let classes = result.original_class_declarations().collect::<Vec<_>>();
        assert_eq!(classes.len(), 1, "{dialect}");
        let own = result
            .original_object_configurations()
            .next()
            .expect("actual named-instance operand and independent delta");
        assert_eq!(own.target().name_input().unwrap().bytes(), b"d");
        assert_eq!(
            own.target().instance().allocation().site.offset,
            u32::try_from(source.find("C create d").unwrap()).unwrap()
        );
        let members = own
            .members()
            .methods(MemberSide::ClassObject)
            .expect("complete own-object source metadata");
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].original_name_input().bytes(), b"own\xed\xa0\x80");
        assert_eq!(members[1].original_name_input().bytes(), b"own\xed\xa0\x81");
        assert_eq!(
            classes[0]
                .metadata()
                .original_members
                .methods(MemberSide::ClassObject)
                .unwrap(),
            [] as [crate::analyser::types::OriginalSourceMethodMetadata; 0]
        );
        assert_eq!(
            classes[0]
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap()
                .len(),
            1
        );
        assert!(result.original_class_configurations().next().is_none());
    }
}

#[test]
fn original_object_configuration_uses_actual_variable_operand_read() {
    let source =
        r"oo::class create C {}; set object [C new]; oo::objdefine $object {method own {} {}}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let own = result
            .original_object_configurations()
            .next()
            .expect("reached object read, independently of generated spelling");
        assert_eq!(
            own.target().instance().allocation().site.offset,
            u32::try_from(source.find("C new").unwrap()).unwrap()
        );
        assert_eq!(
            own.members()
                .methods(MemberSide::ClassObject)
                .unwrap()
                .len(),
            1
        );
        assert!(result.original_class_configurations().next().is_none());
    }
    for source in [
        "oo::class create C {}; C create d; interp alias {} wrapper {} d; oo::objdefine wrapper {method own {} {}}",
        "oo::class create C {}; set object TEXT; oo::objdefine $object {method own {} {}}",
        "oo::class create C {}; set object [C new]; set object TEXT; oo::objdefine $object {method own {} {}}",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result.original_object_configurations().next().is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_generic_definer_members_use_source_value_purpose() {
    let source = r"snit::type T {method p\uD800 {} {}; method p\uD801 {} {}}";
    let result = Analyser::new().analyse(source, "tcl8.5");
    let class = result
        .original_class_declarations()
        .next()
        .expect("original generic definer publication");
    let members = class
        .metadata()
        .original_members
        .methods(MemberSide::Instance)
        .expect("Registry-selected source metadata purpose");
    assert_eq!(members.len(), 2);
    assert!(members.iter().all(|member| member.name_purpose()
        == tcl_registry::definer::DefinitionMemberNamePurpose::SourceValue));
    assert_ne!(
        members[0].original_name_input().bytes(),
        members[1].original_name_input().bytes()
    );
}

#[test]
fn original_readonly_class_configuration_joins_actual_name_value() {
    let source = r"oo::class create C\uD800 {method base {} {}}; set target C\uD800; oo::define $target {method added {} {}}";
    let result = Analyser::new().analyse(source, "tcl9.1");
    let declaration = result.original_class_declarations().next().unwrap();
    let configuration = result
        .original_class_configurations()
        .next()
        .expect("exact readonly target producer and actual class allocation");
    assert!(
        configuration
            .target()
            .name_input()
            .original_word_key()
            .is_none()
    );
    assert_eq!(
        configuration.target().name_input().bytes(),
        b"C\xed\xa0\x80"
    );
    assert_eq!(
        configuration.target().local_allocation().unwrap().site,
        *declaration.declaration_site()
    );
    assert_eq!(
        declaration
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn original_class_source_inventory_keeps_later_opaque_relation_declarations() {
    let source = "oo::class create B\\uD800 {}\noo::class create B\\uD801 {}\noo::class create C {superclass B\\uD800; mixin B\\uD801}\n";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let declarations = result.original_class_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 3, "{dialect}");
        assert_eq!(declarations[0].name_input().bytes(), b"B\xed\xa0\x80");
        assert_eq!(declarations[1].name_input().bytes(), b"B\xed\xa0\x81");
        assert_eq!(declarations[2].name_input().bytes(), b"C");
        let relations = declarations[2]
            .metadata()
            .original_relations
            .effects()
            .flat_map(|effect| effect.values().unwrap().iter())
            .collect::<Vec<_>>();
        assert_eq!(relations.len(), 2, "{dialect}");
        assert_ne!(
            relations[0].name_input().bytes(),
            relations[1].name_input().bytes()
        );
    }
}

#[test]
fn original_classmethod_source_inventory_is_independent_of_instance_ancestry() {
    let source = "oo::class create B {classmethod shared {} {}}\noo::class create C {superclass B; classmethod shared {} {}}\n";
    let result = Analyser::new().analyse(source, "tcl9.0");
    let declarations = result.original_class_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    for declaration in declarations {
        let methods = declaration
            .metadata()
            .original_members
            .methods(MemberSide::ClassObject)
            .expect("source-only original classmethod own table");
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].original_name_input().bytes(), b"shared");
        assert!(methods[0].native_class_delegate());
    }
}

#[test]
fn original_static_generic_class_names_retain_source_bytes_without_entered_handler() {
    let source = r"snit::type T\uD800 {method base {} {}}; snit::type T\uD801 {method other {} {}}";
    let result = Analyser::new().analyse(source, "tcl8.5");
    let declarations = result.original_class_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].name_input().bytes(), b"T\xed\xa0\x80");
    assert_eq!(declarations[1].name_input().bytes(), b"T\xed\xa0\x81");
    assert_ne!(declarations[0].name().slot(), declarations[1].name().slot());
    for declaration in declarations {
        assert_eq!(
            declaration.name_input().source_image().bytes(),
            source.as_bytes()
        );
        assert_eq!(
            declaration
                .metadata()
                .original_members
                .methods(MemberSide::Instance)
                .unwrap()
                .len(),
            1
        );
    }
    let computed = Analyser::new().analyse("snit::type $name {method p {} {}}", "tcl8.5");
    assert_eq!(computed.original_class_declarations().count(), 0);
}
