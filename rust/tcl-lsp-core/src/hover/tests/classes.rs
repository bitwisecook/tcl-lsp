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

//! Class cards: properties, counted class and method sources, hierarchy details.

use super::*;

#[test]
fn original_property_hover_uses_unique_current_source_declaration() {
    let source = "oo::configurable create C {property p\\uD800 p\\uD801; self method make {} {}}\nreturn p\\uD800\n";
    let analysis = analyse_for(source, "tcl9.0");
    let class = analysis.original_class_declarations().next().unwrap();
    assert_eq!(
        class.metadata().original_properties.declarations().count(),
        2
    );
    let mut cards = Vec::new();
    for name in ["p\\uD800", "p\\uD801"] {
        let at = u32::try_from(source.find(name).unwrap()).unwrap();
        let card = hover(source, 0, at, &analysis, None).unwrap();
        assert!(card.value.contains("**property**"), "{}", card.value);
        assert!(
            card.value
                .contains("Source declaration; property availability depends on execution.")
        );
        cards.push(card.value);
    }
    assert_ne!(cards[0], cards[1]);
    let offset = u32::try_from(source.find("p\\uD800").unwrap()).unwrap();
    assert!(
        !hover(&format!("#{source}"), 0, offset, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    assert!(
        !hover(source, 1, 7, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    assert!(
        !hover(source, 0, offset + 7, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    let mut foreign_config = analysis.clone();
    foreign_config.body_lexer_config = analyse_for(source, "jim").body_lexer_config;
    assert_ne!(foreign_config.body_lexer_config, analysis.body_lexer_config);
    assert!(
        !hover(source, 0, offset, &foreign_config, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    let mut foreign = analysis.clone();
    foreign.original_class_metadata = analyse_for(source, "tcl9.1").original_class_metadata;
    assert!(
        !hover(source, 0, offset, &foreign, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let distinct =
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(context.commands())));
    let other_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        distinct,
        input.lexer_config(),
    );
    assert_ne!(&other_input, input);
    let other = Analyser::new()
        .with_resolved_input(other_input)
        .analyse(source, "tcl9.0");
    assert_eq!(other.original_class_declarations().count(), 1);
    let mut foreign_generation = analysis.clone();
    foreign_generation.original_class_metadata = other.original_class_metadata;
    assert!(
        !hover(source, 0, offset, &foreign_generation, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    let mut duplicate = analysis.clone();
    duplicate
        .original_class_metadata
        .push(duplicate.original_class_metadata[0].clone());
    assert!(
        !hover(source, 0, offset, &duplicate, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
}

#[test]
fn original_property_hover_does_not_invent_a_shadowed_definer() {
    let unavailable = "oo::configurable create C {property size}";
    let tcl86 = analyse(unavailable);
    assert_eq!(tcl86.original_class_declarations().count(), 0);
    assert!(
        !hover(unavailable, 0, 36, &tcl86, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
    let source = "proc oo::configurable args {}\noo::configurable create C {property size}\n";
    let analysis = analyse_for(source, "tcl9.0");
    assert_eq!(analysis.original_class_declarations().count(), 0);
    assert!(
        !hover(source, 1, 37, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**property**"))
    );
}

#[test]
fn counted_method_hover_does_not_claim_effect_operands_or_later_declarations() {
    let source = "oo::class create C\\uD800 {method m\\uD801 {} {}; export m\\uD801; deletemethod m\\uD801; renamemethod m\\uD801 n}\noo::define C\\uD800 method m\\uD801 {} {}\nC\\uD800 create obj\nobj m\\uD801\n";
    let analysis = analyse(source);
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("counted class")
    };
    let methods = class
        .metadata()
        .initial_method_declarations()
        .expect("initial declaration inventory");
    assert_eq!(methods.len(), 1);
    let index = tcl_lexer::LineIndex::new(source);
    let all: Vec<_> = source
        .match_indices("m\\uD801")
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(all.len(), 6);
    for (index_in_source, offset) in all.into_iter().enumerate() {
        let position = index.position_at_utf16(u32::try_from(offset).unwrap(), source);
        let card = hover(
            source,
            position.line,
            position.character.get(),
            &analysis,
            None,
        )
        .is_some_and(|hover| hover.value.contains("**Method source declaration**"));
        assert_eq!(card, index_in_source == 0, "occurrence {index_in_source}");
    }
}

#[test]
fn counted_method_hover_shows_only_exact_initial_source_declarations() {
    let source = "oo::class create C\\uD800 {method m\\uD800 {} {}; method m\\uD801 {} {}; self method make {} {}; method m\\uD800 {} {}}\nC\\uD800 create obj\nobj m\\uD800\n";
    let analysis = analyse(source);
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("counted class")
    };
    assert_eq!(
        class
            .metadata()
            .initial_method_declarations()
            .unwrap()
            .len(),
        4
    );
    for name in ["m\\uD800", "m\\uD801", "make"] {
        let offsets: Vec<_> = source
            .match_indices(name)
            .filter(|(offset, _)| *offset < source.find('\n').unwrap())
            .map(|(offset, _)| offset)
            .collect();
        assert_ne!(offsets, [] as [usize; 0]);
        for offset in offsets {
            let card = hover(source, 0, u32::try_from(offset).unwrap(), &analysis, None).unwrap();
            assert!(
                card.value.contains("**Method source declaration**"),
                "{name}: {}",
                card.value
            );
            assert!(card.value.contains(name));
            assert!(
                card.value
                    .contains("Method availability depends on execution.")
            );
        }
    }
    assert!(
        !hover(source, 2, 4, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
    let at = u32::try_from(source.find("m\\uD801").unwrap()).unwrap();
    assert!(
        !hover(source, 0, at + 7, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
    assert!(
        !hover(&format!("#{source}"), 0, at, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
}

#[test]
fn counted_method_hover_rejects_unavailable_ambiguous_and_foreign_inventory() {
    for body in [
        "{unknown_member; method m {} {}}",
        "{method $name {} {}}",
        "{foreach name {a b} {method $name {} {}}}",
        "{method m {} \"unterminated}",
    ] {
        let source = format!("oo::class create C\\uD800 {body}");
        let analysis = analyse(&source);
        let [class] = analysis.original_counted_class_metadata.as_slice() else {
            panic!("{source}")
        };
        assert!(class.metadata().initial_method_declarations().is_none());
        if let Some(offset) = source.find("method m ") {
            let at = u32::try_from(offset + "method ".len()).unwrap();
            assert!(
                !hover(&source, 0, at, &analysis, None)
                    .is_some_and(|hover| hover.value.contains("**Method source declaration**")),
                "{source}"
            );
        }
    }
    let source = "oo::class create C\\uD800 {method m\\uD801 {} {}}";
    let analysis = analyse(source);
    let at = u32::try_from(source.find("m\\uD801").unwrap()).unwrap();
    assert!(
        hover(source, 0, at, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
    let mut duplicate = analysis.clone();
    duplicate
        .original_counted_class_metadata
        .push(duplicate.original_counted_class_metadata[0].clone());
    assert!(
        !hover(source, 0, at, &duplicate, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
    let mut foreign = analysis.clone();
    foreign.original_counted_class_metadata = Analyser::new()
        .analyse(source, "tcl9.0")
        .original_counted_class_metadata;
    assert!(
        !hover(source, 0, at, &foreign, None)
            .is_some_and(|hover| hover.value.contains("**Method source declaration**"))
    );
}

#[test]
fn counted_method_hover_fences_original_backticks() {
    let source = "oo::class create C\\uD800 {method m\\uD801``` {} {}}";
    let analysis = analyse(source);
    let at = u32::try_from(source.find("m\\uD801").unwrap()).unwrap();
    let card = hover(source, 0, at, &analysis, None).unwrap();
    assert!(card.value.contains("````tcl\nm\\uD801```\n````"));
}

#[test]
fn counted_class_hover_keeps_distinct_source_bytes_and_horizons() {
    let source = "oo::class create C\\uD800 {method m {} {}}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\nrename C\\uD800 moved\nmoved new\n";
    let analysis = analyse(source);
    assert_eq!(analysis.original_counted_class_metadata.len(), 2);
    let first = hover(source, 2, 0, &analysis, None).unwrap();
    let second = hover(source, 3, 0, &analysis, None).unwrap();
    let moved = hover(source, 5, 0, &analysis, None).unwrap();
    for card in [&first, &second, &moved] {
        assert!(card.value.contains("**Class declaration**"));
        assert!(
            card.value
                .contains("Class availability depends on execution.")
        );
        assert!(!card.value.contains("**Methods**"));
    }
    assert_eq!(first.value, moved.value);
    assert_ne!(first.value, second.value);
    for (card, expected) in [
        (&first, b"::C\xed\xa0\x80".as_slice()),
        (&second, b"::C\xed\xa0\x81".as_slice()),
    ] {
        let word = card.value.lines().nth(3).unwrap();
        let config = analysis.body_lexer_config.unwrap();
        let policy = analysis.original_counted_class_metadata[0].name().policy();
        assert_eq!(
            tcl_syntax::backslash::native_source_string_bytes_channel_in(
                &word.as_bytes()[1..word.len() - 1],
                tcl_lexer::SourceChannel::Document,
                config.escapes,
                policy.string_protocol()
            )
            .unwrap(),
            expected
        );
    }
    assert!(
        !hover(source, 2, 7, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Class declaration**"))
    );
    assert!(
        !hover(&format!("#{source}"), 2, 0, &analysis, None)
            .is_some_and(|hover| hover.value.contains("**Class declaration**"))
    );
}

#[test]
fn counted_class_hover_refuses_unsettled_and_foreign_binding() {
    for source in [
        "oo::class create C\\uD800 {}\nrename C\\uD800 {}\nC\\uD800 new\n",
        "oo::class create C\\uD800 {}\nrename C\\uD800 {}\nproc C\\uD800 args {}\nC\\uD800 new\n",
        "oo::class create C\\uD800 {}\nproc change args {rename C\\uD800 {}}\ntrace add execution C\\uD800 enter change\nC\\uD800 new\n",
        "oo::class create C\\uD800 {error nope}\nC\\uD800 new\n",
    ] {
        let analysis = analyse(source);
        assert_eq!(
            analysis.original_counted_class_metadata.len(),
            1,
            "{source}"
        );
        let line = u32::try_from(source.lines().count() - 1).unwrap();
        assert!(
            !hover(source, line, 0, &analysis, None)
                .is_some_and(|hover| hover.value.contains("**Class declaration**")),
            "{source}"
        );
    }
    let source =
        "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\n";
    let analysis = analyse(source);
    let first_offset = u32::try_from(source.find("C\\uD800 new").unwrap()).unwrap();
    let second_offset = u32::try_from(source.find("C\\uD801 new").unwrap()).unwrap();
    let mut swapped = analysis.clone();
    let reference = swapped
        .command_invocations
        .iter()
        .find(|call| call.range.start() == second_offset && call.lookup.is_execution_site())
        .unwrap()
        .resolved_command_reference
        .clone();
    assert!(reference.is_some());
    swapped
        .command_invocations
        .iter_mut()
        .find(|call| call.range.start() == first_offset && call.lookup.is_execution_site())
        .unwrap()
        .resolved_command_reference = reference;
    assert!(
        !hover(source, 2, 0, &swapped, None)
            .is_some_and(|hover| hover.value.contains("**Class declaration**"))
    );
    let mut foreign = analysis.clone();
    foreign.original_counted_class_metadata = Analyser::new()
        .analyse(source, "tcl9.0")
        .original_counted_class_metadata;
    assert!(
        !hover(source, 2, 0, &foreign, None)
            .is_some_and(|hover| hover.value.contains("**Class declaration**"))
    );
    let mut duplicate = analysis.clone();
    duplicate
        .original_counted_class_metadata
        .push(duplicate.original_counted_class_metadata[0].clone());
    assert!(
        !hover(source, 2, 0, &duplicate, None)
            .is_some_and(|hover| hover.value.contains("**Class declaration**"))
    );
}

#[test]
fn hover_on_class_name_returns_metaclass_signature() {
    let src = "oo::class create Greeter {}\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 18, &analysis, None);
    let h = h.expect("class name should have a metaclass hover");
    assert!(h.value.contains("Greeter"), "{}", h.value);
    assert!(
        h.value.contains("oo::class create"),
        "expected metaclass declaration, got {}",
        h.value,
    );
}

#[test]
fn class_hover_text_lists_methods_alphabetically() {
    let src = concat!(
        "oo::class create Foo {\n",
        "    method beta {} {}\n",
        "    method alpha {} {}\n",
        "}\n",
    );
    let analysis = analyse(src);
    let class_def = analysis
        .all_classes
        .values()
        .next()
        .expect("class recorded")
        .clone();
    let text = class_hover_text(&analysis, &class_def);
    // Methods listed in sorted order.
    let alpha_pos = text.find("alpha");
    let beta_pos = text.find("beta");
    let a = alpha_pos.expect("hover should list the alpha method");
    let b = beta_pos.expect("hover should list the beta method");
    assert!(a < b, "expected alpha before beta in: {text}");
}

#[test]
fn class_hover_shows_mro_and_subclasses() {
    let src = "oo::class create A {}\noo::class create B {\n    superclass A\n}\noo::class create C {\n    superclass B\n}\n";
    let analysis = analyse(src);
    let b = analysis.all_classes.get("::B").expect("B").clone();
    let text = class_hover_text(&analysis, &b);
    assert!(text.contains("**MRO**"), "MRO missing: {text}");
    assert!(text.contains("::B → ::A"), "MRO chain wrong: {text}");
    assert!(
        text.contains("**Subclasses**") && text.contains("::C"),
        "subclasses missing: {text}"
    );
}

#[test]
fn class_hover_disambiguates_same_name_across_namespaces() {
    // `::A::Shape` and `::B::Shape` share a simple name and each has a
    // distinct subclass.  A bare `Shape` written inside `::A` must hover
    // `::A::Shape` (with `::A::Circle` among its subclasses), never the
    // arbitrary first same-named class a namespace-blind scan would pick.
    let src = "namespace eval A {\n\
                       oo::class create Shape {}\n\
                       oo::class create Circle {\n\
                           superclass Shape\n\
                       }\n\
                   }\n\
                   namespace eval B {\n\
                       oo::class create Shape {}\n\
                       oo::class create Square {\n\
                           superclass Shape\n\
                       }\n\
                   }\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    // Occurrence 2 is `superclass Shape` inside `::A::Circle`.
    let (l, c) = pos_of(src, "Shape", 2);
    let h = hover(src, l, c, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("::A::Shape"), "{}", h.value);
    assert!(h.value.contains("::A::Circle"), "{}", h.value);
    assert!(!h.value.contains("::B::Shape"), "{}", h.value);
    assert!(!h.value.contains("::B::Square"), "{}", h.value);
}
