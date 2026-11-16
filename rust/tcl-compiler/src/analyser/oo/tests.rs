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
use crate::analyser::types::MemberRetractionRecord;

#[test]
fn original_receiver_declaration_keeps_its_whole_parent_bom_configuration() {
    // naming.tcloo.original-source-receiver-body-declaration
    // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
    // Source declaration body only; no allocated class or entered method.
    fn bodies(scope: &Scope) -> Vec<&super::super::types::OriginalSourceReceiverBodyDeclaration> {
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
    let outer = crate::segmenter::segment_commands_with_offset_and_config(
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
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
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
    let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
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

/// The `TclOO` definition-body grammar the `apply_oo_subcommand` /
/// `extract_method_def` helpers read their member argument layout from —
/// the same `&'static` the analyser fetches from the registry at runtime.
fn tcloo() -> &'static DefinitionBodyGrammar {
    &tcl_registry::definer::TCLOO_GRAMMAR
}

fn class() -> ClassDef {
    ClassDef {
        name: "C".to_string(),
        qualified_name: "::C".to_string(),
        name_span: tcl_lexer::Span::new(0, 0),
        body_span: tcl_lexer::Span::new(0, 0),
        ..Default::default()
    }
}

/// The `W315` messages an analysis produced, in emission order — the
/// "this class definition cannot run" reports.
fn w315_messages(r: &super::super::types::AnalysisResult) -> Vec<String> {
    r.diagnostics
        .iter()
        .filter(|d| d.code == tcl_core_types::DiagCode::W315)
        .map(|d| d.message.clone())
        .collect()
}

fn has_w315(r: &super::super::types::AnalysisResult) -> bool {
    !w315_messages(r).is_empty()
}

fn tok(span: (u32, u32)) -> Token {
    Token::new(TokenType::Esc, tcl_lexer::Span::new(span.0, span.1))
}

fn str_tok(span: (u32, u32)) -> Token {
    Token {
        kind: TokenType::Str,
        span: tcl_lexer::Span::new(span.0, span.1),
        content_offset: 1,
        in_quote: false,
    }
}

#[test]
fn superclass_subcommand_assigns_supers() {
    let mut cd = class();
    let texts: Vec<String> = ["superclass", "::A", "::B"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 10)), tok((11, 14)), tok((15, 18))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.superclasses, vec!["::A", "::B"]);
}

#[test]
fn mixin_subcommand_strips_dash_flags() {
    let mut cd = class();
    let texts: Vec<String> = ["mixin", "-append", "::M1", "::M2"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 5)), tok((6, 13)), tok((14, 18)), tok((19, 23))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.mixins, vec!["::M1", "::M2"]);
}

/// Apply a sequence of definition-body words to one class — the shape
/// slot folding is about: later words must fold into,
/// not overwrite, earlier slot state.
fn apply_words(cd: &mut super::ClassDef, calls: &[&[&str]]) {
    for words in calls {
        let texts: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
        let argv: Vec<tcl_lexer::Token> = words.iter().map(|_| tok((0, 1))).collect();
        apply_oo_subcommand(tcloo(), &texts, &argv, cd);
    }
}

// Slot semantics for filter / superclass / mixin / variable (defaults
// pinned against tclOODefineCmds.c 9.0.4 and confirmed live on
// tclsh 9.0.4).

// TP: `filter a ; filter b` appends — `info class filters` → `a b`,
// NOT `b` (which is what a last-writer-wins reading would give).
#[test]
fn filter_slot_appends_across_calls() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "b"]]);
    assert_eq!(cd.filters, vec!["a", "b"]);
}

// TP: the explicit operations fold — `-set` replaces, `-clear` empties,
// `-prepend` front-inserts (tclsh 9.0.4: `filter a; filter -prepend b`
// → `b a`), `-remove` deletes.
#[test]
fn filter_slot_explicit_operations_fold() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a", "b"], &["filter", "-set", "x"]]);
    assert_eq!(cd.filters, vec!["x"]);
    apply_words(&mut cd, &[&["filter", "-clear"]]);
    assert_eq!(cd.filters, [] as [std::string::String; 0]);
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "-prepend", "b"]]);
    assert_eq!(cd.filters, vec!["b", "a"]);
    apply_words(&mut cd, &[&["filter", "-remove", "b"]]);
    assert_eq!(cd.filters, vec!["a"]);
}

// FP guard: an operation word is recognised at argument 0 only —
// `filter a -set b` appends three literal items (tclsh 9.0.4), it does
// not replace the slot with `b`.
#[test]
fn filter_slot_op_word_only_recognised_first() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a", "-set", "b"]]);
    assert_eq!(cd.filters, vec!["a", "-set", "b"]);
}

// TP: `superclass` defaults to `-set` (replace) — the common single
// declaration keeps its meaning — and `-append` extends (tclsh 9.0.4:
// `superclass A` after `superclass B` → `::B`; `superclass -append A`
// → `::B ::A`).
#[test]
fn superclass_slot_replaces_by_default_and_appends_explicitly() {
    let mut cd = class();
    apply_words(&mut cd, &[&["superclass", "::A"], &["superclass", "::B"]]);
    assert_eq!(cd.superclasses, vec!["::B"]);
    apply_words(&mut cd, &[&["superclass", "-append", "::A"]]);
    assert_eq!(cd.superclasses, vec!["::B", "::A"]);
}

// TP: `mixin` defaults to `-set` in 8.6 and 9.0 alike (both C sources
// forward `--default-operation` to `-set`; tclsh 9.0.4:
// `mixin M1; mixin M2` → `::M2`).
#[test]
fn mixin_slot_replaces_by_default() {
    let mut cd = class();
    apply_words(&mut cd, &[&["mixin", "::M1"], &["mixin", "::M2"]]);
    assert_eq!(cd.mixins, vec!["::M2"]);
}

// TP: the class `variable` slot appends with dedup (tclsh 9.0.4:
// `variable a; variable a b` → `a b`) and folds `-set` / `-remove`.
#[test]
fn variable_slot_appends_dedups_and_folds_ops() {
    let mut cd = class();
    apply_words(&mut cd, &[&["variable", "a"], &["variable", "a", "b"]]);
    assert_eq!(cd.variables, vec!["a", "b"]);
    apply_words(&mut cd, &[&["variable", "-remove", "a"]]);
    assert_eq!(cd.variables, vec!["b"]);
    apply_words(&mut cd, &[&["variable", "-set", "c"]]);
    assert_eq!(cd.variables, vec!["c"]);
}

// TN: an unknown leading `-op` aborts the whole definition in real Tcl
// (`unknown method "-bogus"`), so the fold must leave the slot state
// unchanged rather than record `-bogus`/`x` as filters.
#[test]
fn unknown_slot_op_leaves_slot_untouched() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "-bogus", "x"]]);
    assert_eq!(cd.filters, vec!["a"]);
}

// TP, full pipeline: the fold carries across definition blocks — the
// oracle transcript verbatim (tclsh 9.0.4 and 8.6.16, byte-identical):
//   oo::class create C { filter a; filter b } → info class filters → a b
//   oo::define C { filter -set x }            →                    → x
//   oo::define C { filter -clear }            →                    → (empty)
#[test]
fn filter_slot_folds_across_oo_define_blocks() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -set x }\n",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.filters, vec!["x"]);

    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -append c }\n",
        "tcl9.0",
    );
    let c2 = r2.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c2.filters, vec!["a", "b", "c"]);
}

// The class-object side folds through the same slot spec: `self filter`
// appends across calls, and never leaks into the instance slot.
#[test]
fn class_side_filter_slot_folds_and_stays_sided() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { filter a }\nself { filter b } }\n",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_filters, vec!["a", "b"]);
    assert_eq!(c.filters, [] as [std::string::String; 0]);
}

#[test]
fn method_subcommand_records_method_def() {
    let mut cd = class();
    let texts: Vec<String> = ["method", "greet", "name", "puts $name"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 6)), tok((7, 12)), tok((13, 17)), str_tok((18, 32))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("greet"));
    let md = &cd.methods["greet"];
    assert_eq!(md.kind, "method");
    assert_eq!(md.visibility, "public");
}

/// Tcl 9's declaration flag sits between a method name and parameter list.
/// The analyser receives only the generic registry layout and its selected
/// visibility—not a command-specific option parser.
#[test]
fn method_visibility_option_shifts_params_and_sets_visibility() {
    let mut cd = class();
    let texts: Vec<String> = ["method", "internal", "-private", "arg", "puts $arg"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 6)),
        tok((7, 15)),
        tok((16, 24)),
        tok((25, 28)),
        str_tok((29, 38)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let method = cd.methods.get("internal").expect("method recorded");
    assert_eq!(method.visibility, "private");
    assert_eq!(method.params[0].name, "arg");
    assert_eq!(method.body_span, tcl_lexer::Span::new(29, 38));
}

#[test]
fn method_visibility_option_is_profile_gated_without_old_layout_reinterpretation() {
    let src = "oo::class create C { method internal -private {} { return ok } }";
    let mut analyser = Analyser::new();
    let tcl86 = analyser.analyse(src, "tcl8.6").clone();
    assert!(
        tcl86
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "W002"),
        "Tcl 8.6 must diagnose the Tcl 9 method option: {:?}",
        tcl86.diagnostics
    );
    assert!(
        tcl86
            .all_classes
            .get("::C")
            .is_none_or(|class| !class.methods.contains_key("internal")),
        "the invalid Tcl 8.6 member must not be reinterpreted using a shifted layout"
    );

    let mut analyser = Analyser::new();
    let tcl90 = analyser.analyse(src, "tcl9.0").clone();
    assert_eq!(
        tcl90.all_classes["::C"].methods["internal"].visibility,
        "private"
    );
}

#[test]
fn classmethod_subcommand_records_class_method() {
    let mut cd = class();
    let texts: Vec<String> = ["classmethod", "build", "args", "return $args"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 11)),
        tok((12, 17)),
        tok((18, 22)),
        str_tok((23, 38)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.class_methods.contains_key("build"));
    assert!(!cd.methods.contains_key("build"));
}

#[test]
fn constructor_appends_to_constructors_list() {
    let mut cd = class();
    let texts: Vec<String> = ["constructor", "args", "puts ctor"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 11)), tok((12, 16)), str_tok((17, 28))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.constructors.len(), 1);
    assert_eq!(cd.constructors[0].kind, "constructor");
    assert_eq!(cd.constructors[0].name, "<constructor>");
    // Name span anchors on the `constructor` keyword token
    // (argv[0] = 0..11), not the default (0, 0).
    assert_eq!(cd.constructors[0].name_span, tcl_lexer::Span::new(0, 11));
    // Constructors are not mirrored into the methods map.
    assert!(!cd.methods.contains_key("<constructor>"));
}

#[test]
fn destructor_sets_destructor_field() {
    let mut cd = class();
    let texts: Vec<String> = ["destructor", "puts dtor"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 10)), str_tok((11, 22))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let dtor = cd.destructor.as_ref().expect("destructor recorded");
    assert_eq!(dtor.kind, "destructor");
    assert_eq!(dtor.name, "<destructor>");
    // Name span anchors on the `destructor` keyword token
    // (argv[0] = 0..10).
    assert_eq!(dtor.name_span, tcl_lexer::Span::new(0, 10));
    assert!(!cd.methods.contains_key("<destructor>"));
}

#[test]
fn forward_records_method_with_forward_kind() {
    let mut cd = class();
    let texts: Vec<String> = ["forward", "delegate", "::other::cmd"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 7)), tok((8, 16)), tok((17, 29))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("delegate"));
    assert_eq!(cd.methods["delegate"].kind, "forward");
}

#[test]
fn private_method_records_with_private_visibility() {
    let mut cd = class();
    let texts: Vec<String> = ["private", "method", "internal", "args", "puts hi"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 7)),
        tok((8, 14)),
        tok((15, 23)),
        tok((24, 28)),
        str_tok((29, 37)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("internal"));
    assert_eq!(cd.methods["internal"].visibility, "private");
}

#[test]
fn self_method_subcommand_records_class_method_tagged_is_self_method() {
    // TP: `self method NAME ARGS BODY` (TclOO's own spelling of a
    // class-level method, the stock counterpart to ooutil's `classmethod`
    // keyword) must land by its row's type-object receiver, or
    // `class_methods` gains no entry for it. Recorded with
    // `kind: "classmethod"` (both
    // spellings mean "dispatched via the class's own bound command")
    // but tagged `is_self_method` so the class-command MRO walk knows
    // NOT to treat it as inherited the way ooutil's `classmethod` is.
    let mut cd = class();
    let texts: Vec<String> = ["self", "method", "make", "n", "return made"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 4)),
        tok((5, 11)),
        tok((12, 16)),
        tok((17, 18)),
        str_tok((19, 30)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(!cd.methods.contains_key("make"));
    let md = cd
        .class_methods
        .get("make")
        .expect("recorded as a classmethod");
    assert_eq!(md.kind, "classmethod");
    assert!(md.is_self_method);
    assert_eq!(md.visibility, "public");
}

#[test]
fn self_classmethod_subcommand_also_records_into_class_methods() {
    // TP — the `self classmethod` inner spelling (rarer, but valid:
    // `self` always retargets to the class object regardless of which
    // inner member follows) must resolve identically to `self method`.
    let mut cd = class();
    let texts: Vec<String> = ["self", "classmethod", "build", "args", "return $args"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 4)),
        tok((5, 16)),
        tok((17, 22)),
        tok((23, 27)),
        str_tok((28, 43)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let md = cd
        .class_methods
        .get("build")
        .expect("recorded as a classmethod");
    assert!(md.is_self_method);
}

#[test]
fn self_block_form_is_a_silent_noop_not_a_crash() {
    // FN guard — this helper does not read the `self { method NAME ARGS
    // BODY }` *block* form (nor the symmetric `private` one). It must
    // decline cleanly, never panic on the whole braced blob standing in
    // for an inner keyword.
    let mut cd = class();
    let texts: Vec<String> = ["self", "{ method make {n} { return made } }"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 4)), str_tok((5, 42))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.class_methods.len(), 0);
    assert_eq!(cd.methods.len(), 0);
}

#[test]
fn unrecognised_subcommand_is_silent_noop() {
    let mut cd = class();
    let texts: Vec<String> = ["whatever", "x"].iter().map(|s| (*s).to_string()).collect();
    let argv = [tok((0, 8)), tok((9, 10))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    // No fields populated; no panic.
    assert_eq!(cd.methods.len(), 0);
    assert_eq!(cd.superclasses, [] as [std::string::String; 0]);
    assert_eq!(cd.mixins, [] as [std::string::String; 0]);
}

#[test]
fn variable_declaration_spans_use_selected_original_literal_name_operands() {
    let source = "variable {$literal} {${braced}} {}; variable $computed; variable {*}{shifted}";
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let image = tcl_lexer::SourceImage::document(source);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let known = [
        "$literal",
        "${braced}",
        "",
        "computed",
        "$computed",
        "shifted",
    ]
    .map(str::to_owned);
    let spans = collect_var_decl_spans(tcloo(), &commands, &known, &image, config, None);
    for name in ["$literal", "${braced}", ""] {
        assert!(spans.contains_key(name), "{name:?}: {spans:?}");
    }
    assert!(!spans.contains_key("literal") && !spans.contains_key("braced"));
    assert!(
        !spans.contains_key("computed")
            && !spans.contains_key("$computed")
            && !spans.contains_key("shifted")
    );
}

#[test]
fn variable_subcommand_records_class_variables() {
    let mut cd = class();
    let texts: Vec<String> = ["variable", "x", "y"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 8)), tok((9, 10)), tok((11, 12))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.variables, vec!["x", "y"]);
}

#[test]
fn a_second_variable_subcommand_accumulates_rather_than_replacing() {
    // TP — the real corpus shape
    // (georgtree_tclopt's ::tclopt::Mpfit) has TWO separate `variable`
    // statements in the same class body (`variable funct m ftol ...`
    // then, separately, `variable Pars`). tclsh9.0-verified: both
    // statements' names are live, simultaneous instance variables —
    // `variable` inside a class body is additive, never a reset (the
    // same "always present in every method" declaration a `variable`
    // command inside a method body itself would make, just issued
    // once for the whole class). A second statement must not silently
    // discard the first's names.
    let mut cd = class();
    apply_oo_subcommand(
        tcloo(),
        &["variable".to_string(), "funct".to_string(), "m".to_string()],
        &[tok((0, 8)), tok((9, 14)), tok((15, 16))],
        &mut cd,
    );
    apply_oo_subcommand(
        tcloo(),
        &["variable".to_string(), "Pars".to_string()],
        &[tok((20, 28)), tok((29, 33))],
        &mut cd,
    );
    assert_eq!(cd.variables, vec!["funct", "m", "Pars"]);
}

#[test]
fn filter_subcommand_records_filters() {
    let mut cd = class();
    let texts: Vec<String> = ["filter", "log", "trace"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 6)), tok((7, 10)), tok((11, 16))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.filters, vec!["log", "trace"]);
    // …and only the instance slot: the class object stays unfiltered.
    assert_eq!(cd.class_filters.len(), 0, "{:?}", cd.class_filters);
}

#[test]
fn export_and_unexport_record_sets() {
    let mut cd = class();
    let texts1: Vec<String> = ["export", "foo", "bar"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv1 = [tok((0, 6)), tok((7, 10)), tok((11, 14))];
    apply_oo_subcommand(tcloo(), &texts1, &argv1, &mut cd);
    assert!(cd.exports.contains("foo"));
    assert!(cd.exports.contains("bar"));

    let texts2: Vec<String> = ["unexport", "baz"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv2 = [tok((0, 8)), tok((9, 12))];
    apply_oo_subcommand(tcloo(), &texts2, &argv2, &mut cd);
    assert!(cd.unexports.contains("baz"));
}

#[test]
fn property_subcommand_records_property_def() {
    let mut cd = class();
    let texts: Vec<String> = [
        "property",
        "colour",
        "-kind",
        "readable",
        "-get",
        "return red",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let argv = [
        tok((0, 8)),
        tok((9, 15)),
        tok((16, 21)),
        tok((22, 30)),
        tok((31, 35)),
        str_tok((36, 47)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let pd = cd.properties.get("colour").expect("colour recorded");
    assert_eq!(pd.kind, "readable");
    assert!(pd.has_getter);
    assert!(!pd.has_setter);
}

/// `property` is a 9.0 `TclOO` member: under 8.6 it is flagged
/// disabled-in-dialect (W002), but from 9.0 on it is accepted silently.
///
/// Either way the member is **recorded** — reporting "this needs Tcl
/// 9.0" must not also break go-to-definition, references, rename, and
/// document symbols over the code the user wrote, and the whole-command
/// W002 behaves the same way (it reports a dialect-unavailable command
/// while the analyser goes on modelling the call).
#[test]
fn property_member_is_gated_to_9_0() {
    let src = "oo::class create C {\n    property color\n}\n";
    let mut a86 = Analyser::new();
    let r86 = a86.analyse(src, "tcl8.6");
    assert!(
        r86.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W002),
        "property should be W002 under 8.6",
    );
    assert!(
        r86.all_classes
            .get("::C")
            .is_some_and(|c| c.properties.contains_key("color")),
        "the member is still recorded, so editor features keep working",
    );

    let mut a90 = Analyser::new();
    let r90 = a90.analyse(src, "tcl9.0");
    assert!(
        !r90.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W002),
        "property is available in 9.0",
    );
    assert!(
        r90.all_classes
            .get("::C")
            .is_some_and(|c| c.properties.contains_key("color")),
        "9.0 records the property",
    );
}

/// A configurable class answers `configure` for its properties, so that
/// generated accessor is folded into its known methods even though no
/// `method` body defines it.  `cget` is **not** generated
/// (`configurable.n` documents none in 9.0 or 9.1), so it must not be.
#[test]
fn configurable_class_knows_configure_but_not_cget() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "oo::configurable create C {\n    property color\n}\n",
            "tcl9.0",
        )
        .clone();
    let known = r.class_hierarchy().known_methods(
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
        "::C",
    );
    assert!(known.contains(&"configure".to_owned()), "{known:?}");
    assert!(!known.contains(&"cget".to_owned()), "{known:?}");
}

/// A command reference the analyser records for a class-body member
/// argument (`superclass`/`mixin`/`inherit`/`forward` target).  `written`
/// is the token text; `resolved` the qualified command it denotes.
fn has_cmd_ref(r: &crate::analyser::types::AnalysisResult, written: &str, resolved: &str) -> bool {
    r.command_invocations
        .iter()
        .any(|inv| inv.name == written && inv.resolved_qualified_name.as_deref() == Some(resolved))
}

// TP: a `superclass ::ns::Base` names the base class — recorded as a
// command reference so references / go-to-definition / rename reach it.
#[test]
fn superclass_records_a_command_reference_to_the_base_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass ::ns::Base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "superclass must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TP: `mixin ::ns::Role` — the mixed-in class is a command reference.
#[test]
fn mixin_records_a_command_reference_to_the_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Role {}\n  \
                 oo::class create C { mixin ::ns::Role }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "::ns::Role", "::ns::Role"));
}

// The `forward` target resolves through the member grammar's
// `ArgRole::CommandName` and records the delegated command.
#[test]
fn forward_target_still_records_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  proc helper {} {}\n  \
                 oo::class create C { forward f ::ns::helper }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "::ns::helper", "::ns::helper"));
}

// TP: a bare `superclass Base` (same namespace) resolves to the enclosing
// namespace's class — the one-hop call-site resolution, not an ancestor.
#[test]
fn bare_superclass_resolves_in_the_class_namespace() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass Base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "Base", "::ns::Base"));
}

// TP (itcl): `inherit ::ns::Base` names a base class the same way.
#[test]
fn itcl_inherit_records_a_command_reference_to_the_base_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "itcl::class ::ns::Base {}\n\
                 itcl::class ::ns::Sub { inherit ::ns::Base }\n",
            "tcl8.6",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "itcl inherit must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TN: a dynamic base name (`superclass $base`) names no static command —
// no reference is recorded (nothing to navigate or rename).
#[test]
fn dynamic_superclass_records_no_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  \
                 oo::class create Sub { superclass $base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        !r.command_invocations
            .iter()
            .any(|inv| inv.name.contains('$')),
        "a dynamic superclass name must not be recorded as a reference"
    );
}

// FP guard: `mixin -append ::ns::M` — the `-append` flag is not a class,
// only `::ns::M` is a reference.
#[test]
fn mixin_append_flag_is_not_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create M {}\n  \
                 oo::class create C { mixin -append ::ns::M }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::M", "::ns::M"),
        "the class is a reference"
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|inv| inv.name == "-append"),
        "the -append flag must not be recorded as a command reference"
    );
}

// TP: the inline `oo::define Sub superclass Base` form (no `{body}` block)
// records the base-class reference too — the same as the braced body form.
// (The inline path is separate from the body walk.)
#[test]
fn inline_oo_define_superclass_records_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub {}\n  oo::define Sub superclass ::ns::Base\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "inline `oo::define … superclass` must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TP: a `forward` target may legally begin with `-` (a command named
// `-foo`).  The `-`-flag filter applies only to class-list members
// (`mixin -append`), never to the forward command name.
#[test]
fn forward_target_beginning_with_dash_is_recorded() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create C { forward f -foo }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        r.command_invocations.iter().any(|inv| inv.name == "-foo"),
        "a hyphen-prefixed forward target must be recorded: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

#[test]
fn property_subcommand_with_no_kind_defaults_to_readwrite() {
    let mut cd = class();
    let texts: Vec<String> = ["property", "x"].iter().map(|s| (*s).to_string()).collect();
    let argv = [tok((0, 8)), tok((9, 10))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let pd = cd.properties.get("x").expect("x recorded");
    assert_eq!(pd.kind, "readwrite");
    assert!(!pd.has_getter);
    assert!(!pd.has_setter);
}

#[test]
fn property_subcommand_records_multiple_names() {
    let mut cd = class();
    let texts: Vec<String> = ["property", "x", "y", "-kind", "writable"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 8)),
        tok((9, 10)),
        tok((11, 12)),
        tok((13, 18)),
        tok((19, 27)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.properties.len(), 2);
    assert_eq!(cd.properties["x"].kind, "writable");
    assert_eq!(cd.properties["y"].kind, "writable");
}

#[test]
fn extract_method_def_too_few_args_returns_none() {
    // ``method`` with only 1 arg (just the name) — needs 3 (name, params,
    // body), so the grammar's `Body` word at index 2 is absent.
    let member = tcloo().member("method").expect("method is a member");
    let args: Vec<String> = vec!["foo".to_string()];
    let arg_tokens: Vec<Token> = vec![tok((0, 3))];
    let md = extract_method_def(member, &args, &arg_tokens, "method", "public", "");
    assert!(md.is_none());
}

fn param(name: &str) -> ParamDef {
    ParamDef {
        name: name.to_string(),
        has_default: false,
        default_value: None,
    }
}

/// Depth coverage: `walk_unknown_stmt` recurses once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame`
/// body, with no depth cap of its own. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH`
/// (256) by the lowering pass today, so this is defence-in-depth /
/// consistency with every other full-tree walker in this crate, not a
/// currently-reproducible crash. 1000 levels of *source* nesting is
/// comfortably past this new cap; the assertion is that
/// `extract_unknown_proc_info` returns at all, not what it returns.
/// Spawns its own big-stack thread since the lexer/CST/segmenter
/// stages upstream of `walk_unknown_stmt`'s own new cap still walk the
/// full un-truncated source nesting before lowering's cap trims it —
/// same rationale as `structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn extract_unknown_proc_info_survives_deep_nesting() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut body = String::new();
    for _ in 0..DEPTH {
        body.push_str("if {1} {\n");
    }
    body.push_str("exec ls\n");
    for _ in 0..DEPTH {
        body.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let mut a = Analyser::new();
            let _ = a.extract_unknown_proc_info(&body, &[param("cmd"), param("args")]);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn extract_unknown_proc_info_empty_body_marks_empty_stub() {
    let mut a = Analyser::new();
    let info = a.extract_unknown_proc_info("", &[param("cmd"), param("args")]);
    assert!(info.empty_stub);
    assert_eq!(info.dispatch_targets.len(), 0);
}

#[test]
fn extract_unknown_proc_info_whitespace_body_marks_empty_stub() {
    let mut a = Analyser::new();
    let info = a.extract_unknown_proc_info("   \n  \t ", &[param("cmd"), param("args")]);
    assert!(info.empty_stub);
}

fn logical_unknown_input(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> super::super::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    )
}

fn logical_unknown_analyser() -> Analyser {
    let input = logical_unknown_input(tcl_registry::model::ingress::context_for_profile(
        tcl_dialect::DialectProfile::plain_tcl(),
    ));
    assert!(input.has_logical_source_name_context());
    Analyser::new().with_resolved_input(input)
}

#[test]
fn original_unknown_source_context_requires_retained_logical_input_and_current_registry() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let body = "switch -exact $cmd {foo {return 1}}";
    let mut missing = Analyser::new();
    assert!(
        missing
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let options =
        crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap();
    let context = input.context_registry();
    let mut body_builder =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config());
    body_builder.set_source_analysis_options(options);
    let lowered = body_builder.lower(body);
    let statement = lowered.top_level.statements.first().unwrap();
    let binding = lowered
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap()
        .source_binding
        .as_ref()
        .unwrap();
    assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
    assert!(matches!(
        statement,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
    // The possible source bodies do not settle the unknown option branch.
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let case =
        crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
            lowered
                .top_level
                .retained_source_tokens_for_statement(statement)
                .unwrap(),
            context.commands(),
            metadata,
        )
        .unwrap();
    assert!(case.body_operands().selection_unknown);
    let mut logical = logical_unknown_analyser();
    assert!(
        logical
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .contains("foo")
    );
    let mut logical = logical_unknown_analyser();
    // Retired analyser registry labels cannot replace the retained input.
    logical.registry = Some(
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry(),
    );
    assert!(
        logical
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .contains("foo")
    );
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        let profile = environment.unit_profile();
        let input = super::super::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        assert!(!input.has_logical_source_name_context(), "{dialect}");
        let mut native = Analyser::new().with_resolved_input(input);
        assert!(
            native
                .extract_unknown_proc_info(body, &[param("cmd")])
                .dispatch_targets
                .is_empty(),
            "{dialect}"
        );
    }
}

#[test]
fn original_unknown_case_advice_retains_residuals_in_final_info() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Possible source dispatch keeps its selection/error and no-match
    // paths; it supplies no Native occupancy, chosen effect or body entry.
    let mut analyser = logical_unknown_analyser();
    let result = analyser.analyse(
        "proc unknown {cmd args} {switch -exact $cmd {café {return 1} default {return 0}}}",
        "tcl",
    );
    let info = result
        .unknown_proc_info
        .expect("original unknown declaration");
    assert!(info.dispatch_targets.contains("café"));
    assert!(!info.dispatch_targets.contains("default"));
    assert!(!info.has_pattern_dispatch);
    let [residual] = info.case_body_residuals.as_slice() else {
        panic!("the final AnalysisResult must retain its canonical case residual")
    };
    assert!(residual.selection_unknown);
    assert!(!residual.no_match_possible);
    assert_eq!(residual.bodies.len(), 2);
    assert_eq!(residual.patterns.len(), 2);

    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let closed = "switch -exact -- $cmd {foo {return 1} default {return 0}}";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(closed).top_level.clone();
    assert!(matches!(
        script.statements.first(),
        Some(Statement::Switch { .. })
    ));
    let mut analyser = Analyser::new().with_resolved_input(input);
    let info = analyser.extract_unknown_proc_info(closed, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("foo"));
    assert_eq!(info.case_body_residuals.len(), 1);
    assert!(!info.case_body_residuals[0].selection_unknown);
    assert!(!info.case_body_residuals[0].no_match_possible);
}

#[test]
fn original_unknown_case_advice_keeps_captured_ordinals_and_literal_parameter_names() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let source = "interp alias {} ::choisiré {} switch -exact; ::choisiré $cmd {é {return 1} default {return 0}}";
    let mut analyser = logical_unknown_analyser();
    let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("é"));
    let [residual] = info.case_body_residuals.as_slice() else {
        panic!("the captured selector retains a case residual")
    };
    assert!(residual.selection_unknown);
    assert_eq!(residual.bodies[0].argument, 2);
    assert_eq!(residual.bodies[0].list_element, Some(1));
    assert_eq!(residual.patterns[0].argument, 2);
    assert_eq!(residual.patterns[0].list_element, Some(0));

    for source in [
        "switch -exact $cmdSuffix {foo {return 1}}",
        "switch -exact {$cmd} {foo {return 1}}",
        "switch -exact $cmd(index) {foo {return 1}}",
    ] {
        let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
        assert!(info.dispatch_targets.is_empty(), "{source}");
        assert!(info.case_body_residuals.is_empty(), "{source}");
    }
    let info = analyser.extract_unknown_proc_info(
        "switch -exact ${$cmd} {literal {return 1}}",
        &[param("$cmd")],
    );
    assert!(info.dispatch_targets.contains("literal"));
}

fn original_case_availability_contexts() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut spec = registry.get("switch").unwrap().clone();
    spec.name = "current_case";
    spec.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(spec);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(current.commands(), older.commands()));
    (current, older)
}

#[test]
fn original_unknown_case_advice_refuses_missing_foreign_changed_and_shadowed_owners() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    use crate::registry_invocation::{
        InvocationMetadataContext, source_structure::original_logical_source_case_for_tokens,
    };
    use std::sync::Arc;
    let (current, older) = original_case_availability_contexts();
    let input = logical_unknown_input(Arc::clone(&current));
    let profile = input.unit_profile();
    let config = input.lexer_config();
    let source = "interp alias {} selecté {} current_case -exact; selecté $cmd {foo {return 1}}";
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    let positive = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(positive.dispatch_targets.contains("foo"));
    assert!(positive.case_body_residuals[0].no_match_possible);
    let mut lowerer = crate::lowering::Lowerer::with_config(current.commands(), config)
        .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let metadata =
        InvocationMetadataContext::for_analysis_input(current.commands(), &input).unwrap();
    assert!(
        original_logical_source_case_for_tokens(tokens, current.commands(), metadata).is_some()
    );
    assert!(
        original_logical_source_case_for_tokens(
            tokens,
            current.commands(),
            current.as_ref().into()
        )
        .is_none()
    );
    let foreign = logical_unknown_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    let mut changed_config = config;
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed = super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        changed_config,
    );
    for input in [&foreign, &changed] {
        let metadata = InvocationMetadataContext::for_analysis_input(
            input.borrowed_context_registry().commands(),
            input,
        )
        .unwrap();
        assert!(
            original_logical_source_case_for_tokens(tokens, current.commands(), metadata).is_none()
        );
    }
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(
        original_logical_source_case_for_tokens(&missing, current.commands(), metadata).is_none()
    );
    let mut altered = tokens.clone();
    let crate::ir::WordExpr::Variable { spelling, .. } = &mut altered.word_exprs[1] else {
        panic!("original written subject")
    };
    *spelling = "$other".into();
    assert!(
        original_logical_source_case_for_tokens(&altered, current.commands(), metadata).is_none()
    );
    let mut absent = Analyser::new();
    assert!(
        absent
            .extract_unknown_proc_info(source, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    let mut unavailable = Analyser::new().with_resolved_input(logical_unknown_input(older));
    assert!(
        unavailable
            .extract_unknown_proc_info(source, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    for source in [
        "proc current_case args {}; interp alias {} selecté {} current_case -exact; selecté $cmd {foo {return 1}}",
        "interp alias {} selecté {} current_case -exact; proc current_case args {}; selecté $cmd {foo {return 1}}",
        "current_case $option $cmd {foo {return 1}}",
    ] {
        assert!(
            analyser
                .extract_unknown_proc_info(source, &[param("cmd")])
                .dispatch_targets
                .is_empty(),
            "{source}"
        );
    }
}

#[test]
fn original_unknown_case_advice_selects_the_original_case_fold_child() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let mut analyser = logical_unknown_analyser();
    let source =
        "interp alias {} foldé {} string tolower; switch -exact [foldé $cmd] {foo {return 1}}";
    let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.case_insensitive);
    for source in [
        "proc string args {}; switch -exact [string tolower $cmd] {foo {return 1}}",
        "switch -exact [list {string tolower $cmd}] {foo {return 1}}",
        "switch -exact [string tolower $other] {foo {return 1}}",
    ] {
        let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
        assert!(info.dispatch_targets.is_empty(), "{source}");
        assert!(!info.case_insensitive, "{source}");
    }
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let mut tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap()
        .clone();
    assert_ne!(
        tokens.nested_bindings,
        [] as [(u32, crate::command_binding::SourceInvocationBinding); 0]
    );
    tokens.nested_bindings.clear();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let case =
        crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
            &tokens,
            context.commands(),
            metadata,
        )
        .unwrap();
    assert!(!case.references_parameter("cmd"));
    assert!(!case.folds_case());
}

#[test]
fn original_unknown_native_case_advice_requires_typed_switch_and_original_input() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // This fixture retains actual registration and compiler entry. The
    // readonly case query grants no successful variable read or body entry.
    use crate::registry_invocation::{
        InvocationMetadataContext, InvocationMetadataInput,
        source_structure::original_native_source_case_for_statement,
    };
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = super::super::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        profile,
        context.clone(),
        config,
    );
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let options = crate::command_binding::SourceAnalysisOptions {
        metadata_context: InvocationMetadataInput::SuppliedSource(Some(&input)),
        native_entry: Some(&captured),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_dialect(Some(profile))
        .with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(options);
    let script = lowerer
        .lower("switch -exact -- $cmd {foo {return 1} default {return 0}}")
        .top_level
        .clone();
    let statement = script.statements.first().unwrap();
    assert!(matches!(statement, Statement::Switch { .. }));
    let metadata =
        InvocationMetadataContext::for_analysis_input(context.commands(), &input).unwrap();
    let case =
        original_native_source_case_for_statement(&script, statement, context.commands(), metadata)
            .expect("typed original Native case source");
    assert!(case.references_parameter("cmd"));
    assert_eq!(case.patterns(), &["foo"]);
    assert!(!case.body_operands().selection_unknown);
    assert!(!case.body_operands().no_match_possible);
    let mut info = UnknownProcInfo::default();
    walk_unknown_stmt(
        statement,
        &script,
        &context,
        Some(metadata),
        "cmd",
        &mut info,
        0,
    );
    assert!(info.dispatch_targets.contains("foo"));
    assert_eq!(info.case_body_residuals.len(), 1);
    assert!(
        original_native_source_case_for_statement(
            &script,
            statement,
            context.commands(),
            context.as_ref().into(),
        )
        .is_none()
    );
    let mut missing = script.clone();
    missing.command_binding_sites = crate::ir::CommandBindingSites::default();
    assert!(
        original_native_source_case_for_statement(
            &missing,
            &missing.statements[0],
            context.commands(),
            metadata,
        )
        .is_none()
    );
    let mut generic_lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input.clone());
    let generic = generic_lowerer
        .lower("switch -exact -- $cmd {foo {return 1} default {return 0}}")
        .top_level
        .clone();
    let generic_statement = generic.statements.first().unwrap();
    assert!(!matches!(generic_statement, Statement::Switch { .. }));
    assert!(
        original_native_source_case_for_statement(
            &generic,
            generic_statement,
            context.commands(),
            metadata,
        )
        .is_none()
    );
}

#[test]
fn extract_unknown_proc_info_exact_switch_collects_dispatch_targets() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact $cmd {
            foo { return 1 }
            bar { return 2 }
            default { return 0 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(!info.empty_stub);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.dispatch_targets.contains("bar"));
    assert!(!info.dispatch_targets.contains("default"));
    assert!(!info.has_pattern_dispatch);
}

#[test]
fn extract_unknown_proc_info_glob_switch_marks_pattern_dispatch() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -glob $cmd {
            foo* { return 1 }
            *bar { return 2 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_pattern_dispatch);
    assert_eq!(info.dispatch_targets.len(), 0);
}

#[test]
fn extract_unknown_proc_info_chains_original_via_known_target() {
    let mut a = logical_unknown_analyser();
    let body = r"rename unknown _original_unknown
_original_unknown $cmd $args";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.chains_original);
}

#[test]
fn unknown_handler_effects_require_the_live_implementation() {
    for (body, flag) in [
        ("proc exec args {}; exec $cmd", "exec"),
        ("proc auto_load args {}; auto_load $cmd", "load"),
        ("_original_unknown $cmd", "chain"),
        ("if {$condition} {rename exec {}}; exec $cmd", "exec"),
    ] {
        let mut analyser = logical_unknown_analyser();
        let info = analyser.extract_unknown_proc_info(body, &[param("cmd")]);
        let claimed = match flag {
            "exec" => info.has_exec,
            "load" => info.has_auto_load,
            _ => info.chains_original,
        };
        assert!(
            !claimed,
            "unproved implementation acquired builtin effects: {body}"
        );
    }
    let mut analyser = Analyser::new();
    let result = analyser.analyse(
        "proc exec args {}; proc unknown {cmd args} {exec $cmd}",
        "tcl8.6",
    );
    assert!(!result.unknown_proc_info.as_ref().unwrap().has_exec);
}

#[test]
fn unknown_handler_effects_follow_resolved_alias_arguments() {
    let mut analyser = logical_unknown_analyser();
    let info = analyser.extract_unknown_proc_info(
        "interp alias {} ensure {} package require; ensure Tcl",
        &[param("cmd")],
    );
    assert!(info.has_auto_load);
}

#[test]
fn original_unknown_operation_advice_keeps_actual_availability_and_captured_sources() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Operation descriptions are conditional source advice, without an
    // executed load, Native handler, cell, frame or Normal certificate.
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut loader = registry.get("package").unwrap().clone();
    loader.name = "context_package";
    loader.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    let mut subcommands = loader.subcommands.to_vec();
    subcommands
        .iter_mut()
        .find(|subcommand| subcommand.name == "require")
        .unwrap()
        .surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    loader.subcommands = Box::leak(subcommands.into_boxed_slice());
    registry.insert(loader);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(current.commands(), older.commands()));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        |context| super::super::ResolvedAnalysisInput::new(profile, profile, context, config);
    let source = "interp alias {} ::chargeré {} context_package require; ::chargeré Tcl";
    let mut analyser = Analyser::new().with_resolved_input(input(Arc::clone(&current)));
    assert!(
        analyser
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    analyser.registry = Some(
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry(),
    );
    assert!(
        analyser
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    let mut unavailable = Analyser::new().with_resolved_input(input(older));
    assert!(
        !unavailable
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    let replaced = "proc context_package args {}; interp alias {} ensure {} context_package require; ensure Tcl";
    assert!(
        !analyser
            .extract_unknown_proc_info(replaced, &[param("cmd")])
            .has_auto_load
    );
}

fn assert_captured_logical_source_target(
    tokens: &crate::ir::CommandTokens,
    context: &tcl_registry::model::ContextRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) {
    let input = metadata.source_analysis_input().unwrap();
    let original = tokens.source_binding.as_ref().unwrap();
    let target = original
        .proved_execution_target()
        .expect("closed Logical source target");
    assert!(target.registry_backed);
    let dialect = original
        .original_source_word_dialect_for_tokens(tokens)
        .expect("actual Logical source dialect");
    assert!(dialect.native_name_protocol().is_none());
    assert!(
        crate::command_binding::SourceAnalysisOptions::for_logical_source(input)
            .unwrap()
            .native_entry
            .is_none()
    );
    let selected =
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            context.commands(),
            metadata,
            tokens,
        )
        .unwrap();
    assert!(
        selected
            .with_metadata_schema(
                context.commands(),
                metadata,
                original.invocation_realm().unwrap(),
                |schema| {
                    let target_spec = context.context().resolve_spec_in_realm(
                        context.commands(),
                        &target.command,
                        original.invocation_realm().unwrap(),
                    )?;
                    Some(std::ptr::eq(
                        target_spec,
                        schema.authored_source_descriptors().command,
                    ))
                },
            )
            .unwrap()
    );
    assert_eq!(
        selected.evaluated_words[0].literal_bytes(),
        Some(b"require".as_slice())
    );
    assert_eq!(
        selected.effective.origins[1],
        crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0)
    );
    assert_eq!(selected.effective.written_argument(1), Some(0));
}

#[test]
fn original_unknown_operation_advice_refuses_missing_foreign_changed_and_dynamic_source() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    use crate::registry_invocation::InvocationMetadataContext;
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let statement = script.statements.last().unwrap();
    let original = statement.tokens().unwrap().source_binding.as_ref().unwrap();
    assert!(original.original_recorded_head_name_input().is_none());
    let metadata = InvocationMetadataContext::for_source_input(
        context.commands(),
        &input,
        input.lexer_config(),
        Some(input.unit_profile()),
    )
    .unwrap();
    assert_captured_logical_source_target(statement.tokens().unwrap(), &context, metadata);
    let inspect = |statement: &Statement, metadata| {
        let mut info = UnknownProcInfo::default();
        walk_unknown_stmt(statement, &script, &context, metadata, "cmd", &mut info, 0);
        info
    };
    assert!(inspect(statement, Some(metadata)).has_auto_load);
    let foreign = super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    for metadata in [
        None,
        Some(context.as_ref().into()),
        InvocationMetadataContext::for_analysis_input(
            foreign.borrowed_context_registry().commands(),
            &foreign,
        ),
        InvocationMetadataContext::for_analysis_input(context.commands(), &changed),
    ] {
        assert!(!inspect(statement, metadata).has_auto_load);
    }
    let mut missing = statement.clone();
    missing.tokens_mut().unwrap().source_binding = None;
    assert!(!inspect(&missing, Some(metadata)).has_auto_load);
    let mut analyser = logical_unknown_analyser();
    for source in [
        "interp alias {} ensure {} package $selector; ensure Tcl",
        "interp alias {} ensure {} package require; $unknown Tcl",
        "interp alias {} ensure {} package require; proc package args {}; ensure Tcl",
    ] {
        assert!(
            !analyser
                .extract_unknown_proc_info(source, &[param("cmd")])
                .has_auto_load,
            "{source}"
        );
    }
    assert!(
        !Analyser::new()
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
}

#[test]
fn original_unknown_operation_purpose_requires_the_complete_original_source_input() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let statement = script.statements.last().unwrap();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let mut selected = UnknownProcInfo::default();
    walk_unknown_stmt(
        statement,
        &script,
        &context,
        Some(metadata),
        "cmd",
        &mut selected,
        0,
    );
    assert!(selected.has_auto_load);
    let native_profile =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let native_input = super::super::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        std::sync::Arc::clone(&context),
        input.lexer_config(),
    );
    assert!(!native_input.has_logical_source_name_context());
    for metadata in [
        None,
        Some(context.as_ref().into()),
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            context.commands(),
            &native_input,
        ),
    ] {
        let mut refused = UnknownProcInfo::default();
        walk_unknown_stmt(
            statement,
            &script,
            &context,
            metadata,
            "cmd",
            &mut refused,
            0,
        );
        assert!(!refused.has_auto_load);
        assert!(!refused.has_exec);
        assert!(!refused.chains_original);
        assert!(refused.dispatch_targets.is_empty());
    }
}

#[test]
fn extract_unknown_proc_info_detects_exec_call() {
    let mut a = logical_unknown_analyser();
    let body = r"exec $cmd {*}$args";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_exec);
}

#[test]
fn extract_unknown_proc_info_detects_auto_load_call() {
    let mut a = logical_unknown_analyser();
    let body = r"auto_load $cmd";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_auto_load);
}

#[test]
fn extract_unknown_proc_info_case_insensitive_via_string_tolower() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact [string tolower $cmd] {
            foo { return 1 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.case_insensitive);
    assert!(info.dispatch_targets.contains("foo"));
}

#[test]
fn extract_unknown_proc_info_no_first_param_defaults_to_cmd() {
    // Empty params list — the helper should fall back to
    // ``"cmd"`` as the dispatch-subject variable name.
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact $cmd { foo { return 1 } }";
    let info = a.extract_unknown_proc_info(body, &[]);
    assert!(info.dispatch_targets.contains("foo"));
}

// The `self { … }` / `private { … }` block forms.

#[test]
fn self_block_form_records_every_member_as_a_class_method() {
    // TP. Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C {
    //       self { method make {n} {…} ; method other {} {…} }
    //       method inst {} {…}
    //   }
    //   ::C make 7               -> made-7
    //   info object methods ::C  -> other make   (class-object side)
    //   info class methods ::C   -> inst         (instance side)
    //   oo::class create ::F {superclass ::C} ; ::F make 1
    //                            -> error: unknown method "make"
    // i.e. exactly what the `self method …` prefix form declares — which
    // is why both spellings land on the same recording path.
    let src = "oo::class create ::C {\n\
                   self {\n\
                   method make {n} { return $n }\n\
                   method other {} { return other }\n\
                   }\n\
                   method inst {} { return inst }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    for name in ["make", "other"] {
        let md = c
            .class_methods
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` must record as a class method"));
        assert_eq!(md.kind, "classmethod");
        assert!(
            md.is_self_method,
            "`self`-scoped members are not inherited by subclasses",
        );
        assert_eq!(md.visibility, "public");
    }
    // The instance side is untouched — `inst` stays an instance method and
    // the block's members never leak into it.
    assert!(c.methods.contains_key("inst"));
    assert!(!c.methods.contains_key("make"));
    assert!(!c.methods.contains_key("other"));
}

#[test]
fn self_block_form_records_through_oo_define_too() {
    // TP. Same block, reached through `oo::define` rather than the class
    // creation body (oracle: `oo::define ::D { self { method mk {} {…} } }`
    // → `info object methods ::D` is `mk`).
    let src = "oo::class create ::D {}\n\
                   oo::define ::D {\n\
                   self {\n\
                   method mk {} { return dmk }\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::D").expect("::D recorded");
    assert!(c.class_methods.contains_key("mk"));
}

#[test]
fn self_block_scoped_unexport_flips_the_class_side() {
    // TP. Oracle: `oo::class create ::E { self { method
    // hidden {} {…} ; unexport hidden } }` leaves `info object methods
    // ::E` empty while `-all -private` still lists `hidden`, and
    // `::E hidden` errors "unknown method" — so the block's `unexport`
    // really does apply to the class-object side. The member is recorded
    // *and* now carries the visibility the block gave it.
    let src = "oo::class create ::E {\n\
                   self {\n\
                   method hidden {} { return h }\n\
                   unexport hidden\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E").expect("::E recorded");
    assert!(c.class_methods.contains_key("hidden"));
    assert_eq!(c.class_methods["hidden"].visibility, "unexported");
}

#[test]
fn self_scoped_unexport_leaves_a_same_named_instance_method_alone() {
    // TN — block and prefix spellings both.
    // Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    //   oo::class create C { method m {} {return inst-m}
    //                        self { method m {} {return class-m}
    //                               unexport m } }
    //   info object methods ::C  ->            (class side unexported)
    //   info class methods ::C   -> m          (instance side untouched)
    //   ::C m                    -> unknown method "m"
    //   [::C new] m              -> inst-m
    for src in [
        "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self {\n\
             method m {} { return class }\n\
             unexport m\n\
             }\n\
             }",
        "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self method m {} { return class }\n\
             self unexport m\n\
             }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(
            c.class_methods["m"].visibility, "unexported",
            "class side must be flipped",
        );
        assert_eq!(
            c.methods["m"].visibility, "public",
            "instance side must NOT be flipped by a `self`-scoped unexport",
        );
        assert!(
            !c.unexports.contains("m"),
            "the class-level `unexports` set is the instance-side record; a \
                 class-object-side unexport must not enter it",
        );
    }
}

#[test]
fn unwrapped_unexport_leaves_the_class_side_alone() {
    // TN, the mirror direction. Oracle:
    //   oo::class create E2 { self { method onlyclass {} {…} } }
    //   oo::define E2 { unexport onlyclass }
    //   info object methods ::E2 -> onlyclass   (still exported)
    //   ::E2 onlyclass           -> oc          (still dispatches)
    // The unwrapped word acts on the instance side, where the name does
    // not exist — a silent no-op in real Tcl.
    let src = "oo::class create ::E2 {\n\
                   self { method onlyclass {} { return oc } }\n\
                   }\n\
                   oo::define ::E2 { unexport onlyclass }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E2").expect("::E2 recorded");
    assert_eq!(c.class_methods["onlyclass"].visibility, "public");
}

#[test]
fn cross_side_unexport_is_a_silent_no_op_not_an_error() {
    // TN / oracle-shape pin. Unlike `deletemethod` (a hard error that
    // aborts the definition), naming the *other* side's method in an
    // `export`/`unexport` is accepted and does nothing:
    //   oo::class create E { method onlyinst {} {…} }
    //   oo::define E { self unexport onlyinst }   ;# no error
    //   info class methods ::E  -> onlyinst       (still exported)
    //   [::E new] onlyinst      -> oi
    // so we record no visibility change and — crucially — no member stub.
    let src = "oo::class create ::E3 { method onlyinst {} { return oi } }\n\
                   oo::define ::E3 { self unexport onlyinst }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::E3").expect("::E3 recorded");
    assert_eq!(c.methods["onlyinst"].visibility, "public");
    assert!(!c.class_methods.contains_key("onlyinst"));
}

#[test]
fn self_block_export_reexports_a_class_side_member() {
    // TP, the export half — last explicit state wins.
    // Oracle: `oo::class create H1 { self { method lower {} {…} ;
    // unexport lower ; export lower } }` -> `info object methods ::H1`
    // is `lower` on 9.0.4 and 8.6.14 alike.
    let src = "oo::class create ::H1 {\n\
                   self {\n\
                   method lower {} { return l }\n\
                   unexport lower\n\
                   export lower\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::H1").expect("::H1 recorded");
    assert_eq!(c.class_methods["lower"].visibility, "public");
}

#[test]
fn private_scoped_visibility_members_flip_the_instance_side() {
    // TP, the third routing. `private` is 9.0-only (`invalid command
    // name "private"` on 8.6), and under it a visibility word acts on the
    // instance side exactly as the unwrapped spelling does. Oracle:
    //   oo::class create G1 { method m {} {…}; private { unexport m } }
    //   info class methods ::G1              ->            (unexported)
    //   info class methods ::G1 -all -private -> … m …
    //   oo::class create G3 { method m {} {…}; private export m }
    //   info class methods ::G3              -> m          (exported)
    for (src, want) in [
        (
            "oo::class create ::G1 {\n\
                 method m {} { return m }\n\
                 private { unexport m }\n\
                 }",
            "unexported",
        ),
        (
            "oo::class create ::G2 {\n\
                 method m {} { return m }\n\
                 private unexport m\n\
                 }",
            "unexported",
        ),
        (
            "oo::class create ::G3 {\n\
                 method m {} { return m }\n\
                 private export m\n\
                 }",
            "public",
        ),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r
            .all_classes
            .values()
            .find(|c| c.methods.contains_key("m"))
            .expect("class recorded");
        assert_eq!(c.methods["m"].visibility, want, "{src}");
    }
}

#[test]
fn private_block_form_records_instance_methods() {
    // TP, symmetric half — `private` is the other registry member marked
    // wrapper-with-block-body.
    let src = "oo::class create ::P {\n\
                   private {\n\
                   method secret {k} { return $k }\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::P").expect("::P recorded");
    assert_eq!(c.methods["secret"].visibility, "private");
    assert!(!c.class_methods.contains_key("secret"));
}

#[test]
fn self_block_does_not_declare_instance_variables() {
    // TN / abstention boundary. `self { variable x }` declares a variable
    // on the *class object*, not an instance variable of the class — the
    // prefix form (`self variable x`) is not recorded either, and the
    // block form must not start recording it as one.
    let src = "oo::class create ::V {\n\
                   self {\n\
                   variable classlevel\n\
                   }\n\
                   variable instancelevel\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::V").expect("::V recorded");
    assert_eq!(c.variables, vec!["instancelevel".to_string()]);
}

#[test]
fn self_introspection_prefix_is_not_a_block_member() {
    // TN. Inside a *method body*, `self class` is an introspection call,
    // not a definer member. The body is walked as a script (never as a
    // definition body), so nothing here may reach `class_methods`.
    let src = "oo::class create ::W {\n\
                   method whoami {} { return [self class] }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::W").expect("::W recorded");
    assert!(c.methods.contains_key("whoami"));
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
}

#[test]
fn dynamic_self_block_abstains() {
    // Abstention boundary: a non-literal block word cannot be re-segmented
    // statically, so the expansion declines and the class records nothing
    // rather than guessing.
    let src = "set b {method make {n} { return $n }}\n\
                   oo::class create ::Dyn {\n\
                   self $b\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::Dyn").expect("::Dyn recorded");
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
    assert_eq!(c.methods.len(), 0, "{:?}", c.methods);
}

#[test]
fn self_block_destructive_member_drops_the_member_it_deletes() {
    // TP. Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C1 {
    //       self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
    //   }
    //   info object methods ::C1  ->  kept
    //   ::C1 gone                 ->  unknown method "gone"
    // Retaining `gone` would show a stale document symbol and let
    // navigation resolve a name the interpreter does not have.
    let src = "oo::class create ::C {\n                   self {\n                   method gone {} { return g }\n                   method kept {} { return k }\n                   deletemethod gone\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(
        !c.class_methods.contains_key("gone"),
        "{:?}",
        c.class_methods
    );
    assert!(c.class_methods.contains_key("kept"));
}

#[test]
fn self_block_renamemethod_moves_the_member_on_the_class_side() {
    // TP, the class-side half. Oracle, byte-identical on tclsh
    // 9.0.4 and 8.6.14:
    //   oo::class create ::R2 { self { method old {} {return CLSOLD}
    //                                  renamemethod old new } }
    //   info object methods ::R2  -> new
    //   ::R2 new                  -> CLSOLD   (shadowing the stock `new`!)
    //   ::R2 old                  -> unknown method
    // The move stays on the wrapper's own side: the instance table is not
    // touched in either direction.
    let src = "oo::class create ::C {\n                   self {\n                   method old {} { return o }\n                   renamemethod old new\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(
        !c.class_methods.contains_key("old"),
        "{:?}",
        c.class_methods
    );
    let md = c.class_methods.get("new").expect("`new` is a class member");
    assert_eq!(md.name, "new");
    assert!(src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),);
    assert_eq!(
        c.methods.len(),
        0,
        "instance side untouched: {:?}",
        c.methods
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn private_block_destructive_member_drops_the_instance_side_member() {
    // TP, symmetric half. Oracle (9.0 — `private` is a 9.0 member):
    //   oo::class create ::C5 {
    //       private { method secret {} {…} ; method other {} {…} ;
    //                 deletemethod secret }
    //   }
    //   info class methods ::C5 -all -private   ->  no `secret`
    let src = "oo::class create ::C {\n                   private {\n                   method secret {} { return s }\n                   method other {} { return o }\n                   deletemethod secret\n                   }\n                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("secret"), "{:?}", c.methods);
    assert!(c.methods.contains_key("other"));
}

#[test]
fn wrapper_block_and_prefix_forms_retract_identically() {
    // The block form is normalised into the prefix form, so both spellings
    // must land on the same result. Oracle for the prefix side:
    //   oo::define ::C7 { self method sgone {} {…} }
    //   info object methods ::C7                  ->  sgone
    //   oo::define ::C7 { self deletemethod sgone }
    //   info object methods ::C7                  ->  (empty)
    let src = "oo::class create ::C {}\n                   oo::define ::C { self method gone {} { return g } }\n                   oo::define ::C { self deletemethod gone }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_methods.len(), 0, "{:?}", c.class_methods);
}

#[test]
fn a_retraction_does_not_cross_to_the_other_side() {
    // TN. `self deletemethod` touches only the class-object side. Oracle:
    //   oo::class create ::D { method m {} {return inst}
    //                          self { method m {} {return cls} } }
    //   oo::define ::D { self deletemethod m }
    //   info class methods ::D   ->  m        (instance side survives)
    //   info object methods ::D  ->  (empty)
    //   [::D new] m              ->  inst
    let src = "oo::class create ::D {\n                   method m {} { return inst }\n                   self { method m {} { return cls } }\n                   }\n                   oo::define ::D { self deletemethod m }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::D").expect("::D recorded");
    assert!(c.methods.contains_key("m"), "instance side must survive");
    assert!(!c.class_methods.contains_key("m"), "class side must go");
}

#[test]
fn non_retracting_method_reference_members_keep_their_member() {
    // TN, the discrimination the registry flag exists for: `export`,
    // `unexport`, and `filter` are the sibling `MemberRefKind::Method`
    // members — they *name* a method without removing it, so they must not
    // retract. Oracle:
    //   oo::class create ::A { self { method a {} {…} ; unexport a ; export a } }
    //   info object methods ::A  ->  a   ;  ::A a  ->  1
    //   oo::class create ::B { self { method f {} {…} ; filter f } }
    //   info object methods ::B  ->  f
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::A {\n self {\n method a {} { return 1 }\n unexport a\n export a\n }\n}",
        "tcl9.0",
    );
    assert!(
        r.all_classes["::A"].class_methods.contains_key("a"),
        "export/unexport must not retract",
    );
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::B {\n self {\n method f {} { return 1 }\n filter f\n }\n}",
        "tcl9.0",
    );
    assert!(
        r.all_classes["::B"].class_methods.contains_key("f"),
        "filter must not retract",
    );
}

#[test]
fn a_retracted_member_can_be_redeclared_by_a_later_body() {
    // Source-order guard. Oracle:
    //   oo::class create ::C { method m {} {return 1} }
    //   oo::define ::C { self method m {} {return 2} }
    //   oo::define ::C { self deletemethod m }
    //   oo::define ::C { self method m {} {return 3} }
    //   ::C m        ->  3     (class side, redeclared after the delete)
    //   [::C new] m  ->  1     (instance side, never touched)
    // Retraction happens where the body is walked, so a later declaration
    // wins — it is not a whole-document erasure of the name.
    let src = "oo::class create ::C { method m {} { return 1 } }\n                   oo::define ::C { self method m {} { return 2 } }\n                   oo::define ::C { self deletemethod m }\n                   oo::define ::C { self method m {} { return 3 } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_methods.contains_key("m"), "redeclaration must win");
    assert!(c.methods.contains_key("m"), "instance side untouched");
}

#[test]
fn unwrapped_deletemethod_drops_the_instance_side_member() {
    // TP. An *unwrapped* `deletemethod` — in a class-creation
    // body or an `oo::define` body, with no `self` / `private` wrapper —
    // deletes on the instance side. Oracle, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::I1 { method gone {} {…}; method kept {} {…}
    //                           deletemethod gone }
    //   info class methods ::I1  ->  kept
    //   oo::class create ::I4 { method gone {} {…} }
    //   oo::define ::I4 { deletemethod gone }
    //   info class methods ::I4  ->            (empty)
    for src in [
        "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             deletemethod gone\n\
             }",
        "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             }\n\
             oo::define ::C { deletemethod gone }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("gone"), "deleted member retained");
        assert!(c.methods.contains_key("kept"), "sibling member dropped");
    }
}

#[test]
fn unwrapped_renamemethod_moves_the_member_to_the_new_name() {
    // TP. Oracle,
    // byte-identical on tclsh 9.0.4 and 8.6.14:
    //   oo::class create ::I3 { method old {} {return o}; renamemethod old new }
    //   info class methods ::I3        ->  new
    //   [::I3 new] new                 ->  o        (the old body runs)
    //   info class definition ::I3 new ->  {} { return o }
    //   ::I3 old                       ->  unknown method
    // so `new` is a fully navigable member carrying `old`'s body: `old` goes,
    // `new` arrives with the *same* `MethodDef`, its name span moved onto
    // the `renamemethod` call's destination word (the only place the new
    // name is written) and its body span left on the one real body.
    let mut a = Analyser::new();
    let src = "oo::class create ::C { method old {} { return o }\n\
             renamemethod old new }";
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("old"));
    let md = c.methods.get("new").expect("`new` is a member");
    assert_eq!(md.name, "new");
    // The name span is the `renamemethod`'s destination word …
    assert_eq!(
        &src[md.name_span.start() as usize..md.name_span.end() as usize],
        "new"
    );
    // … and it is the *second* `new` in the source (the one in the
    // `renamemethod` call), not any earlier text.
    assert!(md.name_span.start() as usize > src.find("renamemethod").expect("call"));
    // … while the body span still points at the original body.
    assert!(
        src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),
        "body span must stay on the original body",
    );
    // A rename is not a (re)declaration, so the family's name-based export
    // default is not re-applied — the source's visibility travels with it.
    assert_eq!(md.visibility, "public");
    // TN: a rename onto a fresh name is a legal order.
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    // …and the *destination* name leaves no cross-document tombstone: the
    // rename creates `new`, it does not delete it, so another document's
    // `method new` must survive. Registry data decides which arguments a
    // retracting word removes (`MemberRetraction::FirstArgument`).
    assert!(
        !c.retracted_members
            .iter()
            .any(|r| r.member == "new" || r.member == "old"),
        "a locally-declared retraction needs no tombstone: {:?}",
        c.retracted_members,
    );
}

// The class-side visibility + filter channels.

#[test]
fn self_scoped_visibility_words_fill_the_class_side_sets() {
    // TP. A `self export` / `self unexport` has to leave a
    // record of its own or the flip never travels to another file's
    // class-command dispatch. Oracle, byte-identical on tclsh 9.0.4/8.6.14:
    //   oo::class create ::X { self { method cm {} { return cm } } }
    //   ::X cm                    ;# -> cm
    //   oo::define X { self unexport cm }
    //   ::X cm                    ;# -> unknown method "cm": must be create,
    //                             ;#    destroy or new
    //   info object methods ::X   ;# -> (empty)
    //   info object methods ::X -all -private ;# -> … cm …  (still defined)
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { method cm {} { return c } } }\n\
             oo::define ::C { self unexport cm }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_unexports.contains("cm"), "{:?}", c.class_unexports);
    assert_eq!(c.class_exports.len(), 0, "{:?}", c.class_exports);
    assert_eq!(c.class_methods["cm"].visibility, "unexported");
    // …and the instance-side pair — the one every existing consumer reads
    // as the instance record — stays untouched in both directions.
    assert_eq!(c.exports.len(), 0, "{:?}", c.exports);
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
}

#[test]
fn a_rename_carries_the_sources_visibility_not_the_new_names_default() {
    // CRITICAL pin. A rename is not a (re)declaration, so the
    // family's name-based export default (`[a-z]*` is exported for `TclOO`)
    // is NOT re-applied to the destination — the source's own visibility
    // travels with the body. Both directions, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::R4 { method Priv {} {…} ; renamemethod Priv pub }
    //   info class methods ::R4           ;# -> (empty)   `pub` is UNexported
    //   info class methods ::R4 -private  ;# -> pub       despite the name
    //   [::R4 new] pub                    ;# -> unknown method "pub"
    //   oo::class create ::R5 { method low {} {…} ; renamemethod low Up }
    //   info class methods ::R5           ;# -> Up        still EXPORTED
    //   info class methods ::R5 -private  ;# -> Up        despite the name
    //
    // Applying the destination's default instead would flip both of these
    // the wrong way — offering a `pub` no `$obj pub` can call, and hiding an
    // `Up` that dispatches fine.
    for (src_name, dst_name, want) in [("Priv", "pub", "unexported"), ("low", "Up", "public")] {
        let src = format!(
            "oo::class create ::C {{ method {src_name} {{}} {{ return 1 }}\n\
                 renamemethod {src_name} {dst_name} }}"
        );
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(
            c.methods[dst_name].visibility, want,
            "{src_name} -> {dst_name}",
        );
    }
}

#[test]
fn the_class_side_visibility_sets_keep_only_the_last_writer() {
    // The class-side pair takes the identical last-writer-exclusive rule as
    // the instance pair, because the workspace consumer reads any `exports`
    // entry as decisive. Oracle: `oo::class create ::H1 { self { method
    // lower {} {…} ; unexport lower ; export lower } }` leaves
    // `info object methods ::H1` -> lower on 9.0.4 and 8.6.14 alike.
    for (body, exported) in [
        ("unexport m\nexport m", true),
        ("export m\nunexport m", false),
    ] {
        let src =
            format!("oo::class create ::C {{ self {{ method m {{}} {{ return 1 }}\n{body} }} }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        let (winner, loser) = if exported {
            (&c.class_exports, &c.class_unexports)
        } else {
            (&c.class_unexports, &c.class_exports)
        };
        assert!(winner.contains("m"), "{body}");
        assert!(
            !loser.contains("m"),
            "{body}: superseded set still holds `m`"
        );
    }
}

#[test]
fn an_unwrapped_visibility_word_never_touches_the_class_side_sets() {
    // TN, the mirror of `a_self_scoped_visibility_word_never_touches_the_
    // instance_sets`. The unwrapped spelling is instance-scoped, so an
    // identically-named class-side member keeps its own state — oracle:
    // `oo::class create E2 { self { method onlyclass {} {…} } }` then
    // `oo::define E2 { unexport onlyclass }` leaves `onlyclass` exported and
    // dispatchable via `::E2 onlyclass` on 9.0.4 and 8.6.14.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { method m {} { return c } } }\n\
             oo::define ::C { unexport m }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_exports.len(), 0, "{:?}", c.class_exports);
    assert_eq!(c.class_unexports.len(), 0, "{:?}", c.class_unexports);
    assert_eq!(c.class_methods["m"].visibility, "public");
    // The unwrapped word still records its own (instance-side) intent.
    assert!(c.unexports.contains("m"));
}

#[test]
fn filters_are_sided_by_the_wrapper_they_are_written_under() {
    // TP + TN — `filters` is sided like every other member table. The two
    // slots are genuinely independent in `TclOO` and
    // intercept different dispatches. Oracle, byte-identical on tclsh 9.0.4
    // and 8.6.14:
    //   oo::class create ::A { method real {} {…} ; method logit {args} {…}
    //                          filter logit }
    //   info class filters ::A   ;# -> logit      info object filters ::A -> {}
    //   [::A new] real           ;# logit fires, `self target` -> `::A real`
    //   oo::class create ::B { method inst {} {…}
    //       self { method cls {} {…} ; method logit {args} {…} ; filter logit } }
    //   info object filters ::B  ;# -> logit      info class filters ::B  -> {}
    //   ::B cls                  ;# logit fires, `self target` -> `::B cls`
    //   ::B new                  ;# logit fires too — the constructor path
    //   [::B new] inst           ;# logit does NOT fire
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::A { method real {} { return r }\n\
             method logit {args} { next }\n\
             filter logit }\n\
             oo::class create ::B { method inst {} { return i }\n\
             self { method cls {} { return c }\n\
             method logit {args} { next }\n\
             filter logit } }",
        "tcl9.0",
    );
    let a_cls = r.all_classes.get("::A").expect("::A recorded");
    assert_eq!(a_cls.filters, vec!["logit"]);
    assert_eq!(a_cls.class_filters.len(), 0, "{:?}", a_cls.class_filters);
    let b_cls = r.all_classes.get("::B").expect("::B recorded");
    assert_eq!(b_cls.class_filters, vec!["logit"]);
    assert_eq!(b_cls.filters.len(), 0, "{:?}", b_cls.filters);
}

#[test]
fn a_private_scoped_filter_is_instance_side() {
    // TN for the third spelling: `private` is a *visibility* wrapper over
    // the instance side, not a third side, so its `filter` lands in the
    // instance slot. Oracle (9.0 only — `private` does not exist on 8.6,
    // where the same body fails with `invalid command name "private"`):
    //   oo::class create ::P { private { method s {} {} ; filter s } }
    //   info class filters ::P   ;# -> s
    //   info object filters ::P  ;# -> (empty)
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { private { method s {} { return s }\n\
             filter s } }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.filters, vec!["s"]);
    assert_eq!(c.class_filters.len(), 0, "{:?}", c.class_filters);
}

#[test]
fn per_object_visibility_lands_in_object_member_state() {
    // `oo::objdefine $o { unexport m }` really works — oracle, 9.0.4 and
    // 8.6.14 alike:
    //   oo::class create ::C { method m {} {…} } ; set o [::C new]
    //   oo::objdefine $o { unexport m } ; $o m
    //   ;# -> unknown method "m": must be destroy or n
    // The flip lands in the receiver binding's `ObjectMemberState`
    // — the durable per-object home — while the class itself stays
    // untouched, and the throwaway holder never reaches `all_classes`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.methods["m"].visibility, "public");
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
    assert!(!r.all_classes.contains_key("::@objdefine@::o"));
    let states = r
        .object_member_state
        .get("o")
        .expect("the receiver binding has member state");
    assert_eq!(states.len(), 1, "{states:?}");
    assert!(states[0].unexports.contains("m"), "{states:?}");
    assert_eq!(states[0].exports.len(), 0, "{states:?}");
}

#[test]
fn per_object_export_and_unexport_are_last_writer_exclusive() {
    // The per-object pair keeps the same last-writer-exclusive contract
    // as the class-side pairs: a later `export m` cancels the earlier
    // flip, across blocks of the same binding.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }\n\
             oo::objdefine $o { export m }",
        "tcl9.0",
    );
    let states = r.object_member_state.get("o").expect("state recorded");
    assert_eq!(states.len(), 1, "one binding: {states:?}");
    assert!(states[0].exports.contains("m"), "{states:?}");
    assert!(!states[0].unexports.contains("m"), "{states:?}");
}

#[test]
fn per_object_cross_block_retraction_folds_and_stays_silent() {
    // The cross-block hazard that kept W315 out of `oo::objdefine`:
    // a second block retracting what the first declared
    // is legal (tclsh 9.0.4 / 8.6.14 both accept it), so the seeded walk
    // must remove the member silently — no W315 — and the folded state
    // must drop it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method im {} { return i } }\n\
             oo::objdefine $o { deletemethod im }",
        "tcl9.0",
    );
    assert_eq!(w315_messages(&r).len(), 0, "{:?}", r.diagnostics);
    let states = r.object_member_state.get("o").expect("state recorded");
    assert_eq!(states.len(), 1, "{states:?}");
    assert_eq!(
        states[0].methods.len(),
        0,
        "the retraction must fold: {states:?}"
    );
}

#[test]
fn a_retraction_of_a_locally_declared_member_leaves_no_tombstone() {
    // TN for the cross-document channel. Within one document the member is
    // removed outright and the order is known, so exporting a tombstone
    // would wrongly cancel a *different* document's redeclaration of the
    // name — no ordering evidence supports that.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method gone {} { return g } }\n\
             oo::define ::C { deletemethod gone }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("gone"));
    assert_eq!(c.retracted_members.len(), 0, "{:?}", c.retracted_members);
}

// W315, "this class definition cannot run".

#[test]
fn w315_fires_for_every_definition_aborting_retraction() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for (body, want) in [
        (
            "deletemethod ghost\nmethod ghost {} { return g }",
            "this class definition cannot run: method \"ghost\" does not exist",
        ),
        (
            "self { method cm {} { return c } }\ndeletemethod cm",
            "this class definition cannot run: method \"cm\" does not exist",
        ),
        (
            "method a {} { return a }\nmethod b {} { return b }\nrenamemethod a b",
            "this class definition cannot run: method called \"b\" already exists",
        ),
        (
            "method a {} { return a }\nrenamemethod a a",
            "this class definition cannot run: cannot rename method to itself",
        ),
        (
            "method b {} { return b }\nrenamemethod ghost b",
            "this class definition cannot run: method \"ghost\" does not exist",
        ),
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(w315_messages(&r), vec![want.to_string()], "body: {body}");
        assert!(r.all_classes.contains_key("::C"), "body: {body}");
    }
}

#[test]
fn w315_fires_on_the_class_side_too() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method im {} { return i }\nself { deletemethod im }",
        "method im {} { return i }\nself { renamemethod im other }",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(
            w315_messages(&r),
            vec!["this class definition cannot run: method \"im\" does not exist".to_string()],
            "body: {body}",
        );
        assert!(r.all_classes["::C"].methods.contains_key("im"), "{body}");
    }
}

fn logical_object_abort_analyser() -> Analyser {
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "explicit-object-abort-advice",
        &[],
        "Logical object definition advice",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let input = super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    assert!(input.has_logical_source_name_context());
    Analyser::new().with_resolved_input(input)
}

fn injected_class_abort_candidates() -> ClassDef {
    ClassDef {
        definition_aborts: [
            DefinitionAbortKind::MissingMember,
            DefinitionAbortKind::DestinationExists,
            DefinitionAbortKind::RenameToItself,
        ]
        .into_iter()
        .map(|kind| DefinitionAbort {
            kind,
            member: "missing".to_owned(),
            span: Span::new(0, 1),
        })
        .collect(),
        retracted_members: vec![MemberRetractionRecord::deletion(
            "missing".to_owned(),
            MemberSide::Instance,
        )],
        ..Default::default()
    }
}

#[test]
fn original_class_abort_refuses_native_and_hosted_reporting_tables() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Positive injected candidates check admission, not runtime retractions.
    let source = "oo::class create C {deletemethod ghost; method ghost {} {}}";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-tmsh",
        "f5-iapps",
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, dialect);
        assert!(!has_w315(&analyser.result), "{dialect}");
        analyser.result.lexical_declaration_advice = true;
        analyser.result.dialect = "explicit-object-abort-advice".to_owned();
        analyser.result.original_vendor_source_names.clear();
        analyser.result.all_classes.clear();
        let mut class = injected_class_abort_candidates();
        assert_eq!(class.definition_aborts.len(), 3);
        assert!(!class.via_define);
        analyser.emit_w315_definition_cannot_run(&mut class);
        assert!(
            !has_w315(&analyser.result),
            "{dialect}: reports grant no Native closure"
        );
        assert_eq!(
            class.definition_aborts,
            [] as [crate::analyser::types::DefinitionAbort; 0]
        );
        assert_eq!(class.retracted_members.len(), 1);
    }
}

#[test]
fn original_class_abort_refuses_missing_retained_input() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    let mut analyser = Analyser::new();
    analyser.result.lexical_declaration_advice = true;
    analyser.result.dialect = "explicit-object-abort-advice".to_owned();
    assert!(analyser.result.resolved_input.is_none());
    let mut class = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut class);
    assert!(!has_w315(&analyser.result));
    assert_eq!(
        class.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(class.retracted_members.len(), 1);
}

#[test]
fn original_class_abort_keeps_retained_logical_advice_despite_report_mutations() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    let mut analyser = logical_object_abort_analyser();
    analyser.result = analyser.analyse("", "explicit-object-abort-advice");
    assert!(analyser.result.allows_retained_logical_declaration_advice());
    analyser.result.dialect = "tcl9.0".to_owned();
    analyser.result.all_classes.clear();
    analyser.result.command_invocations.clear();
    analyser.result.all_procs.clear();
    let mut class = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut class);
    assert_eq!(w315_messages(&analyser.result).len(), 3);
    assert_eq!(
        class.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(
        class.retracted_members,
        [] as [crate::analyser::types::MemberRetractionRecord; 0]
    );
    analyser.result.lexical_declaration_advice = false;
    let mut withdrawn = injected_class_abort_candidates();
    analyser.emit_w315_definition_cannot_run(&mut withdrawn);
    assert_eq!(w315_messages(&analyser.result).len(), 3);
    assert_eq!(
        withdrawn.definition_aborts,
        [] as [crate::analyser::types::DefinitionAbort; 0]
    );
    assert_eq!(withdrawn.retracted_members.len(), 1);
}

fn inject_object_abort_candidates(analyser: &mut Analyser) {
    analyser
        .result
        .instance_classes
        .insert("owned".to_owned(), "::C".to_owned());
    analyser
        .result
        .created_instance_commands
        .insert("owned".to_owned());
    analyser.result.object_methods.clear();
    analyser.objdefine_unresolved_receiver = false;
    for kind in [
        DefinitionAbortKind::MissingMember,
        DefinitionAbortKind::DestinationExists,
        DefinitionAbortKind::RenameToItself,
    ] {
        analyser
            .objdefine_abort_candidates
            .push(super::super::state::ObjdefineAbortCandidate {
                abort: DefinitionAbort {
                    kind,
                    member: "missing".to_owned(),
                    span: Span::new(0, 1),
                },
                receiver: "owned".to_owned(),
                prior_state_conditional: false,
            });
    }
}

#[test]
fn original_object_abort_refuses_native_and_hosted_reporting_tables() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    // Injected report candidates test admission, without asserting execution.
    let source =
        "oo::class create C {}\nset o [C new]\noo::objdefine $o {method a {} {}; renamemethod a a}";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-tmsh",
        "f5-iapps",
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, dialect);
        assert!(!has_w315(&analyser.result), "{dialect}");
        analyser.result.lexical_declaration_advice = true;
        analyser.result.dialect = "explicit-object-abort-advice".to_owned();
        inject_object_abort_candidates(&mut analyser);
        assert_eq!(analyser.objdefine_abort_candidates.len(), 3);
        analyser.flush_objdefine_abort_diagnostics();
        assert!(
            !has_w315(&analyser.result),
            "{dialect}: report maps grant no Native table"
        );
        assert_eq!(
            analyser.objdefine_abort_candidates,
            [] as [crate::analyser::state::ObjdefineAbortCandidate; 0]
        );
    }
}

#[test]
fn original_object_abort_refuses_missing_retained_input() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    let mut analyser = Analyser::new();
    analyser.result.lexical_declaration_advice = true;
    assert!(analyser.result.resolved_input.is_none());
    inject_object_abort_candidates(&mut analyser);
    analyser.flush_objdefine_abort_diagnostics();
    assert!(!has_w315(&analyser.result));
    assert_eq!(
        analyser.objdefine_abort_candidates,
        [] as [crate::analyser::state::ObjdefineAbortCandidate; 0]
    );
}

#[test]
fn original_object_abort_keeps_explicit_logical_advice_despite_report_label() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    let mut analyser = logical_object_abort_analyser();
    analyser.result = analyser.analyse("", "explicit-object-abort-advice");
    assert!(analyser.result.allows_lexical_declaration_advice());
    assert!(
        analyser
            .result
            .resolved_input
            .as_ref()
            .unwrap()
            .has_logical_source_name_context()
    );
    analyser.result.dialect = "tcl9.0".to_owned();
    inject_object_abort_candidates(&mut analyser);
    analyser.flush_objdefine_abort_diagnostics();
    assert_eq!(w315_messages(&analyser.result).len(), 3);
}

// Per-object abort reports use explicitly selected Logical source advice.
// The String table supplies no Native own-method presence or completeness.

#[test]
fn w315_fires_for_a_per_object_retraction_of_a_never_declared_member() {
    // Explicit Logical compatibility uses the authored object's own table,
    // independently of inherited class metadata. No Native absence is proved.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method ghost {} { return g } }\n\
             set o [::C new]\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: method \"ghost\" does not exist".to_string()],
    );
}

#[test]
fn w315_fires_for_a_per_object_rename_onto_an_existing_member() {
    // Explicit Logical source order retains a destination declared by an
    // earlier block of the same authored binding. It grants no Native table.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method a {} { return a }\n method b {} { return b } }\n\
             oo::objdefine $o { renamemethod a b }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: method called \"b\" already exists".to_string()],
    );
}

#[test]
fn w315_fires_for_a_per_object_rename_to_itself() {
    // The Logical model selects the same source and destination name;
    // the compatibility assertion does not grant a Native worker receipt.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::objdefine $o { method x {} { return x }\n renamemethod x x }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this object definition cannot run: cannot rename method to itself".to_string()],
    );
}

#[test]
fn per_object_w315_abstains_without_the_receivers_construction_in_view() {
    // Logical metadata for an extension lacks a closed authored receiver
    // table; a declaration in another file remains possible.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn per_object_w315_abstains_when_any_key_declares_the_member() {
    // An authored alias can refer to a member declared under another key.
    // Logical metadata with that possibility cannot assert its absence.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set p [::C new]\n\
             oo::objdefine $p { method ghost {} { return g } }\n\
             set o [::C new]\n\
             set o $p\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn per_object_w315_abstains_when_any_receiver_is_unresolved() {
    // An unknown authored receiver can affect any object's source table.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine [pick] { method ghost {} { return g } }\n\
             oo::objdefine $o { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn w315_stays_silent_for_every_legal_order() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method a {} {}\nmethod b {} {}\ndeletemethod a",
        "self { method a {} {}\nmethod b {} {}\ndeletemethod a }",
        "method a {} {}\nrenamemethod a fresh",
        "method a {} {}\nself { method b {} {} }\nrenamemethod a b",
        "method a {} {}\ndeletemethod a\nmethod a {} {}",
        "method a {} {}\nmethod b {} {}\ndeletemethod b\nrenamemethod a b",
        "method a {} {}\nrenamemethod a b\nrenamemethod b c",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
    }
}

#[test]
fn a_cross_side_visibility_word_is_not_a_w315() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method onlyinst {} {}\nself unexport onlyinst",
        "self { method onlyclass {} {} }\nunexport onlyclass",
        "export ghost",
        "self export ghost",
        "filter ghost",
        "self filter ghost",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
    }
}

#[test]
fn w315_abstains_on_a_dynamic_source_name() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method m {} {}\ndeletemethod $gone",
        "method m {} {}\nrenamemethod $old new",
        "method m {} {}\ndeletemethod [pick]",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
        assert!(
            r.all_classes["::C"].methods.contains_key("m"),
            "body: {body}: a dynamic retraction must not remove a member",
        );
        assert_eq!(
            r.all_classes["::C"].retracted_members.len(),
            0,
            "body: {body}: {:?}",
            r.all_classes["::C"].retracted_members,
        );
    }
}

#[test]
fn a_dynamic_rename_destination_still_retracts_the_source() {
    // TN for the *destination* half, which is not symmetric with the source.
    // Whatever `$n` turns out to be, `renamemethod m $n` definitely takes
    // `m` away — `::C m` is an unknown method afterwards on 9.0.4 and
    // 8.6.14 alike — so retracting the source is the faithful answer even
    // though the arrival name is unknowable. Only the arrival abstains, and
    // with no static destination there is nothing to check for a collision.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method m {} { return m }\n\
             method n {} { return n }\n\
             renamemethod m $n }",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(!c.methods.contains_key("m"), "{:?}", c.methods);
    assert!(
        c.methods.contains_key("n"),
        "the collision guess is not made"
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
}

#[test]
fn a_cross_file_stub_retraction_is_a_tombstone_not_a_w315() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }\n\
             oo::define ::E { renamemethod old new }",
        "explicit-object-abort-advice",
    );
    assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    assert_eq!(
        r.all_classes["::C"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "m".to_string(),
            MemberSide::Instance
        )],
    );
    assert!(r.all_classes["::C"].via_define);
}

#[test]
fn a_same_file_define_extending_a_local_class_is_not_a_stub() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec!["this class definition cannot run: method \"ghost\" does not exist".to_string()],
    );
    assert!(!r.all_classes["::C"].via_define);
    assert_eq!(
        r.all_classes["::C"].retracted_members.len(),
        0,
        "{:?}",
        r.all_classes["::C"].retracted_members,
    );
}

#[test]
fn a_move_records_the_member_state_it_ran_against() {
    // The moved member's declaration site is the
    // `renamemethod`'s destination word, so a *rename* of it rewrites that
    // word — and the gate that decides whether the result still runs needs
    // the table as it stood at the move. Captured with the source already
    // removed and the destination not yet inserted, which is exactly what
    // the interpreter reads at that point.
    let src = "oo::class create ::C { method old {} { return 1 }\n\
                   method sib {} { return 2 }\n\
                   renamemethod old new }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.renamed_members.len(), 1, "{:?}", c.renamed_members);
    let moved = &c.renamed_members[0];
    assert_eq!(moved.source, "old");
    assert_eq!(moved.destination, "new");
    assert_eq!(moved.side, MemberSide::Instance);
    assert_eq!(
        &src[moved.destination_span.start() as usize..moved.destination_span.end() as usize],
        "new",
    );
    // `sib` is live at the move; `old` is not (it was just retracted) and
    // `new` is not (it has not arrived yet).
    assert_eq!(moved.blocked, vec!["sib".to_string()]);
}

#[test]
fn the_move_record_answers_which_new_names_would_abort() {
    // The shared fold both consumers use: the walker asks it about the
    // destination actually written (that is how W315 is decided) and the
    // rename gate asks it about the name the user typed. Oracle, identical
    // on tclsh 9.0.4 and 8.6.14 — no class is created in either case:
    //   { method old {} {…} ; renamemethod old old }
    //        -> cannot rename method to itself
    //   { method old {} {…} ; method sib {} {…} ; renamemethod old sib }
    //        -> method called sib already exists
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             renamemethod old new }",
        "tcl9.0",
    );
    let moved = &r.all_classes["::C"].renamed_members[0];
    assert_eq!(
        moved.abort_if_renamed_to("old"),
        Some(DefinitionAbortKind::RenameToItself),
    );
    assert_eq!(
        moved.abort_if_renamed_to("sib"),
        Some(DefinitionAbortKind::DestinationExists),
    );
    // TN: a fresh name is fine, and so is the destination it already has.
    assert_eq!(moved.abort_if_renamed_to("fresh"), None);
    // The mirror direction, from the same record.
    assert!(moved.collides_with_renaming("sib", "new"));
    assert!(!moved.collides_with_renaming("sib", "other"));
}

#[test]
fn a_cross_side_name_is_not_in_the_move_snapshot() {
    // TN (CRITICAL). Collisions are side-local: `method old {} {…}` +
    // `self { method sib {} {…} }` + `renamemethod old sib` runs fine and
    // leaves `info class methods ::C1` -> sib (9.0.4 / 8.6.14). So a
    // class-side `sib` must not appear in an instance-side move's snapshot.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             self { method sib {} { return 2 } }\n\
             renamemethod old new }",
        "tcl9.0",
    );
    let moved = &r.all_classes["::C"].renamed_members[0];
    assert_eq!(moved.blocked.len(), 0, "{:?}", moved.blocked);
    assert_eq!(moved.abort_if_renamed_to("sib"), None);
}

#[test]
fn the_move_snapshot_honours_body_order_in_both_directions() {
    // TN ×2, both oracle-pinned legal on 9.0.4 and 8.6.14:
    //   { method old {} {} ; method sib {} {} ; deletemethod sib
    //     renamemethod old sib }                 -> info class methods -> sib
    //   { method old {} {} ; renamemethod old sib ; method sib {} {} }
    //                                            -> info class methods -> sib
    // so a name deleted *before* the move is free, and one declared
    // *after* it never collides with it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             deletemethod sib\n\
             renamemethod old new }",
        "tcl9.0",
    );
    assert_eq!(
        r.all_classes["::C"].renamed_members[0].abort_if_renamed_to("sib"),
        None,
        "a name deleted before the move is free",
    );
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::D { method old {} { return 1 }\n\
             renamemethod old new\n\
             method sib {} { return 2 } }",
        "tcl9.0",
    );
    assert_eq!(
        r.all_classes["::D"].renamed_members[0].abort_if_renamed_to("sib"),
        None,
        "a name declared after the move never collides with it",
    );
}

#[test]
fn a_rejected_rename_records_no_move() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    for body in [
        "method a {} {}\nrenamemethod a a",
        "method a {} {}\nmethod b {} {}\nrenamemethod a b",
    ] {
        let mut a = logical_object_abort_analyser();
        let r = a.analyse(
            &format!("oo::class create ::C {{ {body} }}"),
            "explicit-object-abort-advice",
        );
        assert_eq!(
            r.all_classes["::C"].renamed_members.len(),
            0,
            "body: {body}: {:?}",
            r.all_classes["::C"].renamed_members,
        );
        assert!(has_w315(&r), "body: {body}");
    }
}

#[test]
fn a_blocked_rename_keeps_both_members_navigable() {
    // Navigation-resilience pin for the aborting shapes. Real Tcl creates no
    // class at all, so there is no "right" answer to mirror — the useful one
    // keeps every declaration the author wrote reachable rather than letting
    // the wreckage take one with it.
    let src = "oo::class create ::C { method a {} { return a }\n\
                   method b {} { return b }\n\
                   renamemethod a b }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.methods.contains_key("a"), "source kept: {:?}", c.methods);
    let b = c.methods.get("b").expect("destination kept");
    assert!(
        src[b.body_span.start() as usize..b.body_span.end() as usize].contains("return b"),
        "the destination keeps its OWN body",
    );
}

#[test]
fn w315_is_reported_once_per_offending_word() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }\n\
             oo::define ::C { method extra {} { return e } }\n\
             oo::define ::C { deletemethod other }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        w315_messages(&r),
        vec![
            "this class definition cannot run: method \"ghost\" does not exist".to_string(),
            "this class definition cannot run: method \"other\" does not exist".to_string(),
        ],
    );
}

#[test]
fn w315_points_at_the_offending_word() {
    // naming.diagnostics.original-definition-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
    // Explicit Logical source-table compatibility; no Native worker/table claim.
    let src = "oo::class create ::C { deletemethod ghost }";
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(src, "explicit-object-abort-advice");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == tcl_core_types::DiagCode::W315)
        .expect("W315");
    assert_eq!(
        &src[d.span.start() as usize..d.span.end() as usize],
        "ghost"
    );
}

#[test]
fn a_stub_retraction_of_an_undeclared_member_leaves_a_tombstone() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Explicit Logical cross-document advisory channel; no Native class or retraction.
    // TP for the cross-document channel. This document
    // is the `oo::define ::C { deletemethod m }` half of a two-file program:
    // it has no local `m` to remove, so the removal has to travel as a
    // tombstone or the workspace keeps advertising a method that sourcing
    // this file deletes. The side travels with it — a `self`-scoped
    // retraction tombstones the class-object side.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        r.all_classes["::C"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "m".to_string(),
            MemberSide::Instance
        )],
    );
    assert_eq!(
        r.all_classes["::D"].retracted_members,
        vec![MemberRetractionRecord::deletion(
            "cm".to_string(),
            MemberSide::ClassObject
        )],
    );
}

#[test]
fn a_stub_renamemethod_tombstones_only_the_source_name() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Explicit Logical cross-document advisory channel; no Native move.
    // TN for the tombstone's shape: `renamemethod old new` removes `old` and
    // *creates* `new` (oracle: `info class methods ::I3` -> new, and
    // `[::I3 new] new` answers), so tombstoning `new` would suppress a live
    // member another document legitimately declares. Which arguments a
    // retracting word removes is registry data
    // ([`tcl_registry::definer::MemberRetraction::FirstArgument`]), not a
    // keyword match here.
    let mut a = logical_object_abort_analyser();
    let r = a.analyse(
        "oo::define ::C { renamemethod old new }",
        "explicit-object-abort-advice",
    );
    assert_eq!(
        r.all_classes["::C"]
            .retracted_members
            .iter()
            .map(|r| (r.member.clone(), r.side))
            .collect::<Vec<_>>(),
        vec![("old".to_string(), MemberSide::Instance)],
    );
    // …and the arrival travels with it: the stub has no
    // `MethodDef` to move, so the destination name is what lets the
    // workspace join re-key the defining file's record.
    assert_eq!(
        r.all_classes["::C"].retracted_members[0].arrival.as_deref(),
        Some("new"),
    );
}

#[test]
fn missing_class_source_cannot_publish_native_definition_stubs() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    let source = "oo::define ::C {deletemethod m}; oo::define ::D {renamemethod old new}";
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"] {
        let result = Analyser::new().analyse(source, profile);
        assert!(!result.all_classes.contains_key("::C"), "{profile}");
        assert!(!result.all_classes.contains_key("::D"), "{profile}");
    }
    let mut missing = logical_object_abort_analyser();
    let mut baseline = logical_object_abort_analyser();
    missing.result = baseline.analyse(source, "explicit-object-abort-advice");
    assert!(missing.result.all_classes.contains_key("::C"));
    missing.result.resolved_input = None;
    missing.result.all_classes.clear();
    missing.source = source.to_owned().into();
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        missing.lexer_config(),
    )
    .remove(0);
    assert!(missing.handle_oo_define_command(
        command.name(),
        command.args(),
        command.arg_tokens(),
        command.arg_single_token(),
        &[],
    ));
    assert!(missing.result.all_classes.is_empty());
}

#[test]
fn visibility_sets_keep_only_the_last_writer() {
    // `exports` / `unexports` each record the
    // *last* explicit writer for a name, and the cross-file consumer
    // (`workspace_index::method_dispatch_chain`) reads any `exports` entry
    // as decisive — so a name must never sit in both. Oracle, identical on
    // tclsh 9.0.4 and 8.6.14:
    //   oo::class create L1 { method m {} {…}; export m; unexport m }
    //   info class methods ::L1  ->            ;  [L1 new] m -> unknown method
    //   oo::class create L2 { method m {} {…}; unexport m; export m }
    //   info class methods ::L2  -> m          ;  [L2 new] m -> 1
    // Both unwrapped and `private`-scoped writers must leave each name
    // in exactly the set selected by its last explicit visibility operation.
    for (body, last, want_vis) in [
        ("export m\nunexport m", "unexports", "unexported"),
        ("unexport m\nexport m", "exports", "public"),
        (
            "private export m\nprivate unexport m",
            "unexports",
            "unexported",
        ),
        ("private unexport m\nprivate export m", "exports", "public"),
        (
            "private { export m }\nprivate { unexport m }",
            "unexports",
            "unexported",
        ),
    ] {
        let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        let (winner, loser) = if last == "exports" {
            (&c.exports, &c.unexports)
        } else {
            (&c.unexports, &c.exports)
        };
        assert!(winner.contains("m"), "{body}: {last} must hold `m`");
        assert!(
            !loser.contains("m"),
            "{body}: the superseded set still holds `m` — cross-file dispatch \
                 reads any `exports` entry as decisive",
        );
        assert_eq!(c.methods["m"].visibility, want_vis, "{body}");
    }
}

#[test]
fn a_self_scoped_visibility_word_never_touches_the_instance_sets() {
    // TN for the same sets: they are the *instance*-side record, so a
    // class-object-side flip must stay out of them entirely (in either
    // direction) — otherwise it would re-state an unrelated instance
    // method's export bit cross-file.
    let src = "oo::class create ::C {\n\
                   method m {} { return 1 }\n\
                   self { method m {} { return 2 }\n\
                   unexport m\n\
                   export m\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.exports.len(), 0, "{:?}", c.exports);
    assert_eq!(c.unexports.len(), 0, "{:?}", c.unexports);
    assert_eq!(c.methods["m"].visibility, "public");
    assert_eq!(c.class_methods["m"].visibility, "public");
}

#[test]
fn unwrapped_deletemethod_does_not_touch_the_class_side() {
    // TN. The unwrapped word is instance-scoped; a class-object-side member
    // of the same name survives it. (Real Tcl goes further and makes the
    // cross-side form a hard definition-aborting error — `oo::define ::I5
    // { deletemethod cm }` over a `self`-only `cm` raises `method cm does
    // not exist` and leaves `info object methods ::I5` -> cm — so a
    // document that reaches this shape never ran; keeping the class-side
    // entry is the abstaining answer either way.)
    let src = "oo::class create ::C {\n\
                   self { method cm {} { return c } }\n\
                   }\n\
                   oo::define ::C { deletemethod cm }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert!(c.class_methods.contains_key("cm"));
}

#[test]
fn unwrapped_non_destructive_members_do_not_retract() {
    // TN for the registry flag: `export` / `unexport` / `filter` name a
    // method without removing it, so the unwrapped arm must leave them be.
    for member in ["export", "unexport", "filter"] {
        let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{member} m }}");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl9.0");
        assert!(
            r.all_classes["::C"].methods.contains_key("m"),
            "`{member}` must not retract",
        );
    }
}

#[test]
fn source_configuration_accumulates_redeclared_members_under_original_class_owner() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Conditional source chronology and canonical metadata, without an
    // executed configuration worker or a live Native method table.
    for source in [
        "oo::class create C {method m {} {return FIRST}}; oo::define C {deletemethod m}; oo::define C {method m {} {return LAST}}",
        "oo::class create C {method m {} {return FIRST}}; rename C Held; interp alias {} configure {} oo::define Held; configure {deletemethod m}; configure {method m {} {return LAST}}",
    ] {
        let mut result = Analyser::new().analyse(source, "tcl9.0");
        let declaration = result.original_class_declarations().next().unwrap();
        assert!(declaration.metadata().methods.contains_key("m"));
        let last = source.find("return LAST").unwrap();
        assert!(
            declaration.metadata().methods["m"]
                .body_span
                .as_range()
                .contains(&last)
        );
        let methods = declaration
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        let method = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"m")
            .unwrap();
        assert!(method.metadata().body_span.as_range().contains(&last));
        let canonical = declaration.clone();
        result.all_classes.clear();
        assert_eq!(
            result.original_class_declarations().next(),
            Some(&canonical)
        );
    }
    let source = "oo::class create C {}; rename C {}; oo::define C {method ghost {} {}}";
    let result = Analyser::new().analyse(source, "tcl9.0");
    assert_eq!(result.original_class_declarations().count(), 1);
    assert!(result.original_class_declarations().all(|declaration| {
        !declaration.metadata().methods.contains_key("ghost")
            && declaration
                .metadata()
                .original_members
                .declarations()
                .all(|method| method.original_name_input().bytes() != b"ghost")
    }));
}

#[test]
fn an_unwrapped_retracted_member_can_be_redeclared_by_a_later_body() {
    // Source-order guard, unwrapped half. Retraction happens where the body
    // is walked, so a later `oo::define` redeclaration wins rather than the
    // name being erased document-wide.
    let src = "oo::class create ::C { method m {} { return 1 } }\n\
                   oo::define ::C { deletemethod m }\n\
                   oo::define ::C { method m {} { return 2 } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(r.all_classes["::C"].methods.contains_key("m"));
}

#[test]
fn objdefine_deletemethod_drops_the_per_object_member() {
    // The per-object table is the same instance-side routing. Oracle:
    //   oo::objdefine $o { method im {} {…}; deletemethod im }
    //   info object methods $o  ->            (empty)
    // `oo::objdefine` records into a throwaway `ClassDef` that never
    // reaches `all_classes`, so this asserts the walk simply does not
    // panic or resurrect the name anywhere.
    let src = "oo::class create ::C { method m {} { return m } }\n\
                   set o [::C new]\n\
                   oo::objdefine $o { method im {} { return i }\n\
                   deletemethod im }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(!r.all_classes.contains_key("::@objdefine@::o"));
    assert!(r.all_classes["::C"].methods.contains_key("m"));
}

// snit (tcllib) type / widget support.  Verified against real
// `tclsh8.6` + tcllib.

#[test]
fn snit_type_recorded_as_class_with_members() {
    let src = "snit::type ::foo::Bar {\n\
                   variable v1\n\
                   typevariable tv1\n\
                   method m1 {a b} { return [expr {$a+$b+$v1}] }\n\
                   typemethod tm1 {} { return $tv1 }\n\
                   constructor {args} { set v1 0 }\n\
                   destructor { unset v1 }\n\
                   typeconstructor { set tv1 0 }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::foo::Bar").expect("Bar class recorded");
    assert_eq!(c.metaclass, "snit::type");
    assert!(c.methods.contains_key("m1"));
    assert!(c.class_methods.contains_key("tm1"));
    assert!(c.class_methods.contains_key("<typeconstructor>"));
    assert_eq!(c.constructors.len(), 1);
    assert!(c.destructor.is_some());
    assert_eq!(c.variables, vec!["v1".to_string(), "tv1".to_string()]);
}

#[test]
fn snit_widget_keeps_win_and_hull_vars() {
    // A snit::widget injects `win` and `hull` instance variables — both
    // are recorded (only the four implicit scalars are filtered).
    let src = "snit::widget Dial {\n\
                   variable state\n\
                   method draw {} { return $win }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::Dial").expect("Dial recorded");
    assert_eq!(c.metaclass, "snit::widget");
    assert_eq!(
        c.variables,
        vec!["win".to_string(), "hull".to_string(), "state".to_string()]
    );
}

#[test]
fn snit_method_body_suppresses_self_dispatch_and_implicit_vars() {
    // Inside a snit method, `$self`/`$component` dispatch and reads of
    // instance variables must not false-fire W307 (non-literal command),
    // W210 (read-before-set), or W211/W214 (unused).
    let src = "snit::widget mywidget {\n\
                   variable helper\n\
                   component inner\n\
                   method draw {} {\n\
                       $self configure -bg white\n\
                       $inner render\n\
                       $helper compute\n\
                       return $win\n\
                   }\n\
                   }";
    let r = crate::provider_fixtures::analyse(
        &format!("package require snit\n{src}"),
        "tcl8.6",
        &[crate::provider_fixtures::Provider::Snit],
    );
    for code in ["W210", "W211", "W214", "W307", "W308"] {
        assert!(
            !r.diagnostics.iter().any(|d| d.code.as_str() == code),
            "{code} must not fire in a snit method body: {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn snit_widgetadaptor_and_qualified_definer() {
    let mut a = Analyser::new();
    let r = a.analyse("::snit::widgetadaptor Foo { method m {} {} }", "tcl8.6");
    let c = r.all_classes.get("::Foo").expect("Foo recorded");
    assert_eq!(c.metaclass, "::snit::widgetadaptor");
    assert!(c.methods.contains_key("m"));
}

#[test]
fn non_snit_command_is_not_a_class() {
    // A plain command that merely starts with `snit` is not a definer.
    let mut a = Analyser::new();
    let r = a.analyse("snitch foo { bar }", "tcl8.6");
    assert_eq!(r.all_classes.len(), 0);
}

// [incr Tcl] `itcl::class` — recorded as a `ClassDef` with method scopes,
// access modifiers unwrapped, `inherit` → superclasses, `this` implicit.

#[test]
fn itcl_class_recorded_with_members() {
    let src = "itcl::class ::widgets::Dial {\n\
                   inherit Base\n\
                   variable value 0\n\
                   common count 0\n\
                   constructor {args} { set value 0 }\n\
                   destructor { unset value }\n\
                   method spin {delta} { incr value $delta }\n\
                   proc reset {} { set count 0 }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r
        .all_classes
        .get("::widgets::Dial")
        .expect("Dial class recorded");
    assert_eq!(c.metaclass, "itcl::class");
    assert_eq!(c.superclasses, vec!["Base".to_string()]);
    assert!(c.methods.contains_key("spin"));
    assert!(
        c.class_methods.contains_key("reset"),
        "proc is a class method"
    );
    assert_eq!(c.constructors.len(), 1);
    assert!(c.destructor.is_some());
    // `variable` + `common` declarations are recorded (implicit `this` is
    // filtered).
    assert_eq!(c.variables, vec!["value".to_string(), "count".to_string()]);
}

#[test]
fn itcl_access_modifiers_unwrap_with_visibility() {
    let src = "itcl::class C {\n\
                   public method api {} {}\n\
                   private method helper {} {}\n\
                   protected variable state 0\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::C").expect("C recorded");
    assert_eq!(c.methods["api"].visibility, "public");
    assert_eq!(c.methods["helper"].visibility, "private");
    // The wrapped `protected variable state` is still a declared variable.
    assert!(c.variables.contains(&"state".to_string()));
}

#[test]
fn itcl_method_body_suppresses_this_and_instance_vars() {
    // Inside an itcl method, `$this` dispatch and reads of instance /
    // `common` variables must not false-fire W210/W211/W307.
    let src = "itcl::class C {\n\
                   variable handler\n\
                   common registry\n\
                   method run {} {\n\
                       $this configure\n\
                       $handler process\n\
                       return $registry\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    for code in ["W210", "W211", "W214", "W307", "W308"] {
        assert!(
            !r.diagnostics.iter().any(|d| d.code.as_str() == code),
            "{code} must not fire in an itcl method body: {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn non_itcl_command_is_not_a_class() {
    // A command that merely starts with `itcl` is not a definer.
    let mut a = Analyser::new();
    let r = a.analyse("itclish foo { bar }", "tcl8.6");
    assert_eq!(r.all_classes.len(), 0);
}

// OO body-walks: `initialise` body, `property -get/-set` accessor bodies,
// and the `new` / `createWithNamespace` class-command variants.

#[test]
fn oo_initialise_body_is_walked() {
    // A `variable` read inside `initialise { … }` must not false-fire
    // W210 read-before-set — the class-level init body is walked with the
    // class's instance variables visible.
    let src = "oo::class create Foo {\n\
                   variable cache\n\
                   initialise { set cache [dict create] }\n\
                   method get {k} { return [dict get $cache $k] }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
        "initialise body should be walked cleanly: {:?}",
        r.diagnostics
    );
}

/// A class-shared variable set up by the class-level `initialize` script
/// and reached from a method through `classvariable` must not draw a
/// read-before-set.
///
/// Oracled on tclsh 9.0.4: this exact class answers `5e-6`. The
/// `initialize` body runs in the class's own object namespace, so the
/// write and the read live in different frames — which is precisely why
/// `classvariable` has to be modelled as a declaration rather than as a
/// plain local.
#[test]
fn classvariable_reaches_a_variable_initialise_declared() {
    let src = "oo::class create Foo {\n\
                   initialize {\n\
                   variable Pitch\n\
                   const Pitch 5e-6\n\
                   }\n\
                   method m {} { classvariable Pitch; return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code.as_str() == "W210"),
        "classvariable reaches the initialise-declared cell: {:?}",
        r.diagnostics
    );
}

/// Near variants of the same shape, each **run** on tclsh
/// 9.0.4: a plain `set` instead of `const`, a value-bearing
/// `variable`, the British `initialise` spelling, the `oo::define`
/// form, two names in one `classvariable` call (both must bind — a
/// `VarWrite` role on index 0 alone leaves the second unbound), and a
/// write/read split across two methods.
///
/// The `variable Pitch` before the `const` is load-bearing,
/// not noise: an `initialize` script runs in a call frame, so a bare
/// `const Pitch 5e-6` there makes a *frame-local* constant that no
/// method can reach (tclsh 9.0.4: `can't read "Pitch": no such
/// variable`). Only the `variable` declaration aims the write at the
/// class-shared cell — which is why that shape is absent from this
/// clean-list.
#[test]
fn classvariable_variants_stay_clean() {
    for (label, src) in [
        (
            "set instead of const",
            "oo::class create Foo {\n\
                 initialize { variable Pitch; set Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "value-bearing variable",
            "oo::class create Foo {\n\
                 initialize { variable Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "initialise spelling",
            "oo::class create Foo {\n\
                 initialise { variable Pitch; const Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "oo::define form",
            "oo::class create Foo {}\n\
                 oo::define Foo {\n\
                 initialize { variable Pitch; const Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "two names in one call",
            "oo::class create Foo {\n\
                 initialize { variable a 1; variable b 2 }\n\
                 method m {} { classvariable a b; return [list $a $b] }\n\
                 }",
        ),
        (
            "written in one method, read in another",
            "oo::class create Foo {\n\
                 method w {} { classvariable Count; set Count 1 }\n\
                 method r {} { classvariable Count; return $Count }\n\
                 }",
        ),
        (
            // The `classvariable(n)` manual page's own example.
            "manual-page Counted example",
            "oo::class create Counted {\n\
                 initialise { variable count 0 }\n\
                 variable number\n\
                 constructor {} { classvariable count; set number [incr count] }\n\
                 method report {} { classvariable count; puts \"$number of $count\" }\n\
                 }",
        ),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
            "{label}: {:?}",
            r.diagnostics
        );
    }
}

/// The other half: the classvariable handling must not silence a genuine
/// unbound read. Without `classvariable`, a method's `$Pitch` is an
/// *instance* variable that the class-level `initialize` never wrote —
/// tclsh 9.0.4 raises `can't read "Pitch": no such variable` — so W210
/// is correct and has to stay.
#[test]
fn a_method_read_without_classvariable_still_warns() {
    let src = "oo::class create Foo {\n\
                   initialize { variable Pitch; const Pitch 5e-6 }\n\
                   method m {} { ::return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.diagnostics.iter().any(|d| d.code.as_str() == "W210"),
        "an instance-side read of a class-scoped cell is a real bug: {:?}",
        r.diagnostics
    );
}

/// The same snippet under 8.6 reports what 8.6 actually lacks —
/// `initialize` and `const` are both 9.0 members — and never W210.
/// `classvariable` itself is reachable there only through Tcllib's
/// `ooutil`, which the document requires.
#[test]
fn the_reporter_snippet_under_86_reports_the_missing_commands() {
    let src = "package require ooutil\n\
                   oo::class create Foo {\n\
                   initialize { variable Pitch; const Pitch 5e-6 }\n\
                   method m {} { classvariable Pitch; return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"W210"),
        "8.6 must not add a read-before-set: {:?}",
        r.diagnostics
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W002" && d.message.contains("'const'")),
        "8.6 has no `const` (TIP 677 is 9.0): {:?}",
        r.diagnostics
    );
}

/// `classvariable` binds only literal, unqualified names: a
/// substitution names no cell the analyser can see, so it stays the
/// W212 it already was rather than silently declaring `$dyn`.
#[test]
fn a_dynamic_classvariable_name_is_not_a_declaration() {
    let src = "oo::class create Foo { method m {} { classvariable $dyn; return $Pitch } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.diagnostics.iter().any(|d| d.code.as_str() == "W212"),
        "a substituted name is still reported: {:?}",
        r.diagnostics
    );
}

#[test]
fn oo_property_accessor_bodies_are_walked() {
    // `-get`/`-set` accessor bodies are walked with the instance variable
    // `val` and the implicit `value` visible — no false W210 / W307.
    // `property` is a 9.0 member, so the vector runs under a 9.0 dialect.
    let src = "oo::configurable create Bar {\n\
                   variable val\n\
                   property color -get { return $val } -set { set val $value }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(r.all_classes.contains_key("::Bar"));
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W210" | "W307")),
        "property accessor bodies should be walked cleanly: {:?}",
        r.diagnostics
    );
}

#[test]
fn oo_class_create_with_namespace_is_not_exported() {
    // C Tcl 9.0.4 and 8.6 both reject this ordinary command dispatch:
    // createWithNamespace exists internally but is not exported on
    // oo::class. A rejected call cannot introduce a class.
    let mut a = Analyser::new();
    let r = a.analyse("oo::class createWithNamespace MyCls ::ns { }", "tcl8.6");
    assert!(
        !r.all_classes.keys().any(|k| k.contains("MyCls")),
        "an unexported manufacturer must not record a class: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}
