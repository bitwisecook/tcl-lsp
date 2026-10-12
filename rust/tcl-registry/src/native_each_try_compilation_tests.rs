// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native compiler windows for iterator and exception recipes.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{NativeControlOutcome, NativeControlPreparationStep};
use crate::native_each_compilation::{NativeEachCollection, compile_native_each};
use crate::native_try_compilation::compile_native_try;
use tcl_dialect::TclVersion;
use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
use tcl_syntax::native_string::NativeStringProtocol;

mod inputs {
    include!("../tests/data/native_each_try_compilation/cases.rs");
}
fn fixtures() -> [(TclVersion, &'static str); 5] {
    [
        (
            TclVersion::V8_4,
            include_str!("../tests/data/native_each_try_compilation/8.4.20.tsv"),
        ),
        (
            TclVersion::V8_5,
            include_str!("../tests/data/native_each_try_compilation/8.5.19.tsv"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../tests/data/native_each_try_compilation/8.6.18.tsv"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../tests/data/native_each_try_compilation/9.0.4.tsv"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../tests/data/native_each_try_compilation/9.1.0.tsv"),
        ),
    ]
}
fn context() -> NativeCompilationContext {
    NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    }
}
fn hex(value: &[u8]) -> String {
    use std::fmt::Write;
    value.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").unwrap();
        text
    })
}
fn declarations(steps: &[NativeControlPreparationStep]) -> Vec<String> {
    let mut named = std::collections::HashSet::new();
    steps
        .iter()
        .filter_map(|step| match step {
            NativeControlPreparationStep::DeclareLocal(name) if named.insert(name.clone()) => {
                Some(hex(name))
            }
            NativeControlPreparationStep::DeclareAnonymousLocal => Some(String::new()),
            _ => None,
        })
        .collect()
}

#[test]
fn original_each_and_try_recipes_match_190_native_selection_and_local_windows() {
    let mut matched = 0;
    for (version, fixture) in fixtures() {
        let dialect = crate::InvocationDialect::for_version(version);
        for row in fixture.lines().skip(1) {
            let columns = row.split('\t').collect::<Vec<_>>();
            let index = columns[0].parse::<usize>().unwrap();
            let (label, source) = inputs::CASES[index];
            // These two source vectors prove child/outer parse failure rather
            // than whether the containing iterator compiler accepts its words.
            if matches!(index, 13 | 14) {
                continue;
            }
            let image = SourceImage::native(source);
            let parsed = native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let head = if index < 17 {
                b"foreach".as_slice()
            } else if index < 19 {
                b"lmap".as_slice()
            } else {
                b"try".as_slice()
            };
            let command = parsed
                .commands
                .iter()
                .find(|command| {
                    NativeCompilerWords::capture(&command.words, NativeStringProtocol::C(version))
                        .unwrap()
                        .literal(0)
                        == Some(head)
                })
                .unwrap();
            let words =
                NativeCompilerWords::capture(&command.words, NativeStringProtocol::C(version))
                    .unwrap();
            let (inline, reservations) = if index < 19 {
                let selected = compile_native_each(
                    &words,
                    1,
                    version,
                    context(),
                    if index < 17 {
                        NativeEachCollection::Foreach
                    } else {
                        NativeEachCollection::Lmap
                    },
                )
                .unwrap();
                (
                    matches!(selected.outcome, NativeControlOutcome::Inline(_)),
                    declarations(&selected.preparations),
                )
            } else {
                let selected = compile_native_try(&words, 1, version, context()).unwrap();
                (
                    matches!(selected.outcome, NativeControlOutcome::Inline(_)),
                    declarations(&selected.preparations),
                )
            };
            let native_generic = columns[6].split(',').any(|literal| literal == hex(head));
            assert_eq!(
                inline, !native_generic,
                "{version:?}/{label}: original native literal/dispatch selection"
            );
            let native_locals = columns[3].split(',').collect::<Vec<_>>();
            if !reservations.is_empty() {
                // Source commands before the iterator/try already own their
                // slots. Recipe declarations retain their relative native order.
                let prior = usize::from(matches!(index, 8 | 9 | 30 | 31 | 32 | 33));
                assert_eq!(
                    reservations,
                    native_locals[prior..prior + reservations.len()],
                    "{version:?}/{label}: original named/anonymous preparation order"
                );
            }
            matched += 1;
        }
    }
    assert_eq!(matched, 190);
}

#[test]
fn try_dynamic_body_is_protected_but_iterator_values_precede_the_loop_range() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = crate::InvocationDialect::for_version(version);
        let image = SourceImage::native(b"try $missing on error {m o} {set m}".as_slice());
        let parsed = native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let selected = compile_native_try(&words, 1, version, context()).unwrap();
        let NativeControlOutcome::Inline(recipe) = selected.outcome else {
            panic!("native try inline")
        };
        assert!(recipe.body.script.is_none());
        assert_eq!(
            recipe.body.operand,
            crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(1)
        );
        assert_eq!(recipe.handlers[0].condition.code(), 1);
    }
}
