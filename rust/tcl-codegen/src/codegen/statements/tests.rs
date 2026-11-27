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
use crate::codegen::CodegenCtx;
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::CommandTokens;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;

#[test]
fn folded_script_without_original_words_requires_a_provider() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let body = tcl_compiler::ir::Script::from_statements(vec![Statement::AssignConst {
        span: Span::new(0, 1),
        name: "x".to_owned(),
        name_braced: false,
        value: "CHILD".to_owned(),
        value_span: None,
    }]);
    let statement = Statement::Block {
        span: Span::new(0, 1),
        body,
        namespace: "::".to_owned(),
        tokens: None,
        error_context: Some(tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation),
    };
    ctx.emit_stmt(&statement, &mut false);
    let function = ctx.into_function_asm("unknown-body".to_owned());
    assert_eq!(
        function.native_compilation_preflight,
        tcl_runtime_api::NativeCompilationPreflight::ProviderRequired
    );
    assert_eq!(function.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(
        !function
            .literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"CHILD")
    );
}

fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
    ctx.instructions.iter().map(|i| i.op).collect()
}

fn sp() -> Span {
    Span::new(0, 0)
}

#[test]
fn native_word_emission_keeps_document_and_counted_value_units_distinct() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let registry = CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let suffix: &[u8] = match version {
            tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5 => {
                b"\xc3\xb0\xc2\x9f\xc2\x98\xc2\x80"
            }
            tcl_dialect::TclVersion::V8_6 => b"\xed\xa0\xbd\xed\xb8\x80",
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => b"\xf0\x9f\x98\x80",
        };
        let mut document_value = b"A\xc0\x80".to_vec();
        document_value.extend_from_slice(suffix);
        for source in ["{A\0😀}", "\"A\0😀\""] {
            for (image, expected) in [
                (
                    tcl_lexer::SourceImage::document(source),
                    document_value.as_slice(),
                ),
                (
                    tcl_lexer::SourceImage::native(source.as_bytes()),
                    b"A\0\xf0\x9f\x98\x80".as_slice(),
                ),
            ] {
                let words = tcl_lexer::native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    config,
                )
                .unwrap();
                let mut context = CodegenCtx::new(false, &[], &registry);
                context.invocation_dialect = Some(dialect);
                context.source_string_protocol = dialect.native_source_string_protocol();
                context.emit_original_native_word(&words.commands[0].words[0]);
                assert!(!context.native_dependency_refusal, "{version:?}/{source}");
                assert!(
                    context
                        .literals
                        .entries()
                        .iter()
                        .any(|literal| literal.bytes() == expected),
                    "{version:?}/{source}"
                );
            }
        }
    }
}

#[test]
fn native_source_word_without_selected_string_protocol_requires_provider() {
    let registry = CommandRegistry::build_default();
    for source in ["{A\0}", "\"A\0\""] {
        let image = tcl_lexer::SourceImage::document(source);
        let words = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        let mut context = CodegenCtx::new(false, &[], &registry);
        context.source_string_protocol = None;
        context.emit_original_native_word(&words.commands[0].words[0]);
        assert!(context.native_dependency_refusal);
        assert!(
            !context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes().starts_with(b"A"))
        );
    }
}

#[test]
fn opaque_native_statement_emits_original_literal_bytes_and_generic_dispatch() {
    let registry = CommandRegistry::build_default();
    let statement = tcl_compiler::ir::native_call_for_test(b"opaque \xff");
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let mut generic = false;
    ctx.emit_stmt(&statement, &mut generic);
    assert!(generic);
    assert_eq!(
        opcodes(&ctx),
        [Op::NOP, Op::PUSH1, Op::PUSH1, Op::INVOKE_STK1, Op::POP]
    );
    assert_eq!(
        ctx.instructions[0].source_command_boundary,
        tcl_bytecode::SourceCommandBoundary::Start
    );
    assert_eq!(ctx.literals.entries()[0].bytes(), b"opaque");
    assert_eq!(ctx.literals.entries()[1].bytes(), b"\xff");
    assert!(
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::PUSH1)
            .all(|instruction| instruction.push_verbatim)
    );
}

#[test]
fn native_array_indices_emit_iteratively_from_the_full_original_arena() {
    let registry = CommandRegistry::build_default();
    let source = include_bytes!("../../../../tcl-compiler/tests/data/native_deep_array_word.tcl");
    let statement = tcl_compiler::ir::native_call_for_test(source);
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let mut generic = false;
    ctx.emit_stmt(&statement, &mut generic);
    assert!(generic);
    assert_eq!(
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::LOAD_ARRAY_STK)
            .count(),
        2000,
    );
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"x\xff")
    );
    assert_eq!(
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::INVOKE_STK1)
            .count(),
        1,
    );
}

#[test]
fn native_registered_return_keeps_original_result_and_exception_range_exit() {
    use tcl_registry::native_compilation::NativeCompilationFrame;
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry: crate::test_support::CapturedNativeEntry =
            crate::test_support::captured_native_entry(profile);
        let image = tcl_lexer::SourceImage::native(&b"return {A\xff\0tail}"[..]);
        let script = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        for (procedure, catch_depth, terminal) in [
            (true, 0, Op::DONE),
            (true, 1, Op::RETURN_IMM),
            (false, 0, Op::RETURN_IMM),
        ] {
            if name == "tcl8.4" && (!procedure || catch_depth != 0) {
                continue; // Native C8.4 deliberately invokes these returns.
            }
            let mut context = CodegenCtx::new(procedure, &[], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect =
                Some(tcl_native_model::InvocationDialect::of_profile(profile));
            context.source_string_protocol = entry.source_string_protocol;
            context.native_compilation.frame = if procedure {
                NativeCompilationFrame::ProcedureCode
            } else {
                NativeCompilationFrame::ScriptCode
            };
            context.native_compilation.catch_depth = Some(catch_depth);
            context.emit_native_words(&script.commands[0].words);
            assert!(
                !context.native_dependency_refusal,
                "{name}/{procedure}/{catch_depth}"
            );
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == terminal),
                "{name}/{procedure}/{catch_depth}"
            );
            assert!(
                !context
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
            );
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.native_compiler_selection.is_some())
            );
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == b"A\xff\0tail")
            );
            if terminal == Op::RETURN_IMM {
                let exit = context
                    .instructions
                    .iter()
                    .position(|instruction| instruction.op == terminal)
                    .unwrap();
                assert_eq!(
                    context.instructions[exit].operands,
                    vec![Operand::Imm(0), Operand::Imm(1)]
                );
                assert!(matches!(
                    context.instructions[exit - 1].op,
                    Op::PUSH1 | Op::PUSH4
                ));
                let Operand::Imm(slot) = context.instructions[exit - 1].operands[0] else {
                    panic!("native private options PUSH")
                };
                assert!(
                    matches!(context.literals.entries()[usize::try_from(slot).unwrap()].allocation(),tcl_bytecode::NativeLiteralAllocation::PrivateReturnOptions(recipe) if recipe.words.is_empty() && (recipe.code,recipe.level,recipe.size)==(0,1,0))
                );
            }
        }
    }
}

#[test]
fn native_registered_return_options_keep_private_literals_and_ordered_stack_operands() {
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry: crate::test_support::CapturedNativeEntry =
            crate::test_support::captured_native_entry(profile);
        for source in [
            b"return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY"
                .as_slice(),
            b"return -options $options $result",
            b"return -code $code $result",
        ] {
            let image = tcl_lexer::SourceImage::native(source);
            let script = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &[], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect =
                Some(tcl_native_model::InvocationDialect::of_profile(profile));
            context.source_string_protocol = entry.source_string_protocol;
            context.native_compilation.frame =
                tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            context.native_compilation.catch_depth = Some(0);
            context.emit_native_words(&script.commands[0].words);
            assert!(!context.native_dependency_refusal, "{name}/{source:?}");
            if source.starts_with(b"return -level") {
                let instruction = context
                    .instructions
                    .iter()
                    .position(|instruction| instruction.op == Op::RETURN_IMM)
                    .unwrap();
                assert_eq!(
                    context.instructions[instruction].operands,
                    vec![Operand::Imm(1), Operand::Imm(0)]
                );
                let Operand::Imm(slot) = context.instructions[instruction - 1].operands[0] else {
                    panic!("private options PUSH")
                };
                assert!(
                    matches!(context.literals.entries()[usize::try_from(slot).unwrap()].allocation(),tcl_bytecode::NativeLiteralAllocation::PrivateReturnOptions(recipe) if (recipe.code,recipe.level,recipe.size)==(1,0,2))
                );
                assert!(
                    context.literals.entries()[usize::try_from(slot).unwrap()]
                        .byte_payload()
                        .is_none()
                );
            } else if name == "tcl8.5" && source.starts_with(b"return -code") {
                assert!(context.instructions.iter().any(|instruction| matches!(
                    instruction.op,
                    Op::INVOKE_STK1 | Op::INVOKE_STK4
                )));
            } else {
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.op == Op::RETURN_STK)
                );
                assert!(!context.instructions.iter().any(|instruction| matches!(
                    instruction.op,
                    Op::INVOKE_STK1 | Op::INVOKE_STK4
                )));
                if source.starts_with(b"return -code") {
                    assert!(
                        context
                            .instructions
                            .iter()
                            .any(|instruction| instruction.op == Op::LIST
                                && instruction.operands == vec![Operand::Imm(2)])
                    );
                }
            }
        }
    }
}

#[test]
fn native_namespace_emission_matches_all_100_instruction_and_local_windows() {
    let bodies: [&[u8]; 20] = [
        b"variable v",
        b"variable {}",
        b"variable :::",
        b"variable {a)}",
        b"variable v $val w [set x 3]",
        b"variable v 1 {a)} 2",
        b"variable ${ns}::v",
        b"variable ${ns}v",
        b"variable ${ns}::[set x 1]",
        b"variable ${ns}::v 1 ${ns}v 2",
        b"variable {*}{v 1}",
        b"variable",
        b"global v",
        b"global {}",
        b"global :::",
        b"global {a)}",
        b"global ${ns}::v",
        b"global ${ns}v",
        b"global v {a)}",
        b"global {*}{v w}",
    ];
    for (name, expected) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv"),
        ),
    ] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry: crate::test_support::CapturedNativeEntry =
            crate::test_support::captured_native_entry(profile);
        for (case, row) in expected.lines().skip(1).enumerate() {
            compare_native_namespace_case(
                name,
                profile,
                &registry,
                &entry,
                case,
                bodies[case],
                row,
            );
        }
    }
}

fn compare_native_namespace_case(
    name: &str,
    profile: &tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    entry: &tcl_runtime_api::NativeCompilationEntry,
    case: usize,
    source: &[u8],
    row: &str,
) {
    let columns = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
    let image = tcl_lexer::SourceImage::native(source);
    let script = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    if columns[2] == "-1" {
        assert!(
            script.fatal_tail.is_some(),
            "{name}/{case}: authentic unsupported expansion syntax"
        );
        assert_eq!(
            script.commands,
            [] as [tcl_lexer::NativeScriptCommandWords; 0]
        );
        return;
    }
    let mut context = CodegenCtx::new(true, &["ns", "val"], registry);
    context.native_entry = Some(entry);
    context.invocation_dialect = Some(tcl_native_model::InvocationDialect::of_profile(profile));
    context.source_string_protocol = entry.source_string_protocol;
    context.compiled_variable_protocol = entry.compiled_variable_protocol;
    context.emit_native_words(&script.commands[0].words);
    assert!(!context.native_dependency_refusal, "{name}/{case}");
    for (column, op) in [(2, Op::VARIABLE), (3, Op::NSUPVAR)] {
        assert_eq!(
            context.instructions.iter().filter(|i| i.op == op).count(),
            columns[column].parse::<usize>().unwrap(),
            "{name}/{case}/{op:?}"
        );
    }
    let locals = context
        .lvt
        .native_slot_names()
        .iter()
        .map(|name| {
            use std::fmt::Write;
            name.as_ref()
                .unwrap()
                .as_bytes()
                .iter()
                .fold(String::new(), |mut hex, byte| {
                    write!(hex, "{byte:02x}").unwrap();
                    hex
                })
        })
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(locals, columns[4], "{name}/{case}");
    if case == 15 && name != "tcl8.4" {
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"::"),
            "{name}: failed first global tail retains its implicit literal"
        );
    }
    if case == 5 && name != "tcl8.4" {
        assert_native_namespace_rollback(&context, entry, name, case);
    }
}

fn assert_native_namespace_rollback(
    context: &CodegenCtx<'_>,
    entry: &tcl_runtime_api::NativeCompilationEntry,
    name: &str,
    case: usize,
) {
    let selections = context
        .instructions
        .iter()
        .filter_map(|instruction| {
            instruction
                .native_compiler_selection
                .as_ref()
                .map(|selection| (instruction, selection))
        })
        .collect::<Vec<_>>();
    let [(instruction, selection)] = selections.as_slice() else {
        panic!("{name}/{case}: original compiler selection before rollback")
    };
    assert_eq!(instruction.op, Op::NOP, "{name}/{case}");
    let tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(required) =
        &selection.prerequisite
    else {
        panic!("{name}/{case}: original variable compiler registration")
    };
    assert_eq!(
        required.guard,
        tcl_runtime_api::CommandBindingGuard::BeforeArguments,
        "{name}/{case}"
    );
    assert_eq!(required.invocation_word.as_bytes(), b"variable");
    let original = entry
        .lookup_command_bytes(
            required.lookup_namespace_token,
            required.invocation_word.as_bytes(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(original.compiler.as_ref(), Some(&required.compiler));
    assert_eq!(original.token, required.token);
    assert_eq!(original.namespace_token, required.namespace_token);
    assert!(
        context
            .instructions
            .iter()
            .any(|i| matches!(i.op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
    );
    assert!(
        context
            .literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"1")
    );
}

#[test]
fn original_byte_commands_retain_exact_boundaries_and_restore_enclosing_source() {
    let registry = CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry: crate::test_support::CapturedNativeEntry =
        crate::test_support::captured_native_entry(profile);
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let image =
        tcl_lexer::SourceImage::native(b"opaque \"first\xff\";\nother \"second\"".as_slice());
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).unwrap()),
        config,
    )
    .unwrap();
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.native_entry = Some(&entry);
    context.set_source("enclosing command");
    context.set_command_source_span(Span::new(0, 17));
    for command in &plan.commands {
        context.emit_native_words(&command.words);
    }
    let boundaries = context
        .instructions
        .iter()
        .filter(|instruction| instruction.source_command_boundary.is_start())
        .collect::<Vec<_>>();
    assert_eq!(boundaries.len(), 2);
    for (instruction, command) in boundaries.iter().zip(&plan.commands) {
        assert_eq!(
            instruction.source_cmd_text.bytes(),
            &image.bytes()[command.span.as_range()]
        );
        assert_eq!(instruction.source_span, Some(command.span));
    }
    assert_eq!(boundaries[1].source_line, 2);
    let trailing = context.emit(Op::NOP, vec![]);
    assert_eq!(
        context.instructions[trailing].source_cmd_text.bytes(),
        b"enclosing command"
    );
    assert_eq!(context.source_image().bytes(), b"enclosing command");
}

#[test]
fn retained_quoted_components_execute_but_braced_and_rewritten_values_do_not() {
    let registry = CommandRegistry::build_default();
    let source = "list \"boom-$x\" {boom-$x}";
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let segment = tcl_source::segment::segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    let quoted = &tokens.words()[1];
    let mut active = CodegenCtx::new(true, &[], &registry);
    active.emit_word_from_source(&quoted.legacy_text(), false, Some(quoted));
    assert!(opcodes(&active).contains(&Op::LOAD_SCALAR1));
    assert!(opcodes(&active).contains(&Op::STR_CONCAT1));

    let braced = &tokens.words()[2];
    let mut literal = CodegenCtx::new(true, &[], &registry);
    literal.emit_word_from_source(&braced.legacy_text(), true, Some(braced));
    assert_eq!(opcodes(&literal), vec![Op::PUSH1]);
    assert!(literal.instructions[0].push_verbatim);

    let mut rewritten = CodegenCtx::new(true, &[], &registry);
    rewritten.emit_word_from_source("REWRITTEN", false, Some(quoted));
    assert_eq!(opcodes(&rewritten), vec![Op::PUSH1]);
}

#[test]
fn literal_script_value_retains_its_word_line_separately_from_the_command() {
    let source = "proc p \\\n{} \\\n{\nerror BOOM\n}";
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::default();
    let segment = tcl_source::segment::segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    let word = &tokens.words()[3];
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.set_source(source);
    ctx.set_command_source_span(segment.span);
    ctx.emit_word_from_source(&word.legacy_text(), true, Some(word));
    assert_eq!(ctx.instructions.len(), 1);
    assert_eq!(ctx.instructions[0].source_line, 1);
    assert_eq!(ctx.instructions[0].source_value_line, Some(3));

    let opaque = WordExpr::BracedLiteral {
        text: word.legacy_text(),
        source: tcl_source::words::SourceSite::opaque(word.source().span),
    };
    ctx.emit_word_from_source(&opaque.legacy_text(), true, Some(&opaque));
    assert_eq!(ctx.instructions[1].source_value_line, None);
    ctx.set_source("");
    ctx.emit_word_from_source(&word.legacy_text(), true, Some(word));
    assert_eq!(ctx.instructions[2].source_value_line, None);
}

#[test]
fn nested_value_emission_retains_only_the_owning_source_dispatch() {
    let registry = CommandRegistry::build_default();
    let source = "set result [list VALUE]";
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings =
        tcl_compiler::command_binding::SourceCommandBindings::analyse(source, config, &registry);
    let segment = tcl_source::segment::segment_commands(source).remove(0);
    let mut parent = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut parent);
    let word = parent.words()[2].clone();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.with_invocation_tokens(Some(&parent), |ctx| {
        let nested = ctx.nested_command_tokens(Some(&word)).unwrap();
        assert!(
            nested
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_some()
        );
    });

    let retained = std::mem::take(&mut parent.nested_bindings);
    ctx.with_invocation_tokens(Some(&parent), |ctx| {
        let nested = ctx.nested_command_tokens(Some(&word)).unwrap();
        assert!(nested.has_unproved_source_binding());
    });
    let opaque = WordExpr::CommandSubstitution {
        spelling: "[list VALUE]".into(),
        source: tcl_source::words::SourceSite::opaque(word.source().span),
    };
    parent.nested_bindings = retained;
    ctx.with_invocation_tokens(Some(&parent), |ctx| {
        let nested = ctx.nested_command_tokens(Some(&opaque)).unwrap();
        assert!(nested.has_unproved_source_binding());
    });
}

#[test]
fn nested_value_tokens_keep_the_retained_lexer_policy_without_lookup_permission() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Lexical source control; this supplies no Native selection or body admission.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let source = "set result [list ${a{b}c}]";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    };
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let parent =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    let word = &parent.words()[2];
    let mut ctx = CodegenCtx::new(true, &[], registry);
    ctx.dialect = Some(profile);
    ctx.ingress_lexer_config = Some(config);
    let nested = ctx.nested_command_tokens(Some(word)).unwrap();
    assert_eq!(
        nested.words()[1].sole_variable_substitution().unwrap().0,
        "${a{b}c}"
    );
    assert_eq!(nested.words()[0].source().span.start(), 12);
    assert!(nested.source_binding.is_none());
    ctx.ingress_lexer_config = None;
    let standalone = ctx.nested_command_tokens(Some(word)).unwrap();
    assert!(standalone.words()[1].sole_variable_substitution().is_none());
    assert!(standalone.source_binding.is_none());
}

#[test]
fn emit_assign_const_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let stmt = Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "42".into(),
        name_braced: false,
        value_span: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(!ugi);
    assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::STORE_SCALAR1, Op::POP]);
}

#[test]
fn emit_assign_const_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "42".into(),
        name_braced: false,
        value_span: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert_eq!(
        opcodes(&ctx),
        vec![Op::PUSH1, Op::PUSH1, Op::STORE_STK, Op::POP]
    );
}

#[test]
fn emit_incr_stmt() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    let stmt = Statement::Incr {
        span: sp(),
        name: "x".into(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert_eq!(opcodes(&ctx), vec![Op::INCR_SCALAR1_IMM, Op::POP]);
}

#[test]
fn emit_call_generic() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::Call {
        span: sp(),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["hello".into()],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(ugi);
    assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
}

#[test]
fn emit_call_break_in_loop() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.break_target = Some("loop_end_0".into());
    let stmt = Statement::Call {
        span: sp(),
        command: "break".into(),
        canonical_command: None,
        args: vec![],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(!ugi);
    assert_eq!(opcodes(&ctx), vec![Op::JUMP4]);
}

#[test]
fn plain_dispatch_keeps_loop_control_spellings_as_runtime_calls() {
    let registry = CommandRegistry::build_default();
    for command in ["break", "continue"] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.plain_command_dispatch = true;
        ctx.break_target = Some("loop_end_0".into());
        ctx.continue_target = Some("loop_next_0".into());
        let stmt = Statement::Call {
            span: sp(),
            command: command.into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut used_generic_invoke = false;
        ctx.emit_stmt(&stmt, &mut used_generic_invoke);
        let ops = opcodes(&ctx);
        assert!(used_generic_invoke, "{command}: {ops:?}");
        assert!(ops.contains(&Op::INVOKE_STK1), "{command}: {ops:?}");
        assert!(!ops.contains(&Op::JUMP4), "{command}: {ops:?}");
    }
}

#[test]
fn plain_dispatch_keeps_internal_dict_loops_as_runtime_calls() {
    let registry = CommandRegistry::build_default();
    for command in ["::tcl::dict::for", "::tcl::dict::map"] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.plain_command_dispatch = true;
        let stmt = Statement::Call {
            span: sp(),
            command: command.into(),
            canonical_command: None,
            args: vec!["k v".into(), "a 1".into(), "set seen $k".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut used_generic_invoke = false;
        ctx.emit_stmt(&stmt, &mut used_generic_invoke);
        let ops = opcodes(&ctx);
        assert!(used_generic_invoke, "{command}: {ops:?}");
        assert!(ops.contains(&Op::INVOKE_STK1), "{command}: {ops:?}");
        assert!(
            !ops.contains(&Op::DICT_FIRST) && !ops.contains(&Op::DICT_NEXT),
            "{command} must not use builtin dict-loop opcodes: {ops:?}"
        );
    }
}

#[test]
fn emit_return_empty() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::Return {
        expr_base: None,
        tokens: None,
        span: sp(),
        value: None,
        value_word: None,
        expr: None,
        command_binding: None,
        braced: false,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(opcodes(&ctx).contains(&Op::RETURN_IMM));
}

#[test]
fn emit_stmt_with_start_cmd_numbering() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "1".into(),
        name_braced: false,
        value_span: None,
    };
    ctx.emit_stmt_with_start_cmd(&stmt, None, None);
    assert!(!opcodes(&ctx).contains(&Op::START_CMD));
    assert_eq!(ctx.cmd_index, 1);

    let stmt2 = Statement::AssignConst {
        span: sp(),
        name: "y".into(),
        value: "2".into(),
        name_braced: false,
        value_span: None,
    };
    ctx.emit_stmt_with_start_cmd(&stmt2, None, None);
    assert!(opcodes(&ctx).contains(&Op::START_CMD));
    assert_eq!(ctx.cmd_index, 2);
}

/// A `Statement::Call` carrying the `<empty_clause>` name.
fn empty_clause_call(marked: bool) -> Statement {
    Statement::Call {
        span: sp(),
        command: "<empty_clause>".into(),
        canonical_command: None,
        args: vec![],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: marked.then(|| {
            tcl_compiler::ir::CommandTokens::marker(tcl_compiler::ir::SyntheticMarker::EmptyClause)
        }),
        foreach_groups: None,
    }
}

#[test]
fn emit_empty_clause() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let mut ugi = false;
    ctx.emit_stmt(&empty_clause_call(true), &mut ugi);
    assert_eq!(opcodes(&ctx), vec![Op::NOP, Op::NOP, Op::NOP]);
}

/// The marker identity is the typed `SyntheticMarker`, never the command
/// spelling: `<empty_clause>` — like every other marker name — is a legal
/// Tcl command name, and tclsh 8.4.20 / 8.5.19 / 8.6.16 / 9.0.4 / 9.1 all
/// run `proc <empty_clause> {} { puts hit-ec }; <empty_clause>`. An
/// unmarked call of that name must therefore be *dispatched*, not folded
/// away.
#[test]
fn an_unmarked_call_named_like_a_marker_is_still_dispatched() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let mut ugi = false;
    ctx.emit_stmt(&empty_clause_call(false), &mut ugi);
    let ops = opcodes(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_STK1),
        "expected a real invoke, got {ops:?}"
    );
    assert!(!ops.contains(&Op::NOP), "must not fold to nops: {ops:?}");
}

#[test]
fn emit_barrier_with_command() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::Barrier {
        span: sp(),
        reason: "test".into(),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["script".into()],
        tokens: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(ugi);
    assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
}

#[test]
fn emit_barrier_without_command() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::Barrier {
        span: sp(),
        reason: "side-effect".into(),
        command: String::new(),
        canonical_command: None,
        args: vec![],
        tokens: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(!ugi);
    assert_eq!(opcodes(&ctx), vec![Op::NOP]);
}

#[test]
fn the_unseen_call_marker_is_not_dispatched() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::unseen_call_marker(sp());
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(!ugi);
    assert_eq!(opcodes(&ctx), [] as [tcl_bytecode::Op; 0]);
}

#[test]
fn the_unseen_call_marker_keeps_the_next_command_boundary() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let first = Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "1".into(),
        name_braced: false,
        value_span: None,
    };
    let marker = Statement::unseen_call_marker(sp());
    let second = Statement::AssignConst {
        span: sp(),
        name: "y".into(),
        value: "2".into(),
        name_braced: false,
        value_span: None,
    };

    ctx.emit_stmt_with_start_cmd(&first, None, None);
    ctx.emit_stmt_with_start_cmd(&marker, None, None);
    ctx.emit_stmt_with_start_cmd(&second, None, None);

    assert_eq!(
        opcodes(&ctx)
            .iter()
            .filter(|&&op| op == Op::START_CMD)
            .count(),
        1,
        "only the second real command receives a startCommand boundary",
    );
    assert_eq!(ctx.cmd_index, 2, "the marker is not a source command");
}

#[test]
fn the_unseen_call_marker_under_start_cmd_does_not_advance_command_index() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let first = Statement::AssignConst {
        span: sp(),
        name: "x".into(),
        value: "1".into(),
        name_braced: false,
        value_span: None,
    };
    let marker = Statement::unseen_call_marker(sp());

    ctx.emit_stmt_with_start_cmd(&first, None, None);
    ctx.emit_stmt_under_start_cmd(&marker);

    assert_eq!(ctx.cmd_index, 1, "the marker is not a source command");
}

#[test]
fn emit_assign_expr() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    let stmt = Statement::AssignExpr {
        span: sp(),
        name: "x".into(),
        name_braced: false,
        expr: ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        },
        command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
        expr_base: None,
        fallback_value: "[expr {42}]".into(),
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(opcodes(&ctx).contains(&Op::START_CMD));
    assert!(opcodes(&ctx).contains(&Op::STORE_SCALAR1));
}

#[test]
fn assign_expr_with_whole_unit_mutation_retains_the_runtime_binding() {
    let registry = CommandRegistry::build_default();
    let mutations = tcl_compiler::command_binding::ModuleCommandMutations::distrust_all();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.command_bindings = Some(&mutations);
    let stmt = Statement::AssignExpr {
        span: sp(),
        name: "x".into(),
        name_braced: false,
        expr: ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        },
        command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
        expr_base: None,
        fallback_value: "[expr {42}]".into(),
    };

    let mut used_generic_invoke = false;
    ctx.emit_stmt(&stmt, &mut used_generic_invoke);
    let ops = opcodes(&ctx);
    assert!(
        !ops.contains(&Op::INVOKE_STK1),
        "the entered expression may stay specialised until its live boundary: {ops:?}"
    );
    assert!(
        ctx.command_binding_requirements
            .contains(&tcl_runtime_api::CommandBindingIdentity::new(
                "expr", "expr"
            )),
        "the fused expression must retain the binding that runtime revalidation protects"
    );
    assert!(ops.contains(&Op::STORE_SCALAR1));
}

#[test]
fn syntax_owned_assign_expr_ignores_command_mutation() {
    let registry = CommandRegistry::build_default();
    let mutations = tcl_compiler::command_binding::ModuleCommandMutations::distrust_all();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.command_bindings = Some(&mutations);
    let stmt = Statement::AssignExpr {
        span: sp(),
        name: "x".into(),
        name_braced: false,
        expr: ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        },
        command_binding: None,
        expr_base: None,
        fallback_value: "$(42)".into(),
    };

    let mut used_generic_invoke = false;
    ctx.emit_stmt(&stmt, &mut used_generic_invoke);
    let ops = opcodes(&ctx);
    assert!(
        !ops.contains(&Op::INVOKE_STK1),
        "syntax-owned Jim expression must remain intrinsic: {ops:?}"
    );
    assert!(ctx.command_binding_requirements.is_empty());
    assert!(ops.contains(&Op::STORE_SCALAR1));
}

#[test]
fn emit_expr_eval() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let stmt = Statement::ExprEval {
        span: sp(),
        command_binding: tcl_runtime_api::CommandBindingIdentity::new("expr", "expr"),
        expr: ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        expr_base: None,
    };
    let mut ugi = false;
    ctx.emit_stmt(&stmt, &mut ugi);
    assert!(opcodes(&ctx).contains(&Op::POP));
    assert_eq!(
        ctx.command_binding_requirements,
        [tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")]
            .into_iter()
            .collect()
    );
}

#[test]
fn original_expanded_calls_keep_complete_words_and_lexical_nul_values() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let source = r#"opaque {*}"A\u0000B" "TAIL\u0000END""#;
    let image = tcl_lexer::SourceImage::native(source.as_bytes());
    let segments =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap();
    assert_eq!(segments.len(), 1);
    let tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        &segments[0],
    );
    assert!(matches!(tokens.words()[1], WordExpr::Expand { .. }));
    let args = tokens.argv_texts[1..].to_vec();
    let expand = tokens.expand_word.as_ref().unwrap();
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = dialect.native_source_string_protocol();
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image.clone());
    context.emit_expanded_call(&tokens.argv_texts[0], &args, expand, Some(&tokens));
    for expected in [b"A\xc0\x80B".as_slice(), b"TAIL\xc0\x80END".as_slice()] {
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == expected),
            "{:?}",
            context.literals.entries()
        );
    }
    assert_eq!(
        context
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::EXPAND_STKTOP)
            .count(),
        1
    );
    assert!(
        context
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::INVOKE_EXPANDED)
    );
    assert!(context.invocation_tokens.is_none());

    let mut missing = CodegenCtx::new(false, &[], &registry);
    missing.invocation_dialect = Some(dialect);
    missing.source_string_protocol = dialect.native_source_string_protocol();
    missing.ingress_lexer_config = Some(config);
    missing.set_source_image(image);
    missing.emit_expanded_call(&tokens.argv_texts[0], &args, expand, None);
    assert!(
        missing.literals.entries().iter().all(
            |literal| literal.bytes() != b"A\xc0\x80B" && literal.bytes() != b"TAIL\xc0\x80END"
        )
    );
    assert!(missing.invocation_tokens.is_none());
}

#[test]
fn emit_expanded_call_basic() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_expanded_call(
        "list head",
        &["ordinary".into(), "tail one tail two".into()],
        &[true, false, true],
        None,
    );
    let ops = opcodes(&ctx);
    assert_eq!(
        ops,
        vec![
            Op::EXPAND_START,
            Op::PUSH1,
            Op::EXPAND_STKTOP,
            Op::PUSH1,
            Op::PUSH1,
            Op::EXPAND_STKTOP,
            Op::INVOKE_EXPANDED,
            Op::POP,
        ],
        "head and tail words must use one ordered expansion path",
    );
}
