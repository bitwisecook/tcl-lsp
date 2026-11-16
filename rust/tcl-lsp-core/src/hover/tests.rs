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

fn assert_foreign_factory_method_source_parent(
    source: &str,
    analysis: &AnalysisResult,
    headers: &[(
        u64,
        tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders,
    )],
) {
    use tcl_compiler::registry_invocation::source_structure::original_foreign_factory_bodies;
    let bodies = original_foreign_factory_bodies(source, analysis, 2, headers);
    let [body] = bodies.as_slice() else {
        panic!("unique foreign body");
    };
    let class = body.class_syntax().unwrap();
    let members = class.callable_syntax().unwrap();
    let [member] = members.as_slice() else {
        panic!("one written method");
    };
    assert_eq!(member.parent().body().caller_document(), 2);
    assert_eq!(member.parent().body().declaring_document(), 1);
    assert_eq!(member.name(), "go");
}

#[test]
fn foreign_factory_method_hover_keeps_the_source_parent() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders;
    let library =
        Analyser::new().analyse("oo::class create Meta {superclass ::oo::class}", "tcl9.0");
    let header = OriginalProjectFactoryHeaders::capture(&library).unwrap();
    let source = "Meta create Made { method go {} {return 1} }";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let caller_header = OriginalProjectFactoryHeaders::capture(&analysis).unwrap();
    let headers = [(1, header.clone()), (2, caller_header.clone())];
    assert_foreign_factory_method_source_parent(source, &analysis, &headers);
    let position = u32::try_from(source.find("go").unwrap()).unwrap();
    assert!(super::may_have_foreign_factory_hover(
        source, 0, position, &analysis
    ));
    assert!(!super::may_have_foreign_factory_hover(
        source, 0, 0, &analysis
    ));
    let hover =
        super::conditional_foreign_factory_hover(source, 0, position, &analysis, 2, &headers)
            .unwrap();
    assert!(hover.value.contains("Conditional source method of `Made`"));
    assert_eq!(
        super::conditional_foreign_factory_hover(
            source,
            0,
            position,
            &analysis,
            2,
            &[
                (1, header.clone()),
                (3, header.clone()),
                (2, caller_header.clone())
            ]
        ),
        None
    );
    let blocker = Analyser::new().analyse("proc Meta args {}", "tcl9.0");
    let blocker = OriginalProjectFactoryHeaders::capture(&blocker).unwrap();
    assert_eq!(
        super::conditional_foreign_factory_hover(
            source,
            0,
            position,
            &analysis,
            2,
            &[
                (1, header.clone()),
                (3, blocker),
                (2, caller_header.clone())
            ]
        ),
        None
    );
    for blocked in [
        "interp alias {} Meta {} other; Meta create Made {method go {} {}}",
        "proc Meta args {}; Meta create Made {method go {} {}}",
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        "set ::auto_index(unknown_effect) {source lib.tcl}; unknown_effect; Meta create Made {method go {} {}}",
        "Meta create [set name] {method go {} {}}",
        "Meta create {*}$args",
    ] {
        let blocked_analysis = Analyser::new().analyse(blocked, "tcl9.0");
        let blocked_header = OriginalProjectFactoryHeaders::capture(&blocked_analysis)
            .unwrap_or_else(|| caller_header.clone());
        let position = u32::try_from(blocked.find("go").unwrap_or(0)).unwrap();
        assert_eq!(
            super::conditional_foreign_factory_hover(
                blocked,
                0,
                position,
                &blocked_analysis,
                2,
                &[(1, header.clone()), (2, blocked_header)]
            ),
            None,
            "{blocked}"
        );
    }
    assert_eq!(
        super::conditional_foreign_factory_hover(
            &format!("{source} "),
            0,
            position,
            &analysis,
            2,
            &headers
        ),
        None
    );
}

#[test]
fn foreign_factory_hover_uses_the_manufacturer_selected_body_ordinal() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders;
    let producer = concat!(
        "oo::class create Meta {superclass oo::class; ",
        "self method create {name extra body} {next $name $body}}",
    );
    let library = Analyser::new().analyse(producer, "tcl9.0");
    let source = "Meta create Made ignored {method go {} {return 1}}";
    let caller = Analyser::new().analyse(source, "tcl9.0");
    let headers = [
        (1, OriginalProjectFactoryHeaders::capture(&library).unwrap()),
        (2, OriginalProjectFactoryHeaders::capture(&caller).unwrap()),
    ];
    let position = u32::try_from(source.find("go").unwrap()).unwrap();
    let hover = super::conditional_foreign_factory_hover(source, 0, position, &caller, 2, &headers)
        .unwrap();
    assert!(hover.value.contains("Conditional source method of `Made`"));
}

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

use tcl_dialect::model::{Family, SurfaceLayer};

use super::*;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn analyse_for(source: &str, environment: &str) -> AnalysisResult {
    let mut analyser = Analyser::new();
    let input = analyser
        .prepare_analysis_input(environment)
        .expect("test environment has a complete analysis input");
    analyser = analyser.with_resolved_input(input);
    analyser.analyse(source, environment).clone()
}

#[test]
fn standalone_reference_cursor_uses_explicit_selected_grammar_and_utf16_geometry() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let source = "puts café🙂 ${a{b}c} ${cash$name} ${scalar(open)tail}";
    let prefix = "puts café🙂 ";
    let character = u32::try_from(prefix.encode_utf16().count() + 2).unwrap();
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var: style,
            ..Default::default()
        };
        assert_eq!(
            find_var_at_position_with_config(source, 0, character, config).as_deref(),
            Some(if style.nests() { "a{b}c" } else { "a{b" }),
        );
        let dollar = u32::try_from(
            source[..source.find("$name").unwrap()]
                .encode_utf16()
                .count(),
        )
        .unwrap();
        assert_eq!(
            find_var_at_position_with_config(source, 0, dollar, config).as_deref(),
            Some("cash$name")
        );
        let tail = u32::try_from(
            source[..source.find("scalar(open)").unwrap()]
                .encode_utf16()
                .count(),
        )
        .unwrap();
        assert_eq!(
            find_var_at_position_with_config(source, 0, tail, config).as_deref(),
            Some("scalar(open)tail")
        );
    }
}

#[test]
fn qualified_colon_variable_hover_keeps_the_empty_cell_separate() {
    let source = "set {} EMPTY; set : COLON; puts ${}; puts $:";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        let hover = qualified_variable_hover(&analysis, ":::").unwrap();
        assert!(hover.value.contains("`:::`"), "{dialect}: {}", hover.value);
        assert!(!hover.value.contains("`::`"), "{dialect}: {}", hover.value);
    }
}

fn position_of(source: &str, needle: &str) -> (u32, u32) {
    let offset = source.find(needle).expect("test cursor text");
    let before = &source[..offset];
    let line = u32::try_from(before.bytes().filter(|b| *b == b'\n').count()).unwrap();
    let column = u32::try_from(
        before
            .rsplit_once('\n')
            .map_or(before, |(_, tail)| tail)
            .len(),
    )
    .unwrap();
    (line, column)
}

#[test]
fn find_word_span_returns_none_at_eol() {
    // Position one past the line's last char yields None.
    let src = "proc foo {} {}\n";
    let line = src.split('\n').next().unwrap();
    let len = u32::try_from(line.chars().count()).expect("len fits u32");
    assert!(find_word_span_at_position(src, 0, len).is_none());
}

#[test]
fn smoke_find_word_span_extracts_word_under_cursor() {
    // Cursor on the 'r' of `proc`.
    let src = "proc greet {} {}\n";
    let (word, start, end) = find_word_span_at_position(src, 0, 1).unwrap();
    assert_eq!(word, "proc");
    assert_eq!(start, 0);
    assert_eq!(end, 4);
}

#[test]
fn find_word_span_stops_at_dollar_sign() {
    // `$var` — `$` is in `_WORD_DELIMS`, so a cursor inside
    // `var` should yield just `var`.
    let src = "set x $var\n";
    let (word, start, end) = find_word_span_at_position(src, 0, 8).unwrap();
    assert_eq!(word, "var");
    assert_eq!(start, 7);
    assert_eq!(end, 10);
}

#[test]
fn smoke_find_var_at_position_recognises_dollar_ref() {
    // Cursor inside `$var`.
    let src = "set x $var\n";
    assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
}

#[test]
fn find_var_at_position_returns_none_for_bare_word() {
    let src = "set x 1\n";
    assert!(find_var_at_position(src, 0, 4).is_none());
}

#[test]
fn find_var_at_position_resolves_second_var_of_concatenation() {
    // `set z $x$y` — a cursor on `y` must resolve `y`, not
    // walk left across the `$` to the first variable `x`.
    let src = "set z $x$y\n";
    // Columns: s(0)e(1)t(2) (3)z(4) (5)$(6)x(7)$(8)y(9)
    assert_eq!(find_var_at_position(src, 0, 9), Some("y".to_owned())); // on `y`
    assert_eq!(find_var_at_position(src, 0, 7), Some("x".to_owned())); // on `x`
    // Three-way concatenation resolves the middle var too.
    let src3 = "set w $a$b$c\n";
    assert_eq!(find_var_at_position(src3, 0, 9), Some("b".to_owned()));
}

#[test]
fn find_var_at_position_stops_at_lone_colon() {
    // `$host:$port` — a lone `:` is not part of the variable name (Tcl
    // substitutes only `$host`); it must not be pulled in as `host:`
    // (issue 183).
    let src = "puts $host:$port\n";
    // Columns: p0 u1 t2 s3 (4) $5 h6 o7 s8 t9 :10 $11 ...
    assert_eq!(find_var_at_position(src, 0, 7), Some("host".to_owned()));
    // The `::` qualifier IS part of the name, and a full colon run stays.
    let src2 = "puts $a::b:$c\n";
    assert_eq!(find_var_at_position(src2, 0, 7), Some("a::b".to_owned()));
    let src3 = "puts $a:::b\n";
    assert_eq!(find_var_at_position(src3, 0, 7), Some("a:::b".to_owned()));
}

#[test]
fn binary_field_bytes_saturates_instead_of_overflowing() {
    // `d600000000` ⇒ 8 × 600000000 overflows u32; report unknown size,
    // never panic (debug) or wrap (release) (issue 182).
    assert_eq!(super::binary_field_bytes('d', 600_000_000, false), None);
    assert_eq!(super::binary_field_bytes('i', u32::MAX, false), None);
    // Ordinary sizes still compute.
    assert_eq!(super::binary_field_bytes('i', 4, false), Some(16));
}

#[test]
fn regex_and_glob_hover_escape_table_pipes() {
    // A `|` inside a code-span table cell must be `\|` or it splits the GFM
    // row (issue 187).
    let rx = regex_hover_text("foo|bar");
    assert!(
        rx.lines().all(|l| !l.starts_with("| `|`")),
        "raw pipe token would break the row: {rx}"
    );
    assert!(
        rx.contains("\\|"),
        "alternation `|` should be escaped: {rx}"
    );
    // Glob alternation `{a,b}` has no pipe, but a bracketed pipe would; a
    // char-class-style pattern with `|` escapes too.
    let rxcc = regex_hover_text("[a|b]");
    assert!(rxcc.contains("\\|"));
    // Every non-separator row keeps a balanced cell count (3 pipes → 2
    // cells) after escaping.
    for line in rx.lines().filter(|l| l.starts_with("| ")) {
        let unescaped = line.replace("\\|", "");
        assert_eq!(
            unescaped.matches('|').count(),
            3,
            "row must have exactly two cells: {line}"
        );
    }
}

#[test]
fn find_var_at_position_recognises_braced_form() {
    // Cursor on the `r` inside `${var}`.  The braced form
    // should still resolve to `"var"` so rename and hover
    // can find the symbol.
    let src = "set x ${var}\n";
    assert_eq!(find_var_at_position(src, 0, 9), Some("var".to_owned()));
    // Cursor immediately after the opening `${` (start of name).
    assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
    // Cursor on the closing `}` itself — the inner name is
    // still resolvable as long as the cursor sits inside the
    // braces inclusive of the close brace.
    assert_eq!(find_var_at_position(src, 0, 11), Some("var".to_owned()));
}

#[test]
fn hover_on_proc_name_returns_signature() {
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("expected hover for proc name");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("proc ::greet"), "{}", h.value);
    assert!(h.value.contains("name"), "{}", h.value);
}

#[test]
fn hover_on_ensemble_subcommand_resolves_target_proc() {
    // TP — hover on an ensemble subcommand call
    // site surfaces the resolved target proc's own signature.
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
    let analysis = analyse(src);
    // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
    let h = hover(src, 7, 8, &analysis, None).expect("expected hover for ensemble subcommand");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("Ensemble subcommand"), "{}", h.value);
    assert!(h.value.contains("proc ::e::Foo"), "{}", h.value);
}

#[test]
fn hover_on_tk_ensemble_configure_splice_resolves_the_real_target() {
    // TP — `tk`'s built-in ensemble is extended at
    // runtime via `namespace ensemble configure tk -map [dict merge
    // [namespace ensemble configure tk -map] {systray ::tk::systray}]`
    // (the real `tk/library/systray.tcl` idiom). Hover on the call
    // site's "systray" must surface the real `::tk::systray` proc, not
    // abstain or resolve a same-tail-name decoy elsewhere.
    let src = "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n";
    let analysis = analyse(src);
    // Cursor on "systray" in `tk systray create ...` (0-based line 5).
    let h = hover(src, 5, 5, &analysis, None).expect("expected hover for tk systray splice");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("proc ::tk::systray"), "{}", h.value);
    assert!(!h.value.contains("DECOY"), "{}", h.value);
}

#[test]
fn hover_on_self_method_call_site_resolves() {
    // TP — `self method make {n} {...}` records into `class_methods`,
    // and `Widget make gadget`'s bare class-command receiver resolves
    // against it, so hover on the call site works end-to-end.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
    let analysis = analyse(src);
    // Cursor on `make` in `Widget make gadget` (line 3, col 8).
    let h = hover(src, 3, 8, &analysis, None).expect("expected hover for self-method call");
    assert!(h.value.contains("classmethod"), "{}", h.value);
    assert!(h.value.contains("Widget::make"), "{}", h.value);
}

#[test]
fn hover_on_proc_qualified_name() {
    let src = "namespace eval ::ns { proc helper {} { return } }\n";
    let analysis = analyse(src);
    // Cursor on `helper` token at column ~28
    let h = hover(src, 0, 28, &analysis, None);
    // Either matches via simple name or qualified name; the
    // contract is that hover surfaces the proc when present.
    let h = h.expect("qualified proc declaration should have hover");
    assert!(h.value.contains("helper"), "{}", h.value);
}

// namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

#[test]
fn hover_unqualified_call_prefers_callers_namespace_proc() {
    // Two namespaces each define `helper`; hovering the unqualified call
    // inside ::b must surface ::b::helper — the current namespace
    // resolves before global, never a sibling namespace.
    let src = "namespace eval a {\n    proc helper {x} { return 1 }\n}\nnamespace eval b {\n    proc helper {y} { return 2 }\n    helper 5\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 5, 6, &analysis, None).expect("hover on namespaced call");
    assert!(h.value.contains("proc ::b::helper {y}"), "{}", h.value);
}

#[test]
fn hover_namespace_proc_set_shadows_builtin_inside_namespace_only() {
    // A namespace proc named like a builtin: inside ::ns the proc wins;
    // at global scope only the builtin resolves — a proc in an unrelated
    // namespace must not hijack builtin hover.
    let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let inside = hover(src, 2, 5, &analysis, Some(&registry)).expect("hover inside ns");
    assert!(inside.value.contains("proc ::ns::set"), "{}", inside.value);
    let global = hover(src, 4, 1, &analysis, Some(&registry)).expect("hover at global");
    assert!(global.value.contains("built-in"), "{}", global.value);
    assert!(!global.value.contains("::ns::set"), "{}", global.value);
}

#[test]
fn hover_global_call_fallback_is_deterministic() {
    // No ::helper exists, so no candidate resolves; the lenient tail
    // fallback fires and must pick the lexicographically smallest
    // qualified name (::a::helper) on every repeat — never a
    // `HashMap`-iteration-order hijack.
    let src = "namespace eval z {\n    proc helper {a} { return 26 }\n}\nnamespace eval a {\n    proc helper {b} { return 1 }\n}\nhelper 1\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let first = hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
    assert!(first.value.contains("proc ::a::helper"), "{}", first.value);
    for attempt in 0..8 {
        let repeat = hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
        assert_eq!(first, repeat, "attempt {attempt}: hover must be stable");
    }
}

#[test]
fn hover_on_unknown_word_returns_none() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // Cursor on "hello" — not a proc / class / var, so None.
    // (`puts` is a builtin and isn't in `all_procs` either.)
    assert!(hover(src, 0, 6, &analysis, None).is_none());
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
fn hover_on_dollar_var_returns_var_text() {
    // Variable defined at top level, referenced via `$x`.
    let src = "set x 1\nset y $x\n";
    let analysis = analyse(src);
    let h = hover(src, 1, 7, &analysis, None);
    let h = h.expect("referenced variable should have hover");
    assert!(h.value.contains("Variable"), "{}", h.value);
    assert!(h.value.contains("`x`"), "{}", h.value);
}

#[test]
fn hover_on_proc_param_bareword_declaration_resolves() {
    // TP — a cursor on a proc parameter's own bareword name (not a
    // `$`-prefixed read) must hover, just as the same variable's `$name`
    // reads do.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let h = hover(src, 0, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
}

#[test]
fn hover_on_catch_resultvar_bareword_resolves() {
    // TP — the other bareword shape: a `catch script name`
    // result-var reuses an existing variable; its own bareword token
    // must still hover, surfacing the same variable.
    let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 16-20).
    let h = hover(src, 1, 17, &analysis, None).expect("hover");
    assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
}

#[test]
fn hover_on_var_surfaces_inferred_intrep() {
    // `x` is assigned an integer literal; the compiler's
    // type-propagation pass should infer `int`, and the hover
    // (given a registry) should surface it.
    let src = "set x 1\nset y $x\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 7, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("**Variable** `x`"), "{}", h.value);
    assert!(h.value.contains("**Inferred intrep**: int"), "{}", h.value);
}

#[test]
fn hover_inference_keeps_same_named_variables_in_the_selected_source_body() {
    for source in [
        "set same 1\nproc local {} {set same TEXT; puts $same}\n",
        "set same [gets stdin]\nproc local {} {set same TEXT; puts $same}\n",
        "proc first {} {set same 1; puts $same}\nproc second {} {set same TEXT; puts $same}\n",
    ] {
        let analysis = analyse(source);
        let cursor = u32::try_from(source.rfind("$same").unwrap() + 1).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
        let registry = tcl_registry::CommandRegistry::build_default();
        let selected = hover(
            source,
            position.line,
            position.character.get(),
            &analysis,
            Some(&registry),
        )
        .expect("actual selected local variable hover");
        assert!(
            selected.value.contains("**Inferred intrep**: string"),
            "{}",
            selected.value
        );
        assert!(
            !selected.value.contains("**Inferred intrep**: int"),
            "{}",
            selected.value
        );
        assert!(!selected.value.contains("**Taint**"), "{}", selected.value);
    }
}

#[test]
fn hover_inference_uses_the_represented_read_version_after_reassignment() {
    let source = "proc local {} {set same 123; puts $same; set same TEXT; puts $same}\n";
    let analysis = analyse(source);
    let registry = tcl_registry::CommandRegistry::build_default();
    let cursor = u32::try_from(source.rfind("$same").unwrap() + 1).unwrap();
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
    let selected = hover(
        source,
        position.line,
        position.character.get(),
        &analysis,
        Some(&registry),
    )
    .expect("actual current version has its own source read");
    assert!(
        selected.value.contains("**Inferred intrep**: string"),
        "{}",
        selected.value
    );
    assert!(
        !selected.value.contains("**Inferred intrep**: int"),
        "{}",
        selected.value
    );
}

#[test]
fn hover_on_pure_literal_surfaces_first_used_as() {
    // `l` is a pure list-shaped literal whose only typed read commits the
    // list intrep — the def-site pushback of the committed-intrep
    // dataflow surfaces "first used as: list" at the creation site.
    let src = "set l {1 2 3}\nllength $l\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("**Variable** `l`"), "{}", h.value);
    assert!(
        h.value
            .contains("**Inferred intrep**: string (first used as: list)"),
        "{}",
        h.value
    );
}

#[test]
fn hover_committed_producer_has_no_first_used_as() {
    // A committed producer (`[list …]`) is not "first used as" anything —
    // its intrep is set at the def, so the pushback annotation must not
    // appear.
    let src = "set l [list 1 2 3]\nllength $l\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
    assert!(
        !h.value.contains("first used as"),
        "committed producers must not carry the pushback label: {}",
        h.value
    );
}

#[test]
fn hover_returns_none_for_out_of_range_line() {
    let src = "proc foo {} {}\n";
    let analysis = analyse(src);
    assert!(hover(src, 99, 0, &analysis, None).is_none());
}

#[test]
fn builtin_hover_shows_requires_when_package_not_imported() {
    // `http::geturl` needs `package require http`; without the
    // import the hover appends the **Requires** line.
    let src = "http::geturl $url\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 6, &analysis, Some(&registry)).expect("hover");
    assert!(
        h.value.contains("**Requires**: `package require http`"),
        "{}",
        h.value
    );
}

#[test]
fn builtin_hover_omits_requires_when_package_imported() {
    // With `package require http` present, the hint is suppressed.
    let src = "package require http\nhttp::geturl $url\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 6, &analysis, Some(&registry)).expect("hover");
    assert!(!h.value.contains("**Requires**"), "{}", h.value);
}

#[test]
fn builtin_hover_resolves_imported_command() {
    // A bare command imported into the global scope resolves
    // to its qualified spec — hovering `test` after `namespace import
    // ::tcltest::*` surfaces the `tcltest::test` documentation.
    let src = "namespace import ::tcltest::*\ntest t-1 {desc} -body { set x 1 } -result 1\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 2, &analysis, Some(&registry)).expect("hover on bare `test`");
    assert!(
        h.value.contains("tcltest::test"),
        "bare imported `test` must hover as tcltest::test: {}",
        h.value
    );
}

#[test]
fn builtin_hover_bare_test_without_import_is_none() {
    // Control: without an import, a bare `test` is not a known command.
    let src = "test t-1 {desc}\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(hover(src, 0, 0, &analysis, Some(&registry)).is_none());
}

#[test]
fn builtin_hover_ignores_nested_namespace_import() {
    // An import made inside `namespace eval ns { … }` is not in scope at top
    // level, so hovering a bare top-level `test` must NOT resolve to
    // tcltest::test (mirrors the analyser's scope rule).
    let src = "namespace eval ns { namespace import ::tcltest::* }\ntest t-1 {desc}\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    // `test` is on line 1 (top level) — out of the `ns` import's scope.
    assert!(
        hover(src, 1, 2, &analysis, Some(&registry)).is_none(),
        "a nested-namespace import must not resolve a top-level bare `test`",
    );
}

#[test]
fn builtin_hover_ignores_import_after_cursor() {
    // Source order: hovering a `test` that appears *before* the
    // `namespace import` must not resolve — the import is not yet in effect.
    let src = "test t-1 {desc}\nnamespace import ::tcltest::*\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(
        hover(src, 0, 2, &analysis, Some(&registry)).is_none(),
        "an import after the cursor must not retroactively resolve `test`",
    );
}

#[test]
fn valid_events_for_asm_profile_requirement() {
    use tcl_registry::events::EventRequires;
    // A command requiring the ASM profile is valid only in ASM events.
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &["ASM"],
        also_in: &[],
        flow: false,
    };
    let events = valid_events(&requires, EventRelease::Unconfigured);
    assert_eq!(
        events,
        vec![
            "ASM_REQUEST_BLOCKING",
            "ASM_REQUEST_DONE",
            "ASM_REQUEST_VIOLATION",
            "ASM_RESPONSE_LOGIN",
            "ASM_RESPONSE_VIOLATION",
        ],
        "ASM-profile events"
    );
}

#[test]
fn valid_events_uses_transitive_profile_expansion() {
    use tcl_registry::events::EventRequires;
    // `HTTP`-profile requirement matches HTTP events directly *and*
    // events whose implied profile transitively requires HTTP (e.g.
    // `ADAPT_REQUEST_HEADERS` via `REQUESTADAPT` → `HTTP`).  This
    // exercises `expand_profile_stack`.
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: Some("tcp"),
        profiles: &["FASTHTTP", "HTTP"],
        also_in: &[],
        flow: false,
    };
    let events = valid_events(&requires, EventRelease::Unconfigured);
    assert!(events.iter().any(|e| e == "HTTP_REQUEST"), "{events:?}");
    assert!(
        events.iter().any(|e| e == "ADAPT_REQUEST_HEADERS"),
        "transitive-profile event missing (expansion broken): {events:?}"
    );
}

#[test]
fn valid_events_apply_the_configured_release_lifecycle() {
    use tcl_registry::events::EventRequires;
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &["XML"],
        also_in: &[],
        flow: false,
    };
    let unconfigured = valid_events(&requires, EventRelease::Unconfigured);
    assert!(unconfigured.iter().any(|e| e == "XML_BEGIN_DOCUMENT"));
    let old = valid_events(&requires, EventRelease::Configured("9.4.0"));
    assert!(old.iter().any(|e| e == "XML_BEGIN_DOCUMENT"), "{old:?}");
    let current = valid_events(&requires, EventRelease::Configured("16.1.0"));
    assert!(
        !current.iter().any(|e| e == "XML_BEGIN_DOCUMENT"),
        "retired at 10.0.0: {current:?}"
    );

    let mut text = String::new();
    append_valid_events(&mut text, &requires, EventRelease::Configured("9.4.0"));
    assert!(text.contains("**Valid events** (BIG-IP 9.4.0)"), "{text}");
    let mut text = String::new();
    append_valid_events(&mut text, &requires, EventRelease::Unconfigured);
    assert!(text.contains("no BIG-IP release configured"), "{text}");
}

#[test]
fn hover_valid_events_follow_the_documents_configured_release() {
    use tcl_compiler::analyser::Analyser;
    for (version, retired_listed) in [("9.4.0", true), ("16.1.0", false)] {
        let analysis = Analyser::new()
            .with_bigip_version(Some(version.to_owned()))
            .analyse("", "f5-irules");
        let context = analysis
            .resolved_input
            .as_ref()
            .expect("an iRules analysis retains its input")
            .availability_context();
        let release = EventRelease::of(context);
        assert_eq!(release, EventRelease::Configured(version));
        let events = valid_events(
            &tcl_registry::events::EventRequires {
                client_side: false,
                server_side: false,
                transport: None,
                profiles: &["XML"],
                also_in: &[],
                flow: false,
            },
            release,
        );
        assert_eq!(
            events.iter().any(|e| e == "XML_BEGIN_DOCUMENT"),
            retired_listed,
            "{version}: {events:?}"
        );
    }
}

#[test]
fn expand_profile_stack_includes_parents() {
    let reg = tcl_registry::profiles::ProfileRegistry::build();
    let stack = reg.expand_profile_stack(&["HTTP"]);
    assert!(stack.contains("HTTP"), "{stack:?}");
    assert!(stack.contains("TCP"), "HTTP should require TCP: {stack:?}");
}

#[test]
fn irules_command_hover_lists_valid_events() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let text =
        builtin_command_hover_text(&registry, "ASM::is_authenticated", &analyse(""), u32::MAX)
            .expect("registered iRules command should have hover");
    assert!(text.contains("**Valid events**"), "{text}");
    assert!(text.contains("ASM_REQUEST_BLOCKING"), "{text}");
    assert!(text.contains("**Requires**: profile ASM"), "{text}");
}

#[test]
fn irules_namespace_backed_profiles_injected_into_hover() {
    // `DIAMETER::retransmission_default` carries `event_requires` with
    // *empty* profiles; `effective_event_requires` substitutes the
    // `DIAMETER` namespace's profiles, so the hover shows a profile
    // **Requires** line (none would appear without the injection).
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let text = builtin_command_hover_text(
        &registry,
        "DIAMETER::retransmission_default",
        &analyse(""),
        u32::MAX,
    )
    .expect("hover");
    assert!(
        text.contains("**Requires**: profile DIAMETER or DIAMETERSESSION or DIAMETER_ENDPOINT"),
        "{text}"
    );
}

#[test]
fn effective_event_requires_injects_namespace_profiles() {
    // Unit check: empty own-profiles + a known `NS::` prefix → the
    // namespace's profiles; a non-namespaced name is unchanged.
    let base = tcl_registry::events::EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &[],
        also_in: &[],
        flow: false,
    };
    let descriptor = |name| tcl_registry::CommandSpec {
        name,
        surface: Some(tcl_dialect::model::SpecSurface::IRULES),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let eff = super::effective_event_requires(&descriptor("DIAMETER::foo"), &base);
    assert_eq!(
        eff.profiles,
        &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"]
    );
    // No `::` qualifier → unchanged (still empty).
    assert_eq!(
        super::effective_event_requires(&descriptor("puts"), &base).profiles,
        [] as [&str; 0]
    );
    // Already-populated profiles are left as-is.
    let with = tcl_registry::events::EventRequires {
        profiles: &["HTTP"],
        ..base
    };
    assert_eq!(
        super::effective_event_requires(&descriptor("HTTP::uri"), &with).profiles,
        &["HTTP"]
    );
}

#[test]
fn builtin_hover_omits_requires_for_core_command() {
    // A core built-in (`lindex`) needs no package — no hint.
    let src = "lindex $l 0\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
    assert!(!h.value.contains("**Requires**"), "{}", h.value);
}

#[test]
fn proc_hover_text_formats_default_param() {
    let src = "proc greet {{name world}} { puts $name }\n";
    let analysis = analyse(src);
    let proc_def = analysis.all_procs.values().next().unwrap();
    let text = proc_hover_text(proc_def);
    assert!(text.contains("{name world}"), "got: {text}");
}

/// The cross-document renderer answers by qualified name and
/// produces the *same* body the in-document path does, for both a proc and
/// a class — one renderer, so a call-site hover in another file can never
/// drift from the declaration's own.
#[test]
fn qualified_symbol_hover_matches_the_in_document_rendering_1018() {
    let src = "namespace eval ::math {}\nnamespace eval ::geo {}\nproc ::math::zzqfrobnicate {val args} { return $val }\noo::class create ::geo::Plane {\n    method fly {} {}\n}\n";
    let analysis = analyse(src);

    let proc_hover =
        qualified_symbol_hover(&analysis, "::math::zzqfrobnicate").expect("proc hover");
    assert_eq!(
        proc_hover.value,
        proc_hover_text(&analysis.all_procs["::math::zzqfrobnicate"]),
    );
    assert!(proc_hover.value.contains("val"), "{proc_hover:?}");

    let class_hover = qualified_symbol_hover(&analysis, "::geo::Plane").expect("class hover");
    assert_eq!(
        class_hover.value,
        class_hover_text(&analysis, &analysis.all_classes["::geo::Plane"]),
    );

    // A name neither map holds resolves to nothing — the fallback never
    // invents a symbol.
    assert!(qualified_symbol_hover(&analysis, "::math::nosuchthing").is_none());
    // The lookup is by *qualified* name only; a bare tail is not a match,
    // so a same-tailed proc in an unrelated namespace cannot be picked up.
    assert!(qualified_symbol_hover(&analysis, "zzqfrobnicate").is_none());
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

/// `pos_of` — (line, character) of the `occurrence`-th `needle`.
fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
    let mut start = 0;
    for _ in 0..occurrence {
        let idx = src[start..].find(needle).expect("needle not found") + start;
        start = idx + 1;
    }
    let idx = start - 1;
    let prefix = &src[..idx];
    let line = u32::try_from(prefix.matches('\n').count()).unwrap();
    let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
    (line, col)
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

#[test]
fn method_hover_notes_inheritance_and_override() {
    let src = "oo::class create A {\n    method greet {} {}\n}\noo::class create B {\n    superclass A\n    method greet {} {}\n}\noo::class create C {\n    superclass A\n}\n";
    let analysis = analyse(src);
    // B::greet overrides A::greet.
    let over = oo_method_resolution_note(&analysis, "::B", "greet").unwrap_or_default();
    assert!(
        over.contains("overrides") && over.contains("::A::greet"),
        "{over}"
    );
    // C inherits greet from A.
    let inh = oo_method_resolution_note(&analysis, "::C", "greet").unwrap_or_default();
    assert!(
        inh.contains("inherited from") && inh.contains("::A"),
        "{inh}"
    );
    // A::greet defined only here — no note.
    assert!(oo_method_resolution_note(&analysis, "::A", "greet").is_none());
}

#[test]
fn var_hover_text_renders_reference_count() {
    let src = "set x 1\nset y $x\nset z $x\n";
    let analysis = analyse(src);
    let var_def = analysis
        .global_scope
        .variables
        .get("x")
        .expect("x recorded");
    let text = var_hover_text(var_def, None, None, None);
    assert!(text.contains("**Variable** `x`"), "{}", text);
    assert!(text.contains("reference"), "{}", text);
}

#[test]
fn var_hover_text_appends_intrep_and_taint() {
    let var_def = VarDef {
        name: "x".to_owned(),
        definition_span: tcl_lexer::Span::new(0, 1),
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    };
    let text = var_hover_text(&var_def, Some("int"), Some("tainted (from I/O)"), None);
    assert!(text.contains("**Inferred intrep**: int"), "{text}");
    assert!(text.contains("**Taint**: tainted (from I/O)"), "{text}");
}

#[test]
fn var_hover_text_appends_proven_format_info() {
    let var_def = VarDef {
        name: "fmt".to_owned(),
        definition_span: tcl_lexer::Span::new(0, 1),
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    };
    let text = var_hover_text(&var_def, None, None, Some("**Format string**"));
    assert!(text.contains("**Variable** `fmt`"), "{text}");
    assert!(text.contains("**Format string**"), "{text}");
}

#[test]
fn special_var_hover_documents_auto_path() {
    // `$auto_path` has no user definition, but the special-variable
    // registry provides documentation on hover.
    let src = "puts $auto_path\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 8, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("special variable"), "{}", h.value);
    assert!(h.value.contains("auto-loader"), "{}", h.value);
}

#[test]
fn special_var_hover_is_dialect_aware() {
    let analysis = analyse_for("puts $auto_path\n", "f5-irules");
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    // iRules provides no `auto_path`, so no special-var hover fires there.
    assert!(
        hover_with_profile(
            "puts $auto_path\n",
            0,
            8,
            &analysis,
            Some(&registry),
            tcl_dialect::DialectProfile::irules(),
        )
        .is_none()
    );

    // `tcl_platform` exists under iRules and is CMP-unsafe as a plain
    // global — the hover surfaces the `static::` guidance.
    let src = "set x $tcl_platform(os)\n";
    let analysis = analyse_for(src, "f5-irules");
    let h = hover_with_profile(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        tcl_dialect::DialectProfile::irules(),
    )
    .expect("hover");
    assert!(h.value.contains("special array"), "{}", h.value);
    assert!(h.value.contains("CMP"), "{}", h.value);
    assert!(h.value.contains("static::tcl_platform"), "{}", h.value);
}

// clock format hover

#[test]
fn scan_clock_specifiers_finds_each_specifier() {
    let s = scan_clock_specifiers("%Y-%m-%d %H:%M:%S");
    assert_eq!(s, vec!["%Y", "%m", "%d", "%H", "%M", "%S"]);
}

#[test]
fn scan_clock_specifiers_handles_locale_prefix() {
    let s = scan_clock_specifiers("%EY-%Om");
    assert_eq!(s, vec!["%EY", "%Om"]);
}

#[test]
fn scan_clock_specifiers_handles_literal_percent() {
    let s = scan_clock_specifiers("100%% complete");
    assert_eq!(s, vec!["%%"]);
}

#[test]
fn clock_format_hover_renders_specifier_table() {
    let text = clock_format_hover_text("%Y-%m-%d");
    assert!(text.contains("**Clock format string**"), "{text}");
    assert!(text.contains("| `%Y` | 4-digit year |"), "{text}");
    assert!(text.contains("| `%m` | Month (01–12) |"), "{text}");
    assert!(text.contains("| `%d` | Day of month (01–31) |"), "{text}");
}

#[test]
fn clock_format_hover_marks_locale_modified_specifiers() {
    let text = clock_format_hover_text("%EY");
    assert!(text.contains("(locale-modified)"), "{text}");
}

#[test]
fn clock_format_hover_handles_empty_format() {
    let text = clock_format_hover_text("no specifiers here");
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_clock_format_specifier() {
    let src = "clock format $time -format {%Y-%m-%d}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the format literal.
    let h = hover(src, 0, 30, &analysis, None).expect("hover");
    assert!(
        h.value.contains("Clock format string"),
        "expected clock hover, got: {value}",
        value = h.value,
    );
}

#[test]
fn quoted_format_words_use_the_production_hover_path() {
    for (source, needle, expected) in [
        (
            "clock format $time -format \"%Y\"\n",
            "%Y",
            "Clock format string",
        ),
        ("format \"%d\" 42\n", "%d", "Format string"),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let column = u32::try_from(source.find(needle).expect("format word")).unwrap();
        let hover = hover(source, 0, column, &analysis, None).expect("format hover");
        assert!(hover.value.contains(expected), "{source:?}: {hover:?}");
    }
}

#[test]
fn unrelated_quoted_words_have_no_format_hover() {
    for source in ["puts \"%Y\"\n", "puts \"%d\"\n"] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let column = u32::try_from(source.find('%').expect("quoted word")).unwrap();
        let result = hover(source, 0, column, &analysis, None);
        assert!(
            result
                .as_ref()
                .is_none_or(|hover| !hover.value.contains("Format string")),
            "{source:?}: {result:?}"
        );
    }
}

#[test]
fn hover_abstains_for_local_proc_shadowing_format_command() {
    let src = "proc clock {args} { return 0 }\nclock format $time {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    assert!(hover(src, 1, 25, &analysis, None).is_none());
}

#[test]
fn hover_abstains_after_static_rename_moves_builtin_away() {
    let src = "rename clock saved\nclock format $time {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let cursor = u32::try_from(src.find("%Y").unwrap() - src.find('\n').unwrap() - 1).unwrap();
    assert!(hover(src, 1, cursor, &analysis, None).is_none());
}

#[test]
fn hover_uses_effective_registry_identity_after_rename_and_alias() {
    let renamed = "rename format local_format\nlocal_format {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(renamed, "tcl8.6").clone();
    let cursor =
        u32::try_from(renamed.find("%Y").unwrap() - renamed.find('\n').unwrap() - 1).unwrap();
    let h = hover(renamed, 1, cursor, &analysis, None).expect("renamed format hover");
    assert!(h.value.contains("**Format string**"), "{}", h.value);

    let aliased = "interp alias {} local_format {} format\nlocal_format {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(aliased, "tcl8.6").clone();
    let cursor =
        u32::try_from(aliased.find("%Y").unwrap() - aliased.find('\n').unwrap() - 1).unwrap();
    let h = hover(aliased, 1, cursor, &analysis, None).expect("aliased format hover");
    assert!(h.value.contains("**Format string**"), "{}", h.value);
}

#[test]
fn pattern_and_format_hover_descend_through_every_registry_executable_region() {
    // The cursor offsets deliberately sit in separately nested source
    // slices.  A line-local or re-based walker finds the command but
    // reports against the wrong source position; all must retain the
    // original document's absolute coordinates.
    let cases = [
        (
            "proc p {} { format {%04d} 1 }\n",
            "%04d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "oo::class create C { method m {} { string match {*needle*} x } }\n",
            "*needle*",
            "**Glob pattern**",
            "tcl8.6",
        ),
        (
            "switch $x { a { format {%03d} 1 } default { puts no } }\n",
            "%03d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "switch $x { a \"format {%08d} 1\" default { puts no } }\n",
            "%08d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "expect { -re {ready} \"format {%09d} 1\" }\n",
            "%09d",
            "**Format string**",
            "expect",
        ),
        (
            "apply {{} {format {%02d} 1}}\n",
            "%02d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "puts [format {%01d} 1]\n",
            "%01d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "when HTTP_REQUEST { format {%05d} 1 }\n",
            "%05d",
            "**Format string**",
            "f5-irules",
        ),
    ];
    for (source, needle, expected, dialect) in cases {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect).clone();
        let (line, column) = position_of(source, needle);
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let hover = hover_with_profile(source, line, column, &analysis, None, profile)
            .unwrap_or_else(|| panic!("no hover for {dialect}: {source}"));
        assert!(hover.value.contains(expected), "{}", hover.value);
    }
}

#[test]
fn quoted_case_action_hover_obeys_head_mutations_and_static_list_validation() {
    let aliased = "interp alias {} fmt {} format\nswitch $x { a \"fmt {%10d} 1\" }\n";
    let analysis = analyse(aliased);
    let (line, column) = position_of(aliased, "%10d");
    let result = hover(aliased, line, column, &analysis, None).expect("aliased quoted action");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );

    for source in [
        "rename format saved\nswitch $x { a \"format {%11d} 1\" }\n",
        "set actions { a \"format {%12d} 1\" }\nswitch $x $actions\n",
        "switch $x { a \"format {%13d} 1\" orphan }\n",
    ] {
        let analysis = analyse(source);
        let (line, column) = position_of(source, "%");
        assert!(
            hover(source, line, column, &analysis, None).is_none(),
            "mutated, dynamic, or malformed case list leaked a nested hover: {source}"
        );
    }
}

#[test]
fn nested_format_hover_honours_aliases_and_shadowing() {
    let aliased = "interp alias {} fmt {} format\nproc p {} { fmt {%06d} 1 }\n";
    let analysis = analyse(aliased);
    let (line, column) = position_of(aliased, "%06d");
    let result = hover(aliased, line, column, &analysis, None).expect("aliased nested format");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );

    // A same-named proc owns the nested call, so the builtin format
    // grammar must not leak through merely because its spelling matches.
    let shadowed = "proc format {args} {}\nproc p {} { format {%07d} 1 }\n";
    let analysis = analyse(shadowed);
    let (line, column) = position_of(shadowed, "%07d");
    assert!(hover(shadowed, line, column, &analysis, None).is_none());
}

#[test]
fn nested_substitution_hover_beats_its_outer_pattern_word() {
    let source = "regexp \"[format {%d} 1]\" $value\n";
    let analysis = analyse(source);
    let (line, column) = position_of(source, "%d");
    let result = hover(source, line, column, &analysis, None)
        .expect("format hover inside regexp substitution");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );
    assert!(
        !result.value.contains("**Regular expression**"),
        "outer regexp incorrectly claimed nested format: {}",
        result.value
    );
}

// sprintf format hover

#[test]
fn scan_sprintf_specifiers_finds_basic_types() {
    let s = scan_sprintf_specifiers("%d - %s : %x");
    assert_eq!(s, vec!["%d", "%s", "%x"]);
}

#[test]
fn scan_sprintf_specifiers_captures_width_and_precision() {
    let s = scan_sprintf_specifiers("%05d %-10s %.3f");
    assert_eq!(s, vec!["%05d", "%-10s", "%.3f"]);
}

#[test]
fn scan_sprintf_specifiers_captures_positional() {
    let s = scan_sprintf_specifiers("%1$s %2$d");
    assert_eq!(s, vec!["%1$s", "%2$d"]);
}

#[test]
fn scan_sprintf_specifiers_handles_literal_percent() {
    let s = scan_sprintf_specifiers("%% done");
    assert_eq!(s, vec!["%%"]);
}

#[test]
fn sprintf_format_hover_renders_specifier_table() {
    let text = sprintf_format_hover_text("%d - %s");
    assert!(text.contains("**Format string** (sprintf-style)"), "{text}");
    assert!(text.contains("| `%d` | Signed decimal integer |"), "{text}");
    assert!(text.contains("| `%s` | String |"), "{text}");
}

#[test]
fn sprintf_format_hover_handles_empty_format() {
    let text = sprintf_format_hover_text("no specifiers here");
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_sprintf_specifier() {
    let src = "format {%d items} $count\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 10, &analysis, None).expect("hover");
    assert!(
        h.value.contains("Format string"),
        "expected sprintf hover, got: {value}",
        value = h.value,
    );
}

// binary format hover

fn binary_ctx(text: &str) -> BinaryContext {
    BinaryContext {
        text: text.to_string(),
        subcmd: "format".to_string(),
        args: Vec::new(),
    }
}

#[test]
fn scan_binary_fields_finds_basic_types() {
    let fields = scan_binary_fields("a4 H2 i");
    let fulls: Vec<&str> = fields.iter().map(|f| f.full.as_str()).collect();
    assert_eq!(fulls, vec!["a4", "H2", "i"]);
    assert_eq!(fields[0].byte_size, Some(4));
    assert_eq!(fields[1].byte_size, Some(1));
    assert_eq!(fields[2].byte_size, Some(4));
}

#[test]
fn scan_binary_fields_handles_star_count() {
    let fields = scan_binary_fields("a* I*");
    assert!(fields[0].star);
    assert!(fields[1].star);
    assert_eq!(fields[0].byte_size, None);
    assert_eq!(fields[1].byte_size, None);
}

#[test]
fn binary_format_hover_renders_summary_and_detail_table() {
    let text = binary_format_hover_text(&binary_ctx("a4 i"));
    assert!(text.contains("**binary format**"), "{text}");
    assert!(text.contains("2 fields"), "{text}");
    assert!(text.contains("8 bytes"), "{text}");
    // Detail table now has 4 columns and a Bytes column.
    assert!(
        text.contains("| Spec | Variable | Type | Bytes |"),
        "{text}"
    );
    assert!(
        text.contains("| `a4` | a4 | str (null-pad) | 4 |"),
        "{text}"
    );
    assert!(text.contains("| `i` | i | int32 LE | 4 |"), "{text}");
}

#[test]
fn binary_format_hover_renders_byte_ruler_diagram() {
    let text = binary_format_hover_text(&binary_ctx("c s i"));
    // Diagram fenced in a code block.
    assert!(text.contains("```"), "{text}");
    // Box-drawing characters for the field boundaries.
    assert!(text.contains('┌'), "{text}");
    assert!(text.contains('┬'), "{text}");
    assert!(text.contains('┐'), "{text}");
    // Numeric ruler — 7 bytes total (1 + 2 + 4).
    assert!(text.contains("0   1"), "{text}");
}

#[test]
fn binary_format_hover_omits_diagram_when_total_exceeds_32_bytes() {
    // `d` is 8 bytes; five of them = 40 bytes — over the
    // 32-byte diagram budget.
    let text = binary_format_hover_text(&binary_ctx("d5"));
    assert!(text.contains("**binary format**"), "{text}");
    assert!(!text.contains('┌'), "diagram should be skipped: {text}");
}

#[test]
fn binary_format_hover_omits_diagram_when_size_unknown() {
    // `a*` has unknown byte count.
    let text = binary_format_hover_text(&binary_ctx("a*"));
    assert!(!text.contains('┌'), "{text}");
    // The Bytes column still renders `…` for star fields.
    assert!(
        text.contains("| `a*` | a* | str (null-pad) | … |"),
        "{text}"
    );
}

#[test]
fn binary_format_hover_labels_fields_with_arg_names() {
    let ctx = BinaryContext {
        text: "c i".to_string(),
        subcmd: "scan".to_string(),
        args: vec!["byte".to_string(), "word".to_string()],
    };
    let text = binary_format_hover_text(&ctx);
    // Detail-table Variable column gets the real names.
    assert!(text.contains("| `c` | byte |"), "{text}");
    assert!(text.contains("| `i` | word |"), "{text}");
    // Ruler diagram labels also pick up the names.
    assert!(text.contains("byte"), "{text}");
    assert!(text.contains("word"), "{text}");
}

#[test]
fn binary_format_hover_renders_uint_modifier() {
    let text = binary_format_hover_text(&binary_ctx("iu"));
    assert!(text.contains("uint32"), "{text}");
}

#[test]
fn binary_format_hover_no_specifiers_returns_friendly_message() {
    let text = binary_format_hover_text(&binary_ctx("ZZZ"));
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_binary_specifier() {
    let src = "binary format {a4 i} val\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 17, &analysis, None).expect("hover");
    assert!(h.value.contains("binary format"), "{}", h.value);
}

#[test]
fn binary_scan_hover_uses_variable_names_from_parsed_words() {
    let source = "binary scan $buf \"cI\" byte word\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    let column = u32::try_from(source.find("cI").expect("format word")).unwrap();
    let hover = hover(source, 0, column, &analysis, None).expect("binary scan hover");
    assert!(hover.value.contains("binary scan"), "{hover:?}");
    assert!(hover.value.contains("byte"), "{hover:?}");
    assert!(hover.value.contains("word"), "{hover:?}");
}

#[test]
fn hover_fires_for_bare_binary_format_specifier() {
    // Tcl accepts an unquoted format word.  The registry identifies it as
    // the `FormatString`; hover must not require braces or quotes.
    let src = "binary format c2s value\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 15, &analysis, None).expect("hover");
    assert!(h.value.contains("binary format"), "{}", h.value);
}

#[test]
fn hover_uses_lsearchs_registry_selected_pattern_language() {
    // `-regexp` changes lsearch's embedded language, while an exact or
    // abbreviated option still shifts list/pattern positions through the
    // registry's OptionSpec grammar.
    let src = "lsearch -regexp {a b} {a+}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 24, &analysis, None).expect("hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
}

#[test]
fn dynamic_lsearch_option_prefix_does_not_claim_the_following_list_as_glob() {
    let src = "lsearch $mode {a b} {a*}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // `$mode` can evaluate to a value-taking lsearch option. The source
    // shape therefore cannot prove that `{a b}` is the pattern word.
    assert!(hover(src, 0, 16, &analysis, None).is_none());
}

/// A substituted leading word is not a positional operand until Tcl has
/// evaluated it.  The source-aware registry query must therefore keep
/// every resolver-owned pattern and regsub replacement unclaimed, not
/// merely the lsearch-specific descriptor.
#[test]
fn dynamic_leading_options_abstain_for_pattern_and_regsub_format_hovers() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for (src, needle) in [
        ("regexp $mode {a+} $value\n", "a+"),
        ("glob $mode /tmp {*.tcl}\n", "*.tcl"),
        ("regsub $mode {a} $value {\\1}\n", "\\1"),
        ("regsub -c {a} $value {\\1}\n", "\\1"),
        ("regsub -command {a} $value callback\n", "callback"),
    ] {
        let analysis = analyse(src);
        let (line, character) = position_of(src, needle);
        assert!(
            !hover(src, line, character, &analysis, Some(&registry)).is_some_and(|found| {
                found.value.contains("Regex pattern")
                    || found.value.contains("Glob pattern")
                    || found.value.contains("Substitution spec")
            }),
            "{src:?}: an unresolved, invalid, or callback-leading layout must not claim {needle:?} as an embedded language",
        );
    }

    let src = "regsub -start $start {a} $value {\\1}\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "\\1");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("Substitution spec"),
        "a declared -start value has a fixed width: {}",
        found.value
    );
}

#[test]
fn hover_explains_a_computed_format_string() {
    // A literal `format "%-20s %d" a 1` already explains its
    // specifiers; the lattice proves the same text when it reaches the
    // word through a variable, so the computed spelling must not go
    // blind.
    let registry = tcl_registry::CommandRegistry::build_default();
    let src = "set fmt \"%-20s %d\"\nformat $fmt a 1\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "$fmt");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("**Format string** (sprintf-style)"),
        "{}",
        found.value
    );
    assert!(
        found.value.contains("| `%d` | Signed decimal integer |"),
        "{}",
        found.value
    );

    // Negative: an unknown `$fmt` (a bare parameter, never assigned a
    // literal) proves nothing, so the plain variable card shows with no
    // format-string section appended.
    let src = "proc p {fmt} {\n    format $fmt a 1\n}\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "$fmt");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("**Variable** `fmt`"),
        "{}",
        found.value
    );
    assert!(
        !found.value.contains("Format string"),
        "an unproven $fmt must not draw a format-string hover: {}",
        found.value
    );
}

#[test]
fn lsearch_pattern_hover_uses_the_documents_profiled_option_set() {
    // Tcl 9 makes -str the unique -stride abbreviation. Before 9 it is
    // not an lsearch option, so the preceding -regexp cannot claim the
    // final word as a regex pattern. This reaches the profile-aware
    // PatternArgResolver path, not a command-name hover branch.
    let src = "lsearch -regexp -str 2 {a b} {a+}\n";
    let cursor = u32::try_from(src.rfind("a+").expect("final pattern")).unwrap();
    for (dialect, expected) in [("tcl8.6", false), ("tcl9.0", true)] {
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let analysis = analyser.analyse(src, dialect).clone();
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let found = hover_with_profile(
            src,
            0,
            cursor,
            &analysis,
            Some(&tcl_registry::CommandRegistry::build_default()),
            profile,
        );
        assert_eq!(
            found.is_some_and(|hover| hover.value.contains("Regex pattern")),
            expected,
            "{dialect}: profile-filtered lsearch options decide the pattern position"
        );
    }
}

// regsub substitution-spec hover

#[test]
fn scan_regsub_backrefs_finds_each_backref() {
    let r = scan_regsub_backrefs("\\1-\\2 (\\& and \\0)");
    assert_eq!(r, vec!["\\1", "\\2", "\\&", "\\0"]);
}

#[test]
fn regsub_hover_renders_backref_table() {
    let text = regsub_hover_text("prefix \\1 suffix");
    assert!(text.contains("**Substitution spec**"), "{text}");
    assert!(text.contains("| `\\1` | First capture group |"), "{text}");
}

#[test]
fn regsub_hover_handles_no_backrefs() {
    let text = regsub_hover_text("plain text");
    assert!(text.contains("No backreferences found"), "{text}");
}

#[test]
fn hover_fires_for_regsub_backref() {
    let src = "regsub foo bar {\\1-baz} out\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 18, &analysis, None).expect("hover");
    assert!(h.value.contains("Substitution spec"), "{}", h.value);
}

// glob pattern hover

#[test]
fn scan_glob_metachars_finds_star_and_question() {
    let m = scan_glob_metachars("*.tcl");
    let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"*"), "{m:?}");
}

#[test]
fn scan_glob_metachars_finds_character_class() {
    let m = scan_glob_metachars("[abc]*.tcl");
    let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"[abc]"), "{m:?}");
    assert!(toks.contains(&"*"), "{m:?}");
}

#[test]
fn glob_hover_renders_table() {
    let text = glob_hover_text("*.tcl");
    assert!(text.contains("**Glob pattern**"), "{text}");
    assert!(text.contains("| `*` |"), "{text}");
}

#[test]
fn glob_hover_for_literal_string() {
    let text = glob_hover_text("plain");
    assert!(text.contains("Literal string"), "{text}");
}

#[test]
fn hover_fires_for_glob_pattern() {
    // Braced glob pattern — single-line literal detection
    // requires `"..."` or `{...}` delimiters.  Bare globs
    // (`glob *.tcl`) fall through to the proc / word
    // lookup; their support lives in the same multi-line
    // / arg-position machinery that other `*-rich`
    // sub-strips defer.
    let src = "glob {*.tcl}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the braced pattern.
    let h = hover(src, 0, 8, &analysis, None).expect("hover");
    assert!(h.value.contains("Glob pattern"), "{}", h.value);
}

// regex pattern hover

#[test]
fn scan_regex_components_finds_anchors_and_quantifiers() {
    let r = scan_regex_components("^foo.*$");
    let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"^"), "{r:?}");
    assert!(toks.contains(&"."), "{r:?}");
    assert!(toks.contains(&"*"), "{r:?}");
    assert!(toks.contains(&"$"), "{r:?}");
}

#[test]
fn scan_regex_components_finds_character_class() {
    let r = scan_regex_components("[a-z]+");
    let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"[a-z]"), "{r:?}");
    assert!(toks.contains(&"+"), "{r:?}");
}

#[test]
fn scan_regex_components_finds_escapes() {
    let r = scan_regex_components("\\d+");
    let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"\\d"), "{r:?}");
}

#[test]
fn scan_regex_components_finds_groups_and_lookahead() {
    let r = scan_regex_components("(?:foo)(?=bar)");
    let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"(?:"), "{r:?}");
    assert!(toks.contains(&"(?="), "{r:?}");
}

#[test]
fn regex_hover_renders_table() {
    let text = regex_hover_text("^foo$");
    assert!(text.contains("**Regex pattern**"), "{text}");
    assert!(text.contains("| `^` |"), "{text}");
}

#[test]
fn hover_fires_for_regex_pattern() {
    let src = "regexp {^foo.*$} $line\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the pattern literal.
    let h = hover(src, 0, 10, &analysis, None).expect("hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
}

#[test]
fn conditional_pattern_hover_keeps_source_and_method_owners() {
    for subject in ["$line", "line"] {
        let source = format!("regexp {{^foo.*$}} {subject}\n");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let selected = hover(&source, 0, 10, &analysis, None).expect("original regex pattern");
        assert!(
            selected.value.contains("Regex pattern"),
            "{}",
            selected.value
        );
        let changed = source.replace("^foo", "^bar");
        assert!(
            hover(&changed, 0, 10, &analysis, None).is_none(),
            "foreign source"
        );
    }
    for source in [
        "proc regexp {args} { return 0 }\nregexp {^foo.*$} $line\n",
        "rename regexp {}\noo::class create regexp {}\nregexp {^foo.*$} $line\n",
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let line = u32::try_from(source.lines().count() - 1).unwrap();
        assert!(
            hover(source, line, 10, &analysis, None)
                .is_none_or(|h| !h.value.contains("Regex pattern")),
            "shadowed command: {source}"
        );
    }
    let source = "oo::class create C {}\nC {^foo.*$} $line\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        hover(source, 1, 5, &analysis, None).is_none_or(|h| !h.value.contains("Regex pattern")),
        "OO selector is not a regexp source pattern"
    );
    let unresolved = "oo::class create C {method m {} {}}\nset obj unknown\n$obj m\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(unresolved, "tcl8.6");
    assert!(
        matches!(
            crate::method_symbol::local_candidate(unresolved, &analysis, 2, 5),
            std::ops::ControlFlow::Break(None)
        ),
        "real unknown object selector retains its method refusal"
    );
    assert!(
        hover(unresolved, 2, 5, &analysis, None).is_none(),
        "an unresolved OO selector has no source pattern fallback"
    );
}

#[test]
fn hover_abstains_for_local_proc_shadowing_regexp_command() {
    let src = "proc regexp {args} { return 0 }\nregexp {^foo$} $line\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    assert!(hover(src, 1, 10, &analysis, None).is_none());
}

// IP address hover

#[test]
fn ip_hover_classifies_private_ipv4() {
    let t = ip_address_hover_text("10.0.0.1").expect("hover");
    assert!(t.contains("IPv4 address"), "{t}");
    assert!(t.contains("Private (RFC 1918)"), "{t}");
}

#[test]
fn ip_hover_classifies_loopback() {
    let t = ip_address_hover_text("127.0.0.1").expect("hover");
    assert!(t.contains("Loopback"), "{t}");
}

#[test]
fn ip_hover_classifies_public_ipv4() {
    let t = ip_address_hover_text("8.8.8.8").expect("hover");
    assert!(t.contains("Public"), "{t}");
}

#[test]
fn ip_hover_renders_cidr_prefix() {
    let t = ip_address_hover_text("10.0.0.0/8").expect("hover");
    assert!(t.contains("CIDR network: `10.0.0.0/8`"), "{t}");
}

#[test]
fn ip_hover_classifies_ipv6_loopback() {
    let t = ip_address_hover_text("::1").expect("hover");
    assert!(t.contains("IPv6 address"), "{t}");
    assert!(t.contains("Loopback"), "{t}");
}

#[test]
fn ip_hover_detects_ipv4_mapped_ipv6() {
    let t = ip_address_hover_text("::ffff:192.0.2.1").expect("hover");
    assert!(t.contains("IPv4-mapped"), "{t}");
}

#[test]
fn ip_hover_rejects_non_ip_strings() {
    assert!(ip_address_hover_text("hello").is_none());
    assert!(ip_address_hover_text("256.256.256.256").is_none());
    assert!(ip_address_hover_text("not.an.ip.address").is_none());
}

#[test]
fn hover_fires_for_ip_address_word() {
    let src = "set host 10.0.0.1\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor on `10.0.0.1`.
    let h = hover(src, 0, 11, &analysis, None).expect("hover");
    assert!(h.value.contains("IPv4 address"), "{}", h.value);
}

// registry-driven hovers

#[test]
fn builtin_command_hover_surfaces_summary_from_registry() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let t = builtin_command_hover_text(&registry, "puts", &analyse(""), u32::MAX).expect("hover");
    assert!(t.contains("built-in command"), "{t}");
    assert!(t.contains("`puts`"), "{t}");
}

#[test]
fn builtin_command_hover_lists_subcommands() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let t = builtin_command_hover_text(&registry, "string", &analyse(""), u32::MAX).expect("hover");
    assert!(t.contains("Subcommands:"), "{t}");
    assert!(t.contains("length"), "{t}");
}

#[test]
fn builtin_command_hover_returns_none_for_unknown() {
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(
        builtin_command_hover_text(&registry, "totallyMadeUpCommand", &analyse(""), u32::MAX,)
            .is_none()
    );
}

fn retained_registry_text(source: &str, cursor: u32, environment: &str) -> Option<String> {
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-registry-cards";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment(environment).default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "presentation-only");
    original_registry::hover(source, &analysis, cursor).map(|hover| hover.value)
}

#[test]
fn subcommand_hover_surfaces_for_string_length() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text =
        retained_registry_text("string length $name\n", 10, "tcl8.6").expect("subcommand hover");
    assert!(text.contains("`string length`"), "{text}");
    assert!(text.contains("subcommand"), "{text}");
}

#[test]
fn subcommand_hover_resolves_unique_prefix_abbreviation() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("string le $name\n", 8, "tcl8.6").expect("prefix hover");
    assert!(text.contains("`string length`"), "{text}");
}

#[test]
fn subcommand_hover_prefix_is_dialect_aware() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "info class def ::C\n";
    assert!(
        retained_registry_text(source, 13, "tcl8.6")
            .is_some_and(|text| text.contains("`info class definition`"))
    );
    assert!(retained_registry_text(source, 13, "tcl9.0").is_none());
    let source = "string rev abc\n";
    assert!(retained_registry_text(source, 8, "tcl8.6").is_some());
    assert!(retained_registry_text(source, 8, "tcl8.4").is_none());
}

#[test]
fn subcommand_hover_skips_unknown_subcommand() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    assert!(retained_registry_text("string bogusSubcommand\n", 12, "tcl8.6").is_none());
}

#[test]
fn option_hover_resolves_subcommand_scoped_option() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("chan configure $chan -inputmode raw\n", 26, "tcl9.0")
        .expect("configure-scoped option");
    assert!(text.contains("`-inputmode`"), "{text}");
    assert!(text.contains("chan configure"), "{text}");
}

#[test]
fn option_hover_notes_dialect_unavailability() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "chan configure $chan -inputmode raw\n";
    let old = retained_registry_text(source, 26, "tcl8.6").expect("excluded exact option metadata");
    assert!(old.contains("Not available in the active dialect"), "{old}");
    let new = retained_registry_text(source, 26, "tcl9.0").expect("admitted option");
    assert!(
        !new.contains("Not available in the active dialect"),
        "{new}"
    );
}

#[test]
fn option_hover_falls_back_to_top_level_options_for_simple_command() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("lsearch -exact {a b} x\n", 11, "tcl8.6")
        .expect("root option metadata");
    assert!(text.contains("`-exact`"), "{text}");
    assert!(text.contains("of `lsearch`"), "{text}");
}

#[test]
fn sub_subcommand_hover_surfaces_for_info_object_class() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("info object class $obj\n", 14, "tcl8.6")
        .expect("nested subcommand");
    assert!(text.contains("`info object class`"), "{text}");
    assert!(text.contains("subcommand"), "{text}");
    let text =
        retained_registry_text("info class super $cls\n", 13, "tcl8.6").expect("nested prefix");
    assert!(text.contains("`info class superclasses`"), "{text}");
    // The first selector is described by its own retained subcommand row.
    let parent =
        retained_registry_text("info object class\n", 6, "tcl8.6").expect("parent selector");
    assert!(parent.contains("`info object`"), "{parent}");
    assert!(!parent.contains("`info object class`"), "{parent}");
}

#[test]
fn logical_registry_hover_keeps_actual_context_and_source_currency() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "string cat A B";
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-hover-context";
    let profile = profile.intern();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        analysis.command_invocations.clear();
        let answer = hover(source, 0, 8, &analysis, None);
        assert_eq!(answer.is_some(), expected, "{environment}: {answer:?}");
        assert!(hover("string cat C D", 0, 8, &analysis, None).is_none());
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(hover(source, 0, 8, &analysis, None).is_none());
    }
}

#[test]
fn logical_registry_hover_preserves_whole_alias_operands_and_declines_data() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    for (source, needle, expected) in [
        (
            "interp alias {} length {} string; length length value",
            "length value",
            "`string length`",
        ),
        (
            "interp alias {} rx {} regexp; rx -nocase pattern text",
            "-nocase",
            "option of `regexp`",
        ),
        (
            "proc p {} {string length value}",
            "length",
            "`string length`",
        ),
        (
            r"string \u006cength value",
            r"\u006cength",
            "`string length`",
        ),
    ] {
        let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
        let text = retained_registry_text(source, offset, "tcl8.6").expect(source);
        assert!(text.contains(expected), "{source}: {text}");
    }
    for (source, needle) in [
        (
            "interp alias {} rx {} regexp -nocase; rx pattern -nocase",
            "-nocase",
        ),
        ("puts {string length value}", "length"),
        ("proc string args {}; string length value", "length value"),
        ("string $selector value", "$selector"),
    ] {
        let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
        assert!(
            retained_registry_text(source, offset, "tcl8.6").is_none(),
            "{source}"
        );
    }
}

#[test]
fn hover_fires_for_builtin_command_with_registry() {
    let src = "puts hello\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("built-in command"), "{}", h.value);
}

#[test]
fn hover_fires_for_subcommand_with_registry() {
    let src = "string length $name\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 10, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("subcommand"), "{}", h.value);
}

// docstring formatting

#[test]
fn format_docstring_renders_brief_param_return_tags() {
    let raw = concat!(
        "@brief Greet someone\n",
        "Free-form description\n",
        "spanning two lines.\n",
        "@param name the person's name\n",
        "@param greeting optional greeting prefix\n",
        "@return the formatted greeting\n",
    );
    let rendered = format_docstring(raw);
    assert!(rendered.contains("Greet someone"), "{rendered}");
    assert!(rendered.contains("Free-form description"), "{rendered}");
    assert!(rendered.contains("**Parameters:**"), "{rendered}");
    assert!(
        rendered.contains("- **name** \u{2014} the person's name"),
        "{rendered}",
    );
    assert!(
        rendered.contains("- **greeting** \u{2014} optional greeting prefix"),
        "{rendered}",
    );
    assert!(
        rendered.contains("**Returns:** the formatted greeting"),
        "{rendered}",
    );
}

#[test]
fn format_docstring_drops_decoration_lines() {
    // Pure-decoration lines (`.....`, `-----`) shouldn't
    // pollute the description block.
    let raw = "..........\nA description.\n..........\n";
    let rendered = format_docstring(raw);
    assert_eq!(rendered, "A description.");
}

#[test]
fn format_docstring_passes_through_plain_text() {
    let raw = "Just a free-form description.\nNo tags here.\n";
    let rendered = format_docstring(raw);
    assert!(rendered.contains("Just a free-form description"));
    assert!(rendered.contains("No tags here"));
}

#[test]
fn format_docstring_handles_param_without_description() {
    let raw = "@param naked\n";
    let rendered = format_docstring(raw);
    assert!(
        rendered.contains("- **naked**"),
        "expected bare param entry; got {rendered}",
    );
    // No trailing em-dash since there's no description.
    assert!(!rendered.contains("**naked** \u{2014}"), "{rendered}");
}

// class-member hover

#[test]
fn class_member_hover_bare_sibling_method_without_link_abstains() {
    // FP — a bareword sibling method call is NOT
    // actually reachable from another method's body unless `link`
    // exposed it that way; real tclsh: "invalid command name". Must
    // abstain rather than falsely resolve.
    let src = "oo::class create C {\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` invocation (line 2, col 22).
    assert!(hover(src, 2, 22, &analysis, None).is_none());
}

#[test]
fn class_member_hover_linked_sibling_method_resolves() {
    // C9's selected link schema couples the alias and target name.
    let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method twice {} { greet ; greet }` — same text/col
    // as the un-linked test above, one line further down.
    let h = hover(src, 3, 22, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("C::greet"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
    assert!(!h.value.contains("linked from"), "{}", h.value);
}

#[test]
fn class_member_hover_resolves_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; the
    // `link`, the linked method, and the bareword call site that depends
    // on it are all added via a *separate*, later `oo::define Gadget
    // { ... }` block — the `ticklecharts::chart` shape. Hover on
    // the bareword `Helper` call must still resolve — the cursor sits
    // inside that separate block, which `class_member_hover_text`'s
    // `enclosing_class_at` containment check must recognise as part of
    // `Gadget`'s body too, not just the original creation block.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    constructor {} { link Helper }\n    method Helper {} { return hi }\n    method Caller {} { Helper }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 6: `    method Caller {} { Helper }` — cursor on the bareword
    // `Helper` call (col 23).
    let h = hover(src, 6, 23, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("Gadget::Helper"), "{}", h.value);
}

#[test]
fn class_member_hover_two_element_link_alias_notes_the_real_target() {
    // C9's two-field link keeps the alias distinct from its target member.
    let src = "oo::class create C {\n    constructor {} { link {shortcut realMethod} }\n    method realMethod {x} { return $x }\n    method bar {} { return [shortcut 42] }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method bar {} { return [shortcut 42] }` — col 28
    // lands on the `s` of `shortcut`.
    let h = hover(src, 3, 28, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("C::realMethod"), "{}", h.value);
    assert!(h.value.contains("linked from `shortcut`"), "{}", h.value);
}

// `my method` internal-dispatch hover

#[test]
fn my_dispatch_hover_resolves_a_plain_call_in_a_single_block_class() {
    // Explicit self-dispatch selects its member without a bareword link.
    let src = "oo::class create geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method WhichAmI {} { return [my GetType] }` — cursor
    // on `GetType` inside `my GetType` (col 38).
    let h = hover(src, 2, 38, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
}

#[test]
fn my_dispatch_hover_resolves_when_class_extended_via_separate_oo_define() {
    // TP — the two-block `oo::class create` + separate `oo::define`
    // pattern (constructor in `create`, every method including the
    // dispatched-on one in a later `define`), exercised here for hover
    // specifically; definition and references have their own tests for
    // the same shape.
    let src = "oo::class create geo::Plane {\n    constructor {args} {}\n}\noo::define geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
    let analysis = analyse(src);
    // Line 5: `    method WhichAmI {} { return [my GetType] }` — cursor
    // on `GetType` inside `my GetType` (col 38).
    let h = hover(src, 5, 38, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
}

#[test]
fn my_dispatch_hover_abstains_for_an_undefined_method() {
    // TN — `my` dispatching to a method the class genuinely doesn't
    // have must not fabricate a hover.
    let src = "oo::class create C {\n    method twice {} { return [my nope] }\n}\n";
    let analysis = analyse(src);
    // Line 1, cursor on `nope` inside `my nope` (col 34).
    assert!(hover(src, 1, 34, &analysis, None).is_none());
}

// A class member's own declaration hovers as that member, in the
// creation block and in a reopening `oo::define` block alike — the same
// way `proc` and the class name itself hover at their declarations.

#[test]
fn member_declaration_hover_resolves_in_the_creation_block() {
    // TP — the baseline: `method plain` hovers at its own name token.
    let src = "oo::class create Foo {\n    method plain {a} { return $a }\n}\n";
    let analysis = analyse(src);
    // Line 1, col 13 — inside `plain`.
    let h = hover(src, 1, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Foo::plain"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
}

#[test]
fn member_declaration_hover_resolves_in_a_reopening_oo_define_block() {
    // TP — the SpiceGenTcl
    // `generalClasses.tcl`: `oo::configurable create Parameter { … }`
    // then a separate `oo::define Parameter { method <WriteProp-value>
    // … }`).  tclsh 9.0.4 dispatches `$f reopened hi` to the reopened
    // block's method, so it is every bit `Foo`'s own member and must
    // hover identically to one written in the creation block.
    let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\noo::define Foo {\n    method reopened {val} { return $val }\n}\n";
    let analysis = analyse(src);
    // Line 4, col 13 — inside `reopened`, in the reopening block.
    let h = hover(src, 4, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Foo::reopened"), "{}", h.value);
    assert!(h.value.contains("1 param"), "{}", h.value);
}

#[test]
fn member_declaration_hover_covers_classmethods_and_properties() {
    // TP — the other two member tables render with their own labels.
    let src = "oo::configurable create C {\n    property size\n    self method make {n} { return $n }\n}\n";
    let analysis = analyse_for(src, "tcl9.0");
    let prop = hover(src, 1, 15, &analysis, None).expect("property hover");
    assert!(prop.value.contains("**property**"), "{}", prop.value);
    assert!(prop.value.contains("::C::size"), "{}", prop.value);
    let cm = hover(src, 2, 19, &analysis, None).expect("classmethod hover");
    assert!(cm.value.contains("**classmethod**"), "{}", cm.value);
    assert!(cm.value.contains("::C::make"), "{}", cm.value);
}

#[test]
fn member_declaration_hover_notes_an_override() {
    // TP — the declaration renders the same MRO note a call site does,
    // so the two descriptions of one member cannot disagree.
    let src = "oo::class create Base {\n    method run {} { return base }\n}\noo::class create Derived {\n    superclass Base\n    method run {} { return derived }\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 5, 12, &analysis, None).expect("hover");
    assert!(h.value.contains("::Derived::run"), "{}", h.value);
    assert!(h.value.contains("Base"), "MRO note expected: {}", h.value);
}

#[test]
fn member_declaration_hover_does_not_fire_on_a_same_named_word_elsewhere() {
    // FP guard — only the declaration's own name token hovers as the
    // member.  A same-spelled *data* word inside a method body is not a
    // member reference, and must keep falling through the tier order
    // (here: to nothing at all).
    let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\n";
    let analysis = analyse(src);
    // Line 1, col 30 — the bareword `plain` in `return plain`, which is
    // a value, not a call (an un-linked sibling name is not callable).
    assert!(hover(src, 1, 30, &analysis, None).is_none());
}

#[test]
fn member_declaration_hover_abstains_outside_a_class_body() {
    // TN — a proc whose name happens to match a method of some class in
    // the file must not borrow that class's member hover.
    let src =
        "oo::class create Foo {\n    method plain {} { return 1 }\n}\nproc plain {} { return 2 }\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 6, &analysis, None).expect("proc hover");
    assert!(
        !h.value.contains("**method**"),
        "the proc, not Foo's method: {}",
        h.value
    );
}

#[test]
fn class_member_hover_bare_sibling_classmethod_without_link_abstains() {
    // FP — same shape as the method case above,
    // for `classmethod`.
    let src =
        "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = analyse(src);
    assert!(hover(src, 2, 20, &analysis, None).is_none());
}

#[test]
fn class_member_hover_linked_sibling_classmethod_resolves() {
    // C9's link retains the selected classmethod's member side.
    let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method use {} { factory }` — same text/col as the
    // un-linked test above, one line further down.
    let h = hover(src, 3, 20, &analysis, None).expect("hover");
    assert!(h.value.contains("**classmethod**"), "{}", h.value);
    assert!(h.value.contains("C::factory"), "{}", h.value);
}

#[test]
fn class_member_hover_fires_for_constructor_keyword() {
    let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 2, 27, &analysis, None).expect("hover");
    assert!(h.value.contains("constructor"), "{}", h.value);
    // Class qualified name is `::C`.
    assert!(h.value.contains("::C"), "{}", h.value);
}

#[test]
fn class_member_hover_skipped_outside_class_body() {
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the bare `greet` outside the class body.
    // No proc / class / method match — should return None.
    assert!(hover(src, 3, 2, &analysis, None).is_none());
}

// $obj method dispatch

#[test]
fn obj_method_hover_fires_for_known_instance() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Line 4 `$d bark` — cursor on `bark` (col 3).
    let h = hover(src, 4, 3, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Dog::bark"), "{}", h.value);
}

#[test]
fn obj_method_hover_fires_for_a_literal_foreach_installed_method() {
    // The member's name comes from the loop's own literal
    // list, not a written `method NAME` word, but a call site must
    // still resolve to a hover — go-to-definition and hover share the
    // same member table this walk populates.
    let src = concat!(
        "oo::class create Widget {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "}\n",
        "set d [Widget new]\n",
        "$d alpha\n",
    );
    let analysis = analyse(src);
    // Line 6 `$d alpha` — cursor on `alpha` (col 3).
    let h = hover(src, 6, 3, &analysis, None).expect("hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(h.value.contains("::Widget::alpha"), "{}", h.value);
}

#[test]
fn obj_method_hover_none_for_unknown_instance() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
    let analysis = analyse(src);
    // `x` has no recorded class — no hover.
    assert!(hover(src, 3, 3, &analysis, None).is_none());
}

/// A Tk widget's instance command (a registry-modelled, self-referential
/// `object_class`, not a user-defined one) needs the
/// registry passed to resolve hover text at all.
#[test]
fn obj_method_hover_fires_for_bareword_widget() {
    let reg = tcl_registry::CommandRegistry::build_default();
    let src = "ttk::treeview .t\n.t instate {selected} {}\n";
    let analysis = analyse(src);
    // Line 1 `.t instate …` — cursor on `instate` (col 3).
    let h = hover(src, 1, 3, &analysis, Some(&reg)).expect("hover");
    assert!(h.value.contains("ttk::treeview instate"), "{}", h.value);
}

#[test]
fn widget_method_hover_follows_the_resolved_tk_lifecycle() {
    // ttk::treeview's `current` instance method is introduced by Tk 9.1.
    // The registry lookup is the only method-specific input here, so this
    // proves an older profile cannot render an impossible method hover.
    let src = "ttk::treeview .tree\n.tree current\n";
    for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
        let analysis = analyse_for(src, dialect);
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let hover = hover_with_profile(src, 1, 6, &analysis, Some(registry), profile);
        assert_eq!(
            hover.is_some(),
            expected,
            "ttk::treeview current hover under {dialect}: {hover:?}"
        );
    }

    // A document's unconditional package require can raise the profile
    // floor. The request-time hover path passes that explicit package
    // floor to the same registry lookup instead of baking Tk knowledge
    // into the provider.
    let raised = "package require Tk 9.1\nttk::treeview .tree\n.tree current\n";
    let raised_analysis = analyse_for(raised, "tcl9.0");
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let raised_hover = hover_with_profile(
        raised,
        2,
        6,
        &raised_analysis,
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
        tcl90,
    );
    assert!(
        raised_hover.is_some(),
        "an explicit Tk 9.1 floor must expose current: {raised_hover:?}"
    );
}

/// Without a registry passed, the same widget dispatch has nowhere to
/// look up the method — `obj_method_hover_text`'s registry fallback is
/// skipped, not a panic.
#[test]
fn obj_method_hover_none_for_widget_without_registry() {
    let src = "ttk::treeview .t\n.t instate {selected} {}\n";
    let analysis = analyse(src);
    assert!(hover(src, 1, 3, &analysis, None).is_none());
}

#[test]
fn obj_method_hover_fires_for_var_captured_widget() {
    let reg = tcl_registry::CommandRegistry::build_default();
    let src = "set lb [listbox .l]\n$lb curselection\n";
    let analysis = analyse(src);
    // Line 1 `$lb curselection` — cursor on `curselection` (col 4).
    let h = hover(src, 1, 4, &analysis, Some(&reg)).expect("hover");
    assert!(h.value.contains("listbox curselection"), "{}", h.value);
}

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: hover on either the declaration or a
// call site must show the real `args`-only signature, never
// `optlist`'s own literal text.

#[test]
fn opt_proc_declaration_and_call_site_both_hover_the_real_args_only_signature() {
    let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
    let decl = hover(src, 0, 15, &analysis, None).expect("hover on decl");
    assert!(
        decl.value.contains("greet") && decl.value.contains("args"),
        "{}",
        decl.value
    );
    // Line 1 — cursor on the `greet foo` call site (col 0).
    let call = hover(src, 1, 0, &analysis, None).expect("hover on call site");
    assert_eq!(decl.value, call.value, "decl and call site must agree");
}

// Hover shares `definition()`'s command-resolution seam, so it shares
// both of its gates — the command-position gate and the
// resolved-indirect-head preference.

#[test]
fn fp_argument_word_sharing_a_procs_name_does_not_hover_that_proc() {
    // FP guard — `dump` is `anotherproc`'s first argument, never a
    // command, so rendering `proc ::dump`'s signature there is a wrong
    // answer.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 24, &analysis, None);
    assert!(
        h.is_none_or(|hv| !hv.value.contains("proc ::dump")),
        "an argument word must not hover as a command"
    );
}

#[test]
fn tp_the_command_head_beside_that_argument_still_hovers() {
    // TP — the gate must not cost the real head its hover.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 12, &analysis, None).expect("hover on the call head");
    assert!(h.value.contains("proc ::anotherproc"), "{}", h.value);
}

#[test]
fn consumed_procedure_name_hover_uses_its_lookup_receipt() {
    let source = "proc target {} {return OK}\ninfo args target\nrename target moved\nmoved\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    for (written, prefix) in [
        ("info args target", "info args "),
        ("rename target", "rename "),
    ] {
        let offset = u32::try_from(source.find(written).unwrap() + prefix.len()).unwrap();
        assert!(!crate::definition::offset_is_command_head(
            &analysis, offset
        ));
        let result = proc_hover_at(
            &analysis,
            source,
            offset,
            "target",
            crate::definition::CallResolution::document_only(),
        )
        .expect("hover on the consumed source procedure");
        assert!(result.value.contains("proc ::target"), "{}", result.value);
    }
}

#[test]
fn tp_resolved_indirect_head_hovers_the_command_it_reaches() {
    // TP — hover's half: the `${ns}::setdef` head carries
    // no written command name, so only the analyser's own resolution
    // can answer, and it must not lose to a same-named decoy.
    let src = "namespace eval ::tc { proc setdef {a} { return $a } }\nnamespace eval ::other { proc setdef {b c} { return $b } }\nset ns ::tc\n${ns}::setdef\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 7, &analysis, None).expect("hover on the indirect head");
    assert!(
        h.value.contains("::tc::setdef"),
        "must describe ::tc::setdef, not the ::other decoy: {}",
        h.value
    );
}

#[test]
fn command_profile_metadata_keeps_foreign_surface_and_own_requirements() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let descriptor = |surface| tcl_registry::CommandSpec {
        name: "ACCESS::log",
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let mut text = String::new();
    super::append_command_event_metadata(
        &mut text,
        &descriptor(Some(tcl_dialect::model::SpecSurface::IRULES)),
        super::EventRelease::Unconfigured,
    );
    assert_eq!(text, "\n\n**Requires**: ACCESS profile");
    for surface in [None, Some(tcl_dialect::model::SpecSurface::TCL86)] {
        let mut text = String::new();
        super::append_command_event_metadata(
            &mut text,
            &descriptor(surface),
            super::EventRelease::Unconfigured,
        );
        assert_eq!(text, "");
    }
    let own = tcl_registry::CommandSpec {
        name: "DIAMETER::fixture",
        surface: Some(tcl_dialect::model::SpecSurface::IRULES),
        event_requires: Some(tcl_registry::events::EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let mut text = String::new();
    super::append_command_event_metadata(&mut text, &own, super::EventRelease::Unconfigured);
    assert!(text.contains("**Requires**: profile HTTP"), "{text}");
    assert!(!text.contains("profile DIAMETER"), "{text}");
}
