// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original which-hook selection and retained command-name operands.

use super::*;
use crate::native_compiler_words::NativeCompilerWords;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::native_string::NativeStringProtocol;

const SPEC: NativeCompilationSpec = NativeCompilationSpec {
    grammar: NativeCompilationGrammar::NamespaceWhich,
    operation: SemanticOperationId::Invoke,
    body: NativeBodyCompilation::Direct,
};

fn context() -> NativeCompilationContext {
    NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        loop_depth: 0,
        catch_depth: Some(0),
    }
}

#[test]
fn original_which_shapes_match_retained_compiler_choice_boundaries() {
    let input =
        include_str!("../../../../experiments/name-handling/namespace-which-compiler/input.tcl");
    let (_, cases) = input.split_once("foreach {label source} {\n").unwrap();
    let (cases, _) = cases.split_once("\n} {").unwrap();
    let cases = tcl_syntax::list::split_list(cases).unwrap();
    assert_eq!(cases.len(), 32);
    for version in TclVersion::ALL {
        let classification = match version {
            TclVersion::V8_4 => include_str!(
                "../../../../experiments/name-handling/namespace-which-compiler/8.4.20/classification.txt"
            ),
            TclVersion::V8_5 => include_str!(
                "../../../../experiments/name-handling/namespace-which-compiler/8.5.19/classification.txt"
            ),
            TclVersion::V8_6 => include_str!(
                "../../../../experiments/name-handling/namespace-which-compiler/8.6.18/classification.txt"
            ),
            TclVersion::V9_0 => include_str!(
                "../../../../experiments/name-handling/namespace-which-compiler/9.0.4/classification.txt"
            ),
            TclVersion::V9_1 => include_str!(
                "../../../../experiments/name-handling/namespace-which-compiler/9.1.0/classification.txt"
            ),
        };
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            SPEC.compiler_hook_presence(dialect),
            Some(version >= TclVersion::V8_6)
        );
        assert_eq!(
            SPEC.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_6
        );
        let mut compared = 0;
        for (case, row) in cases.chunks_exact(2).zip(classification.lines().skip(1)) {
            let columns: Vec<_> = row.split_whitespace().collect();
            assert_eq!(columns[0], case[0]);
            let source = case[1].as_bytes();
            let parsed = tcl_lexer::native_script_words_in(
                SourceImage::native(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &parsed.commands[0].words,
                NativeStringProtocol::C(version),
            )
            .unwrap();
            let expected = if version < TclVersion::V8_6 {
                NativeCompilationSelection::Generic
            } else if columns[2] == "1" {
                NativeCompilationSelection::Inline {
                    operation: SPEC.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                }
            } else {
                assert_eq!(columns[4], "1");
                NativeCompilationSelection::Generic
            };
            assert_eq!(
                SPEC.select_native_words(&words, 2, Some(dialect), context()),
                expected,
                "{version:?}: {}",
                case[0]
            );
            let recipe = crate::native_introspection_compilation::compile_native_introspection(
                &words,
                2,
                crate::native_introspection_compilation::NativeIntrospectionKind::NamespaceWhich,
                version,
            );
            assert_eq!(
                recipe.is_some(),
                version >= TclVersion::V8_6 && columns[2] == "1"
            );
            if let Some(recipe) = recipe {
                assert_eq!(
                    recipe.operands,
                    vec![
                        crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                            words.original_words().len() - 1
                        ),
                    ]
                );
            }
            compared += 1;
        }
        assert_eq!(compared, 16);
        eprintln!(
            "{version:?}: 16 syntactic selector controls; C84 disassembler is unavailable, so C84 no-hook controls do not claim opcode observations; accepted resolveCmd cases select one original name operand"
        );
    }
}

#[test]
fn literal_variable_option_declines_before_opaque_name_projection() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        for option in [b"-variable".as_slice(), b"-command".as_slice()] {
            let mut source = b"namespace which ".to_vec();
            source.extend_from_slice(option);
            source.extend_from_slice(b" \xff");
            let parsed = tcl_lexer::native_script_words_in(
                SourceImage::native(source.as_slice()),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &parsed.commands[0].words,
                NativeStringProtocol::C(version),
            )
            .unwrap();
            assert!(words.checked_words().is_none());
            assert_eq!(
                SPEC.select_native_words(&words, 2, Some(dialect), context()),
                if option == b"-variable" {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Unknown
                }
            );
            let foreign = NativeCompilerWords::capture(
                &parsed.commands[0].words,
                NativeStringProtocol::Jim084,
            )
            .unwrap();
            assert_eq!(
                SPEC.select_native_words(&foreign, 2, Some(dialect), context()),
                NativeCompilationSelection::Unknown
            );
        }
    }
}

#[test]
fn checked_member_facts_keep_original_which_operand_offset() {
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for option in ["-variable", "-command"] {
            let args = ["which", option, "g"];
            let resolved = registry
                .resolve_invocation("namespace", &args, registry.own_surface_query())
                .unwrap();
            assert_eq!(
                SPEC.select_for_facts(
                    InvocationWords::literals("namespace", &args),
                    &[NativeCompilationWordShape::Literal; 3],
                    &resolved.facts(),
                    Some(dialect),
                    context(),
                ),
                if version < TclVersion::V8_6 || option == "-variable" {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Inline {
                        operation: SPEC.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                }
            );
        }
    }
}
