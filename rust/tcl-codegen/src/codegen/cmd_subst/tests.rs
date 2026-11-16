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
use tcl_registry::CommandRegistry;

#[test]
fn a_builtin_is_named_by_any_root_marker() {
    // tclsh8.5, 8.6, 9.0 and 9.1, `tcl::unsupported::disassemble script`:
    //   set x 1, ::set x 1, :::set x 1, ::::set x 1 -> push push storeStk done
    // A colon run of two or more is one root marker in every release.
    for head in ["set", "::set", ":::set", "::::set"] {
        assert!(names_builtin(head, "set"), "{head}");
    }
    for head in [":set", "n::set", "::n::set", "sett"] {
        assert!(!names_builtin(head, "set"), "{head}");
    }
    assert!(names_builtin("tcl::dict::with", "tcl::dict::with"));
    assert!(!names_builtin("::tcl::dict::with", "tcl::dict::with"));
}

fn original_argument_tokens(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) -> CommandTokens {
    let mut commands =
        tcl_compiler::segmenter::segment_commands_image_with_offset_and_config(image, 0, config)
            .unwrap();
    assert_eq!(commands.len(), 1);
    CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        &commands.remove(0),
    )
}

fn original_argument_context(
    registry: &CommandRegistry,
    image: tcl_lexer::SourceImage,
    version: tcl_dialect::TclVersion,
    config: tcl_lexer::LexerConfig,
) -> CodegenCtx<'_> {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let mut context = CodegenCtx::new(true, &[], registry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = dialect.native_source_string_protocol();
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image);
    context
}

#[test]
fn inline_source_recovery_keeps_ingress_expansion_and_quote_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Opaque compatibility syntax owns lexical shape, never Native admission.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let source = "list {*}$values";
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.dialect = Some(profile);
    for expanded in [false, true] {
        let config = tcl_lexer::LexerConfig {
            expand_syntax: expanded,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        context.ingress_lexer_config = Some(config);
        let recovered = context.original_inline_syntax(source).unwrap();
        let original = original_argument_tokens(&tcl_lexer::SourceImage::document(source), config);
        assert_eq!(recovered.argv, original.argv);
        assert_eq!(recovered.argv_texts, original.argv_texts);
        assert_eq!(
            matches!(recovered.words()[1], WordExpr::Expand { .. }),
            expanded
        );
        assert!(recovered.source_binding.is_none());
        assert!(context.native_entry.is_none());
    }
    let config = tcl_lexer::LexerConfig {
        quote_termination: tcl_dialect::QuoteTermination::Concatenating,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    context.ingress_lexer_config = Some(config);
    let concatenated = context.original_inline_syntax(r#"[list "A"B]"#).unwrap();
    assert_eq!(concatenated.words().len(), 2);
    context
        .ingress_lexer_config
        .as_mut()
        .unwrap()
        .quote_termination = tcl_dialect::QuoteTermination::Strict;
    assert!(context.original_inline_syntax(r#"[list "A"B]"#).is_none());
}

#[test]
fn retained_source_lexing_does_not_supply_native_invocation_admission() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        config,
    );
    let source = "lappend x {*}$values";
    let registry = context.commands();
    let bindings = tcl_compiler::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        tcl_compiler::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    let image = tcl_lexer::SourceImage::document(source);
    let mut tokens = original_argument_tokens(&image, config);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.original_lexer_config_for_tokens(&tokens).is_some());
    assert!(binding.admitted_inline_invocation().is_none());
    let parts = parse_cmd_parts_expand(source);
    let mut codegen = CodegenCtx::new(false, &[], registry);
    codegen.dialect = Some(profile);
    codegen.ingress_lexer_config = Some(config);
    codegen.set_source_image(image);
    codegen.with_invocation_tokens(Some(&tokens), |codegen| {
        assert!(codegen.entered_command_surrogate("lappend").is_none());
        assert!(!codegen.try_emit_expanded_native_call(&parts));
        assert_eq!(codegen.instructions, [] as [tcl_bytecode::Instruction; 0]);
    });
    codegen.with_invocation_tokens(None, |codegen| {
        assert!(codegen.entered_command_surrogate("lappend").is_none());
        assert!(!codegen.try_emit_expanded_native_call(&parts));
    });
    assert!(codegen.native_entry.is_none());
}

#[test]
fn original_inline_argument_emitters_keep_lexical_nul_and_escape_values() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Lexical emission and opcode wiring only; no native process or handler claim.
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        for (source, operation, expected) in [
            (r#"list "A\u0000B""#, Op::LIST, b"A\xc0\x80B".as_slice()),
            (
                r#"lrange "A\u0000B" 0 end"#,
                Op::LIST_RANGE_IMM,
                b"A\xc0\x80B".as_slice(),
            ),
            (
                r#"lreplace {one two} 0 0 "A\u0000B""#,
                Op::LREPLACE4,
                b"A\xc0\x80B".as_slice(),
            ),
            (
                r#"linsert {one} 0 "A\u0000B""#,
                Op::LREPLACE4,
                b"A\xc0\x80B".as_slice(),
            ),
            (
                r#"dict get "A\u0000B" key"#,
                Op::DICT_GET,
                b"A\xc0\x80B".as_slice(),
            ),
            (
                "lrange \"A\0B\" 0 end",
                Op::LIST_RANGE_IMM,
                b"A\0B".as_slice(),
            ),
            (
                r"lrange {A\u0000B} 0 end",
                Op::LIST_RANGE_IMM,
                br"A\u0000B".as_slice(),
            ),
            (
                r#"lrange "A\$literal\[\]\\Ω" 0 end"#,
                Op::LIST_RANGE_IMM,
                "A$literal[]\\Ω".as_bytes(),
            ),
        ] {
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let tokens = original_argument_tokens(&image, config);
            let parts = parse_cmd_parts(source);
            let mut context = original_argument_context(&registry, image, version, config);
            context.with_invocation_tokens(Some(&tokens), |context| {
                let args = &parts[1..];
                match parts[0].0.as_str() {
                    "list" => {
                        context.emit_inline_hook_operands(source, "list", args, Some(&tokens));
                    }
                    "lrange" => context.emit_inline_lrange(args),
                    "lreplace" => context.emit_inline_lreplace(args),
                    "linsert" => context.emit_inline_linsert(args),
                    "dict" => context.emit_inline_dict_get(args),
                    _ => unreachable!(),
                }
            });
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == operation),
                "{version:?}/{source}"
            );
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == expected),
                "{version:?}/{source}: {:?}",
                context.literals.entries()
            );
            assert!(context.invocation_tokens.is_none());
        }
    }
}

#[test]
fn original_regexp_argument_indices_survive_consumed_options() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let version = tcl_dialect::TclVersion::V8_6;
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
    );
    for source in [
        r#"regexp "P\u0000AT" "S\u0000UB""#,
        r#"regexp -- "P\u0000AT" "S\u0000UB""#,
        r#"regexp -- "S\u0000UB" "S\u0000UB""#,
        r#"regexp -nocase {abc} "S\u0000UB""#,
        r#"regexp -nocase -- {abc} "S\u0000UB""#,
    ] {
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let tokens = original_argument_tokens(&image, config);
        let parts = parse_cmd_parts(source);
        let mut context = original_argument_context(&registry, image, version, config);
        context.with_invocation_tokens(Some(&tokens), |context| {
            context.emit_inline_regexp(&parts[1..]);
        });
        assert!(
            context
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"S\xc0\x80UB"),
            "{source}"
        );
        if source.contains("-nocase") {
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::STR_MATCH)
            );
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal == "*abc*")
            );
        } else {
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::REGEXP)
            );
            let pattern = if source.contains("P\\u0000AT") {
                b"P\xc0\x80AT".as_slice()
            } else {
                b"S\xc0\x80UB".as_slice()
            };
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == pattern),
                "{source}"
            );
            assert_eq!(
                context
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(instruction.op, Op::PUSH1 | Op::PUSH4))
                    .count(),
                2
            );
        }
        assert!(
            context
                .literals
                .entries()
                .iter()
                .all(|literal| literal != "--" && literal != "-nocase")
        );
    }
}

#[test]
fn original_argument_ordinals_keep_worker_prefixes_and_derived_heads_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let version = tcl_dialect::TclVersion::V8_6;
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
    );
    let source = r#"worker "A\u0000B" key"#;
    let image = tcl_lexer::SourceImage::native(source.as_bytes());
    let tokens = original_argument_tokens(&image, config);
    let mut context = original_argument_context(&registry, image.clone(), version, config);
    let logical = vec![
        ("get".to_owned(), true),
        (r"A\u0000B".to_owned(), false),
        ("key".to_owned(), false),
    ];
    context.with_invocation_tokens(Some(&tokens), |context| {
        context.native_hook_layout = Some(("worker".to_owned(), 1));
        context.emit_inline_dict_get(&logical);
        context.emit_native_argument_word(0, "get", true);
    });
    assert!(
        context
            .literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"A\xc0\x80B")
    );
    assert!(
        context
            .literals
            .entries()
            .iter()
            .any(|literal| literal == "get")
    );
    assert!(context.native_hook_layout.is_none());

    let array_registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let array_source = r#"array names "A\u0000B" pattern"#;
    let array_image = tcl_lexer::SourceImage::native(array_source.as_bytes());
    let array_tokens = original_argument_tokens(&array_image, config);
    let array_parts = parse_cmd_parts(array_source);
    let mut array = original_argument_context(array_registry, array_image, version, config);
    array.with_invocation_tokens(Some(&array_tokens), |context| {
        context.emit_inline_array("array", &array_parts[1..]);
    });
    assert!(
        array
            .literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"A\xc0\x80B")
    );
    assert!(
        array
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::INVOKE_STK1)
    );
    assert!(
        array
            .literals
            .entries()
            .iter()
            .any(|literal| literal == "::tcl::array::names")
    );
}

#[test]
fn original_argument_emission_refuses_missing_stale_and_unavailable_lexical_receipts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let version = tcl_dialect::TclVersion::V8_6;
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
    );
    let source = r#"worker "A\u0000B" key"#;
    let image = tcl_lexer::SourceImage::native(source.as_bytes());
    let tokens = original_argument_tokens(&image, config);
    let logical = vec![
        ("get".to_owned(), true),
        (r"A\u0000B".to_owned(), false),
        ("key".to_owned(), false),
    ];
    // The same text and source image do not issue an original argument receipt.
    let mut missing = original_argument_context(&registry, image.clone(), version, config);
    missing.with_invocation_tokens(None, |context| context.emit_inline_dict_get(&logical));
    assert!(
        missing
            .literals
            .entries()
            .iter()
            .all(|literal| literal.bytes() != b"A\xc0\x80B")
    );
    assert!(missing.original_lexical_words.borrow().is_none());

    let mut stale = original_argument_context(&registry, image.clone(), version, config);
    stale.set_source("worker WRONG key");
    stale.with_invocation_tokens(Some(&tokens), |context| {
        context.native_hook_layout = Some(("worker".to_owned(), 1));
        context.emit_inline_dict_get(&logical);
    });
    assert!(
        stale
            .literals
            .entries()
            .iter()
            .all(|literal| literal.bytes() != b"A\xc0\x80B")
    );
    assert!(stale.original_lexical_words.borrow().is_none());

    let mut unavailable = original_argument_context(&registry, image.clone(), version, config);
    unavailable.source_string_protocol = None;
    unavailable.with_invocation_tokens(Some(&tokens), |context| {
        context.native_hook_layout = Some(("worker".to_owned(), 1));
        context.emit_inline_dict_get(&logical);
    });
    assert!(
        unavailable
            .literals
            .entries()
            .iter()
            .all(|literal| literal.bytes() != b"A\xc0\x80B")
    );
    assert!(unavailable.original_lexical_words.borrow().is_none());

    let mut altered = original_argument_context(&registry, image, version, config);
    altered.with_invocation_tokens(Some(&tokens), |context| {
        context.emit_native_argument_word(0, r"A\u0000CHANGED", false);
    });
    assert!(
        altered
            .literals
            .entries()
            .iter()
            .all(|literal| literal.bytes() != b"A\xc0\x80B"
                && literal.bytes() != b"A\xc0\x80CHANGED")
    );
}

#[test]
fn coroutine_emitters_preserve_native_namespace_and_expansion_segments() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let mut ctx = CodegenCtx::new(true, &[], registry);
    ctx.emit_inline_cmd_subst("[yield]");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::YIELD)
    );
    let mut ctx = CodegenCtx::new(true, &["relay"], registry);
    ctx.emit_inline_cmd_subst("[yieldto {*}$relay tail]");
    let ops = ctx
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect::<Vec<_>>();
    assert_eq!(ops.first(), Some(&Op::CURRENT_NAMESPACE), "{ops:?}");
    assert!(ops.contains(&Op::LIST_CONCAT), "{ops:?}");
    assert_eq!(ops.last(), Some(&Op::YIELD_TO_INVOKE), "{ops:?}");
    assert!(!ops.contains(&Op::INVOKE_EXPANDED), "{ops:?}");
    let old = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut ctx = CodegenCtx::new(true, &["relay"], old);
    ctx.emit_inline_cmd_subst("[yieldto {*}$relay tail]");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::INVOKE_EXPANDED)
    );
    assert!(
        !ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::YIELD_TO_INVOKE)
    );
}

#[test]
fn tailcall_value_emission_retains_native_namespace_and_original_words() {
    for (dialect, opcode) in [
        ("tcl8.6", Op::TAILCALL),
        ("tcl9.0", Op::TAILCALL),
        ("tcl9.1", Op::TAILCALL4),
    ] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let mut ctx = CodegenCtx::new(true, &[], registry);
        ctx.invocation_dialect = registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile);
        ctx.emit_inline_cmd_subst("[tailcall target {$literal}]");
        let ops = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>();
        assert!(ops.contains(&opcode), "{dialect}: {ops:?}");
        assert!(
            !ops.contains(&Op::POP),
            "the value bridge must not discard the result"
        );
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == "$literal")
        );
        let namespace = ops
            .iter()
            .position(|op| *op == Op::CURRENT_NAMESPACE)
            .unwrap();
        if dialect == "tcl9.1" {
            assert_eq!(namespace, 0);
        } else {
            assert!(namespace >= 2);
        }
    }
}

#[test]
fn tailcall_empty_and_expanded_native_lists_are_selected_only_in_91() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    for text in ["[tailcall]", "[tailcall {*}$args tail]"] {
        let mut ctx = CodegenCtx::new(true, &["args"], registry);
        ctx.emit_inline_cmd_subst(text);
        let ops = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>();
        assert_eq!(ops.first(), Some(&Op::CURRENT_NAMESPACE), "{text}: {ops:?}");
        assert_eq!(ops.last(), Some(&Op::TAILCALL_LIST), "{text}: {ops:?}");
        assert!(!ops.contains(&Op::INVOKE_EXPANDED));
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut ctx = CodegenCtx::new(true, &["args"], registry);
    ctx.emit_inline_cmd_subst("[tailcall {*}$args]");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::INVOKE_EXPANDED)
    );
    assert!(
        !ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::TAILCALL_LIST)
    );
}

#[test]
fn compiled_error_retains_optional_original_values_and_operation_context() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut ctx = CodegenCtx::new(true, &[], registry);
    ctx.emit_inline_cmd_subst("[error {$literal} {INFO} {CUSTOM CODE}]");
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "$literal")
    );
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "-errorinfo")
    );
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "-errorcode")
    );
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::LIST
                && instruction.operands == [Operand::Imm(4)])
    );
    let error = ctx.instructions.last().unwrap();
    assert_eq!(error.op, Op::RETURN_IMM);
    assert_eq!(error.operands, [Operand::Imm(1), Operand::Imm(0)]);
    assert!(matches!(
        error.error_stack_context,
        Some(tcl_bytecode::ErrorStackContext::ReturnImmediate { .. })
    ));
}

#[test]
fn original_native_operation_range_survives_argv_worker_replacement() {
    use tcl_runtime_api::CompileService;
    for (dialect, source, opcode) in [
        (
            "tcl8.6",
            "namespace origin [rename ::tcl::namespace::origin savedOrigin]",
            Op::ORIGIN_CMD,
        ),
        (
            "tcl9.0",
            "namespace origin [rename ::tcl::namespace::origin savedOrigin]",
            Op::ORIGIN_CMD,
        ),
        (
            "tcl9.1",
            "namespace origin [rename ::tcl::namespace::origin savedOrigin]",
            Op::ORIGIN_CMD,
        ),
        (
            "tcl9.1",
            "set x X; lappend x {*}[rename lappend savedAppend]",
            Op::LAPPEND_LIST,
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
        let module = service
            .compile_procedure_with_entry(
                tcl_runtime_api::ProcedureCompileTarget {
                    source,
                    namespace: "::",
                    parameters: &[],
                },
                profile,
                &entry,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        let instructions = &module.top_level_body.instructions;
        let operation = instructions
            .iter()
            .position(|instruction| instruction.op == opcode)
            .unwrap_or_else(|| panic!("{dialect}: missing original {opcode:?}: {source}"));
        assert!(
            instructions[..=operation].iter().any(|instruction| {
                instruction
                    .native_operation_selections
                    .iter()
                    .any(|selection| {
                        selection.source
                            == source
                                .split_once("; ")
                                .map_or(source, |(_, command)| command)
                            && selection.guard
                                == tcl_runtime_api::CommandBindingGuard::BeforeArguments
                            && selection.requirements.iter().all(|required| {
                                required.guard
                                    == tcl_runtime_api::CommandBindingGuard::BeforeArguments
                            })
                    })
            }),
            "{dialect}: {source}"
        );
    }
}

#[test]
fn namespace_code_literal_builder_is_shared_by_every_result_context() {
    use tcl_runtime_api::CompileService;
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) =
            crate::test_support::captured_native_namespace_entry_with_owner(profile, "::receiver");
        for source in [
            "namespace code {my tick}",
            "set result [namespace code {my tick}]",
            "return [namespace code {my tick}]",
            "catch {namespace code {my tick}} result",
            "try {error FAIL} on error {message options} {namespace code {my tick}}",
        ] {
            let module = service
                .compile_procedure_with_entry(
                    tcl_runtime_api::ProcedureCompileTarget {
                        source,
                        namespace: "::receiver",
                        parameters: &[],
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .unwrap();
            let count = module
                .top_level_body
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::CURRENT_NAMESPACE)
                .count();
            assert_eq!(
                count,
                usize::from(matches!(dialect, "tcl8.6" | "tcl9.0" | "tcl9.1")),
                "{dialect}: {source}"
            );
        }
        for source in [
            "namespace code [list my tick]",
            "namespace code {::namespace inscope :: my}",
        ] {
            let module = service
                .compile_procedure_with_entry(
                    tcl_runtime_api::ProcedureCompileTarget {
                        source,
                        namespace: "::receiver",
                        parameters: &[],
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .unwrap();
            assert!(
                !module
                    .top_level_body
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::CURRENT_NAMESPACE),
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn namespace_origin_opcode_is_shared_by_every_result_context() {
    use tcl_runtime_api::CompileService;
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
        for source in [
            "namespace origin target",
            "set result [namespace origin target]",
            "return [namespace origin target]",
            "catch {namespace origin target} result",
            "try {error FAIL} on error {message options} {namespace origin target}",
        ] {
            let module = service
                .compile_procedure_with_entry(
                    tcl_runtime_api::ProcedureCompileTarget {
                        source,
                        namespace: "::",
                        parameters: &[],
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .unwrap();
            let count = module
                .top_level_body
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::ORIGIN_CMD)
                .count();
            assert_eq!(
                count,
                usize::from(matches!(dialect, "tcl8.6" | "tcl9.0" | "tcl9.1")),
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn current_namespace_emission_requires_its_actual_native_compiler_hook() {
    use tcl_runtime_api::CompileService;
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
        let module = service
            .compile_procedure_with_entry(
                tcl_runtime_api::ProcedureCompileTarget {
                    source: "set result [namespace current]",
                    namespace: "::",
                    parameters: &[],
                },
                profile,
                &entry,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        let selected = module
            .top_level_body
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::CURRENT_NAMESPACE);
        assert_eq!(
            selected,
            matches!(dialect, "tcl8.6" | "tcl9.0" | "tcl9.1"),
            "{dialect}"
        );
        let statement = service
            .compile_procedure_with_entry(
                tcl_runtime_api::ProcedureCompileTarget {
                    source: "namespace current",
                    namespace: "::",
                    parameters: &[],
                },
                profile,
                &entry,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        assert_eq!(
            statement
                .top_level_body
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::CURRENT_NAMESPACE),
            selected,
            "statement: {dialect}",
        );
    }
}

/// A value-position `[expr {…}]` is re-parsed here, and must be re-parsed
/// under the compile dialect: dialect-blind, an iRules word operator lexes
/// as a function name, the parse falls back to `ExprNode::Raw`, and codegen
/// pushes the source text for a second dialect-blind parse in the VM —
/// which returns the text itself rather than evaluating the operator.
#[test]
fn inline_expr_subst_parses_under_the_compile_dialect() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let mut ctx = CodegenCtx::new(true, &["x"], registry);
    ctx.dialect =
        Some(tcl_registry::model::ingress::resolve_environment(profile.name).analyser_profile());
    ctx.emit_inline_cmd_subst("[expr {$x contains \"a\"}]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::IRULE_CONTAINS), "{ops:?}");
}

/// Public codegen consumers may supply a profile-projected registry
/// without separately setting the optional module dialect. Nested source
/// must still follow that registry's grammar rather than ambient Tcl.
#[test]
fn inline_cmd_subst_expansion_uses_the_profiled_registry_grammar() {
    let f5_registry = tcl_registry::model::ingress::static_context_for_profile(
        tcl_dialect::DialectProfile::irules(),
    )
    .commands();
    let mut f5 = CodegenCtx::new(true, &["head"], f5_registry);
    assert!(f5.dialect.is_none());
    f5.emit_inline_cmd_subst("[{*}$head ordinary]");
    let f5_ops: Vec<Op> = f5
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert!(!f5_ops.contains(&Op::EXPAND_STKTOP), "{f5_ops:?}");
    assert!(!f5_ops.contains(&Op::INVOKE_EXPANDED), "{f5_ops:?}");

    let modern_profile =
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let modern_registry =
        tcl_registry::model::ingress::static_context_for_profile(modern_profile).commands();
    let mut modern = CodegenCtx::new(true, &["head"], modern_registry);
    assert!(modern.dialect.is_none());
    modern.emit_inline_cmd_subst("[{*}$head ordinary]");
    let modern_ops: Vec<Op> = modern
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert!(modern_ops.contains(&Op::EXPAND_STKTOP), "{modern_ops:?}");
    assert!(modern_ops.contains(&Op::INVOKE_EXPANDED), "{modern_ops:?}");
}

/// The same site's release axis: an operator the target release lacks is
/// not specialised at all, so the VM raises the interpreter's own
/// diagnostic instead of executing an opcode 8.4 has no grammar for.
#[test]
fn inline_expr_subst_respects_the_target_release() {
    let registry = CommandRegistry::build_default();

    let mut old = CodegenCtx::new(true, &[], &registry);
    old.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile());
    old.emit_inline_cmd_subst("[expr {2 ** 3}]");
    let ops: Vec<Op> = old.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::EXPR_STK), "{ops:?}");
    assert!(old.literals.entries().iter().any(|l| l == "2 ** 3"));

    let mut modern = CodegenCtx::new(true, &[], &registry);
    modern.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile());
    modern.emit_inline_cmd_subst("[expr {2 ** 3}]");
    let ops: Vec<Op> = modern.instructions.iter().map(|i| i.op).collect();
    assert!(!ops.contains(&Op::EXPR_STK), "{ops:?}");
    assert!(modern.literals.entries().iter().any(|l| l == "8"));
}

// unroll_nested_set.

#[test]
fn unroll_simple() {
    let r = unroll_nested_set("[set y [set z 42]]").unwrap();
    assert_eq!(r, vec!["y", "z", "42"]);
}

#[test]
fn unroll_not_nested() {
    assert!(unroll_nested_set("hello").is_none());
}

// is_pure_cmd_subst.

#[test]
fn pure_cmd_subst_simple() {
    assert!(is_pure_cmd_subst("[expr {1+2}]"));
}

#[test]
fn pure_cmd_subst_nested() {
    assert!(is_pure_cmd_subst("[set x [set y 1]]"));
}

#[test]
fn pure_cmd_subst_not() {
    assert!(!is_pure_cmd_subst("hello"));
    assert!(!is_pure_cmd_subst("[a] [b]"));
    assert!(!is_pure_cmd_subst("[llength $args]:[join $args ,]"));
}

// has_command_separator.

#[test]
fn separator_semicolon() {
    assert!(has_command_separator("set x 1; set y 2"));
}

#[test]
fn separator_newline() {
    assert!(has_command_separator("set x 1\nset y 2"));
}

#[test]
fn separator_in_braces_ignored() {
    assert!(!has_command_separator("{set x 1; set y 2}"));
}

#[test]
fn separator_in_quotes_ignored() {
    assert!(!has_command_separator("\"set x 1; set y 2\""));
}

#[test]
fn no_separator() {
    assert!(!has_command_separator("set x 1"));
}

#[test]
fn original_substitution_geometry_preserves_braces_quotes_and_comments() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        for substitution in [
            "[list {]}]",
            "[list \"]\"]",
            "[list a; # ] ignored\nlist b]",
            "[list {[}]",
            "[list a; list b]",
        ] {
            assert!(is_pure_cmd_subst_with_config(substitution, config));
        }
        assert!(!is_pure_cmd_subst_with_config(
            "[list {]}]:[list z]",
            config
        ));
        assert!(!is_pure_cmd_subst_with_config("[list {]", config));
        assert!(!has_command_separator_with_config("[list \";\"]", config));
        assert!(has_command_separator_with_config(
            "[list a; list b]",
            config
        ));
        assert!(parse_cmd_parts_with_config("[list {]", config).is_none());
    }
}

// parse_cmd_parts.

#[test]
fn standalone_default_source_parts_preserve_outer_presentation_whitespace() {
    assert_eq!(
        parse_cmd_parts(" \t[list {a [inert]}]\n"),
        vec![("list".into(), false), ("a [inert]".into(), true),]
    );
    assert_eq!(
        parse_cmd_parts_expand(" \t[list {*}$args]\n"),
        vec![("list".into(), false, false), ("$args".into(), false, true),]
    );
    let actual =
        parse_cmd_parts_with_config(" \t[list a]\n", tcl_lexer::LexerConfig::default()).unwrap();
    assert_eq!(actual, vec![("[list a]".into(), false)]);
}

#[test]
fn unavailable_configured_word_projection_keeps_the_actual_script_for_eval() {
    let registry = tcl_registry::model::ingress::static_context_for("jimtcl").commands();
    let mut context = CodegenCtx::new(false, &[], registry);
    context.ingress_lexer_config = Some(tcl_lexer::LexerConfig::for_file_grammar(
        registry
            .profile()
            .expect("selected Jim fixture profile")
            .grammar,
    ));
    context.emit_inline_cmd_subst(r#"[capture "a"b]"#);
    assert!(
        context
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::EVAL_STK)
    );
    assert_eq!(context.literals.entries(), &[r#"capture "a"b"#.to_owned()]);
    assert!(context.command_binding_requirements.is_empty());
}

#[test]
fn parse_simple_cmd() {
    let parts = parse_cmd_parts("[set x 42]");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], ("set".into(), false));
    assert_eq!(parts[1], ("x".into(), false));
    assert_eq!(parts[2], ("42".into(), false));
}

#[test]
fn parse_braced_arg() {
    let parts = parse_cmd_parts("[expr {1+2}]");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], ("expr".into(), false));
    assert_eq!(parts[1], ("1+2".into(), true));
}

#[test]
fn parse_quoted_arg() {
    let parts = parse_cmd_parts("[puts \"hello world\"]");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], ("puts".into(), false));
    assert_eq!(parts[1], ("hello world".into(), false));
}

#[test]
fn parse_nested_cmd() {
    let parts = parse_cmd_parts("[set x [expr {1+2}]]");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], ("set".into(), false));
    assert_eq!(parts[1], ("x".into(), false));
    assert_eq!(parts[2], ("[expr {1+2}]".into(), false));
}

// emit_cmd_subst_arg.

#[test]
fn emit_arg_literal() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_cmd_subst_arg("hello", false);
    assert_eq!(ctx.instructions[0].op, Op::PUSH1);
}

#[test]
fn emit_braced_arg_as_exact_verbatim_literal() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_cmd_subst_arg("{}", true);
    assert_eq!(ctx.instructions[0].op, Op::PUSH1);
    assert_eq!(
        ctx.instructions[0].literal_rep,
        tcl_bytecode::LiteralRep::Verbatim
    );
    assert_eq!(ctx.literals.entries(), &["{}".to_owned()]);

    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_cmd_subst_arg("$x", true);
    assert_eq!(ctx.instructions[0].op, Op::PUSH1);
    assert_eq!(
        ctx.instructions[0].literal_rep,
        tcl_bytecode::LiteralRep::Verbatim
    );
    assert_eq!(ctx.literals.entries(), &["$x".to_owned()]);
}

#[test]
fn emit_arg_var_ref() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_cmd_subst_arg("${x}", false);
    assert_eq!(ctx.instructions[0].op, Op::LOAD_SCALAR1);
}

// emit_generic_cmd_subst.

#[test]
fn emit_generic_simple() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_generic_cmd_subst("puts", &[("hello".into(), false)]);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert_eq!(ops, vec![Op::PUSH1, Op::PUSH1, Op::INVOKE_STK1]);
}

// emit_inline_cmd_subst.

#[test]
fn inline_expr() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    // A variable operand keeps the ADD opcode; a constant `1+2` would be
    // folded to a single push by the codegen-time expr const-folder.
    ctx.emit_inline_cmd_subst("[expr {$x+2}]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::ADD));
}

#[test]
fn inline_incr_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_inline_cmd_subst("[incr x]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::INCR_SCALAR1_IMM));
}

#[test]
fn inline_incr_braced_amount_does_not_evaluate_its_command_text() {
    let registry = CommandRegistry::build_default();
    for local in [true, false] {
        let mut ctx = CodegenCtx::new(local, &["x"], &registry);
        ctx.emit_inline_cmd_subst("[incr x {[set ::seen 4]}]");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == "[set ::seen 4]")
        );
        assert!(!ctx.instructions.iter().any(|instruction| matches!(
            instruction.op,
            Op::LOAD_SCALAR1 | Op::LOAD_STK | Op::INVOKE_STK1 | Op::INVOKE_STK4
        )));
        assert!(ctx.instructions.iter().any(|instruction| instruction.op
            == if local {
                Op::INCR_SCALAR1
            } else {
                Op::INCR_STK
            }));
    }
}

#[test]
fn inline_string_length() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[string length ${x}]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::STR_LEN));
}

fn registry_with_custom_string_command() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut text = registry.get("string").expect("string spec").clone();
    text.name = "text";
    text.surface = None;
    registry.insert(text);
    registry
}

#[test]
fn registry_hook_alone_does_not_capture_a_generic_string_fallback() {
    let registry = registry_with_custom_string_command();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[text is integer -failindex fi 1.5]");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| { matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4) })
    );
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "text")
    );
    assert!(ctx.literals.entries().iter().any(|literal| literal == "is"));
    assert!(
        !ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "string")
    );
    assert!(
        ctx.instructions
            .iter()
            .all(|instruction| instruction.entered_command.is_none())
    );
    assert!(ctx.command_binding_requirements.is_empty());
}

#[test]
fn registry_hook_alone_does_not_turn_a_named_string_call_into_a_token_capture() {
    let registry = registry_with_custom_string_command();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[text equal -nocase a A]");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| { matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4) })
    );
    assert!(ctx.instructions.iter().all(|instruction| {
        instruction.op != Op::INVOKE_REPLACE && instruction.entered_command.is_none()
    }));
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "text")
    );
    assert!(ctx.command_binding_requirements.is_empty());
}

#[test]
fn a_statement_hook_declines_an_unproved_named_string_invocation() {
    let registry = registry_with_custom_string_command();
    let args = vec!["equal", "-nocase", "a", "A"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let mut used_generic_invoke = false;
    assert!(!ctx.try_inline_statement_codegen("text", &args, None, &mut used_generic_invoke));
    assert!(!used_generic_invoke);
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(ctx.command_binding_requirements.is_empty());
}

#[test]
fn inline_list() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[list a b c]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::LIST));
}

#[test]
fn inline_multicommand_falls_back() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[set x 1; set y 2]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::EVAL_STK));
}

// Regression: label reconstruction in string equal/compare.

/// `string equal` in non-proc context with a nested command
/// substitution in one arg. The nested substitution allocates
/// labels (via `fresh_label`), so if the fast-path reconstructed
/// the end-label name from `label_counter - 1` it would resolve
/// to a different label.
#[test]
fn inline_string_equal_nested_cmd_label_resolves() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    // Nested `[expr ...]` does not allocate labels, but nested
    // `[array names ...]` wraps in a startCommand, which does.
    ctx.emit_inline_cmd_subst("[string equal [array names a] foo]");
    // Build expected: the startCommand end label should resolve
    // to the `STR_EQ` position (or later), not a dangling index.
    let has_str_eq = ctx.instructions.iter().any(|i| i.op == Op::STR_EQ);
    assert!(has_str_eq, "expected STR_EQ in output");
    // All Label operands must resolve (i.e. exist in label_positions).
    for instr in &ctx.instructions {
        for op in &instr.operands {
            if let Operand::Label(l) = op {
                assert!(
                    ctx.label_positions.contains_key(l),
                    "unresolved label {l:?} in {:?}",
                    instr.op
                );
            }
        }
    }
}

#[test]
fn inline_string_compare_nested_cmd_label_resolves() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_inline_cmd_subst("[string compare [array names a] foo]");
    let has_str_cmp = ctx.instructions.iter().any(|i| i.op == Op::STR_CMP);
    assert!(has_str_cmp, "expected STR_CMP in output");
    for instr in &ctx.instructions {
        for op in &instr.operands {
            if let Operand::Label(l) = op {
                assert!(
                    ctx.label_positions.contains_key(l),
                    "unresolved label {l:?} in {:?}",
                    instr.op
                );
            }
        }
    }
}

// Specialised value-emission paths.

#[test]
fn try_list_expand_concat_matches_two_vars() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(ctx.try_list_expand_concat("[list {*}$a {*}$b]"));
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    // push "a"; loadStk; push "b"; loadStk; listConcat
    assert_eq!(
        ops,
        vec![
            Op::PUSH1,
            Op::LOAD_STK,
            Op::PUSH1,
            Op::LOAD_STK,
            Op::LIST_CONCAT,
        ]
    );
}

#[test]
fn list_expand_concat_keeps_configured_whole_scalar_words() {
    let registry = CommandRegistry::build_default();
    for (source, names) in [
        (
            "[list {*}${with space} {*}${$literal}]",
            ["with space", "$literal"],
        ),
        ("[list {*}${::N::value} {*}${}]", ["::N::value", ""]),
    ] {
        let mut context = CodegenCtx::new(false, &[], &registry);
        assert!(context.try_list_expand_concat(source), "{source}");
        for name in names {
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal == name)
            );
        }
        assert_eq!(context.instructions.last().unwrap().op, Op::LIST_CONCAT);
        assert_eq!(context.command_binding_requirements.len(), 1);
    }
    for source in [
        "[list{*}$a {*}$b]",
        "[list {*}$a {*}$b; missing]",
        "[list {*}$array(key) {*}$b]",
        "[list {*}${array(key)} {*}$b]",
        "[list {*}$a-tail {*}$b]",
        "[list {*}{$a} {*}$b]",
    ] {
        let mut context = CodegenCtx::new(false, &[], &registry);
        assert!(!context.try_list_expand_concat(source), "{source}");
        assert_eq!(context.instructions, [] as [tcl_bytecode::Instruction; 0]);
        assert!(context.command_binding_requirements.is_empty());
    }
    let source = "[list {*}${a{b}c} {*}$other]";
    for (style, expected) in [
        (tcl_dialect::BracedVarStyle::FirstClose, false),
        (tcl_dialect::BracedVarStyle::Tcl9Nesting, true),
    ] {
        let mut context = CodegenCtx::new(false, &[], &registry);
        context.ingress_lexer_config = Some(tcl_lexer::LexerConfig {
            braced_var: style,
            ..context.lexer_config()
        });
        assert_eq!(context.try_list_expand_concat(source), expected);
    }
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.ingress_lexer_config = Some(tcl_lexer::LexerConfig {
        expand_syntax: false,
        ..context.lexer_config()
    });
    assert!(!context.try_list_expand_concat("[list {*}$a {*}$b]"));
    assert_eq!(context.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(context.command_binding_requirements.is_empty());
}

#[test]
fn try_list_expand_concat_rejects_single_var() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(!ctx.try_list_expand_concat("[list {*}$a]"));
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
}

#[test]
fn try_list_expand_concat_rejects_three_vars() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(!ctx.try_list_expand_concat("[list {*}$a {*}$b {*}$c]"));
}

#[test]
fn try_list_expand_concat_rejects_non_expanded_arg() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    // Literal first arg without {*} prefix — falls back to generic path.
    assert!(!ctx.try_list_expand_concat("[list a {*}$b]"));
}

#[test]
fn list_expand_concat_requires_the_owned_registry_list_hook() {
    let mut registry = CommandRegistry::build_default();
    let mut list = registry.get("list").expect("list spec").clone();
    list.inline_codegen_hook = Some(InlineCodegenHookId::Expr);
    registry.insert(list);

    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(!ctx.try_list_expand_concat("[list {*}$a {*}$b]"));
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert_eq!(ctx.command_binding_requirements.len(), 0);
}

#[test]
fn try_inline_list_without_target_emits_break_as_literal() {
    // When `[list ... [break] ...]` appears without
    // a loop target in scope, the pattern still claims the value
    // and emits `[break]` as a literal list element. The generic
    // fallback is never reached.
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    // No JUMP4 since no break target.
    assert!(!ops.contains(&Op::JUMP4));
    // Still emits LIST N at the end.
    assert!(ops.contains(&Op::LIST));
}

#[test]
fn try_inline_list_with_break_emits_jump_to_target() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.break_target = Some("loop_break_1".into());
    assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    // push "a"; startCommand; pop; jump4 break_target; push "c"; list 3
    assert!(ops.contains(&Op::JUMP4), "expected JUMP4, got {ops:?}");
    assert!(
        ops.contains(&Op::START_CMD),
        "expected START_CMD, got {ops:?}"
    );
    assert!(ops.contains(&Op::LIST), "expected LIST, got {ops:?}");
    for name in ["list", "break"] {
        assert!(
            ctx.command_binding_requirements
                .contains(&tcl_runtime_api::CommandBindingIdentity::new(name, name)),
            "missing {name} dependency"
        );
    }
}

#[test]
fn inline_list_control_requires_owned_registry_control_hooks() {
    for (command, source, target_break) in [
        ("break", "[list a [break] c]", true),
        ("continue", "[list a [continue] c]", false),
    ] {
        let mut registry = CommandRegistry::build_default();
        let mut control = registry.get(command).expect("control spec").clone();
        control.inline_codegen_hook = Some(InlineCodegenHookId::Error);
        registry.insert(control);

        let mut ctx = CodegenCtx::new(true, &[], &registry);
        if target_break {
            ctx.break_target = Some("loop_break_1".into());
        } else {
            ctx.continue_target = Some("loop_continue_1".into());
        }
        assert!(
            !ctx.try_inline_list_with_break_continue(source),
            "owned {command} hook must control the jump specialisation"
        );
        assert_eq!(ctx.instructions.len(), 0, "partial emission for {command}");
        assert_eq!(ctx.command_binding_requirements.len(), 0);
    }
}

#[test]
fn try_inline_list_without_break_returns_false() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.break_target = Some("loop_break_1".into());
    // No [break]/[continue] inside — should not match.
    assert!(!ctx.try_inline_list_with_break_continue("[list a b c]"));
}

/// `lrange` with non-literal indices must not push the list arg
/// twice when falling back to the generic invoke path.
#[test]
fn inline_lrange_variable_indices_no_double_push() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["lst", "a", "b"], &registry);
    ctx.emit_inline_cmd_subst("[lrange ${lst} ${a} ${b}]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    // Should be: push "lrange"; load lst; load a; load b; invokeStk1 4
    // → 1 push, 3 loads, 1 invoke. NOT: load lst; push "lrange"; ...
    let load_count = ops
        .iter()
        .filter(|o| matches!(o, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4))
        .count();
    assert_eq!(load_count, 3, "expected 3 var loads, got {ops:?}");
    let invoke_count = ops.iter().filter(|o| **o == Op::INVOKE_STK1).count();
    assert_eq!(invoke_count, 1, "expected one invokeStk1, got {ops:?}");
}

#[test]
fn inline_lrange_end_plus_n_falls_back_to_non_imm() {
    // An `end+N` index is left to the opcode that reads it at run time:
    // 8.4 to 8.6 sum it with `end` in an `int`, which a long enough value
    // wraps, so the emitter does not fix it as LIST_RANGE_IMM.
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
    ctx.emit_inline_cmd_subst("[lrange ${lst} 0 end+1]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(
        !ops.contains(&Op::LIST_RANGE_IMM),
        "end+1 must not emit LIST_RANGE_IMM, got {ops:?}",
    );
}

#[test]
fn inline_lrange_end_uses_imm() {
    // A plain `end` index is valid and still takes the fast immediate path.
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
    ctx.emit_inline_cmd_subst("[lrange ${lst} 0 end]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::LIST_RANGE_IMM),
        "end should use LIST_RANGE_IMM, got {ops:?}",
    );
}

// Registry drift: inline codegen hook stamping.

/// Registry inline hooks match the supported emitter set and dispatch purpose.
const EXPECTED_INLINE_HOOKS: &[(&str, tcl_registry::hooks::InlineCodegenHookId)] = &[
    ("expr", tcl_registry::hooks::InlineCodegenHookId::Expr),
    ("incr", tcl_registry::hooks::InlineCodegenHookId::Incr),
    ("set", tcl_registry::hooks::InlineCodegenHookId::Set),
    ("string", tcl_registry::hooks::InlineCodegenHookId::String),
    ("lindex", tcl_registry::hooks::InlineCodegenHookId::Lindex),
    ("lrange", tcl_registry::hooks::InlineCodegenHookId::Lrange),
    (
        "lreplace",
        tcl_registry::hooks::InlineCodegenHookId::Lreplace,
    ),
    ("linsert", tcl_registry::hooks::InlineCodegenHookId::Linsert),
    ("regexp", tcl_registry::hooks::InlineCodegenHookId::Regexp),
    ("list", tcl_registry::hooks::InlineCodegenHookId::List),
    ("array", tcl_registry::hooks::InlineCodegenHookId::Array),
    ("catch", tcl_registry::hooks::InlineCodegenHookId::Catch),
    ("return", tcl_registry::hooks::InlineCodegenHookId::Return),
    ("error", tcl_registry::hooks::InlineCodegenHookId::Error),
    ("break", tcl_registry::hooks::InlineCodegenHookId::Break),
    (
        "continue",
        tcl_registry::hooks::InlineCodegenHookId::Continue,
    ),
    ("try", tcl_registry::hooks::InlineCodegenHookId::Try),
    ("yield", tcl_registry::hooks::InlineCodegenHookId::Yield),
    ("yieldto", tcl_registry::hooks::InlineCodegenHookId::YieldTo),
];

const EXPECTED_INLINE_SUBCOMMANDS: [(&str, &str, tcl_registry::hooks::InlineCodegenHookId); 8] = [
    (
        "info",
        "exists",
        tcl_registry::hooks::InlineCodegenHookId::InfoExists,
    ),
    (
        "info",
        "commands",
        tcl_registry::hooks::InlineCodegenHookId::InfoCommandsResolve,
    ),
    (
        "info",
        "level",
        tcl_registry::hooks::InlineCodegenHookId::InfoLevel,
    ),
    (
        "namespace",
        "current",
        tcl_registry::hooks::InlineCodegenHookId::NamespaceCurrent,
    ),
    (
        "namespace",
        "origin",
        tcl_registry::hooks::InlineCodegenHookId::NamespaceOrigin,
    ),
    (
        "namespace",
        "code",
        tcl_registry::hooks::InlineCodegenHookId::NamespaceCode,
    ),
    (
        "dict",
        "get",
        tcl_registry::hooks::InlineCodegenHookId::DictGet,
    ),
    (
        "string",
        "match",
        tcl_registry::hooks::InlineCodegenHookId::StringMatch,
    ),
];

#[test]
fn registry_inline_hook_stamping_matches_previous_hardcoded_dispatch() {
    use tcl_registry::hooks::InlineCodegenHookId as H;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    );
    let expected = EXPECTED_INLINE_HOOKS;
    for (name, hook) in expected {
        assert_eq!(
            registry.get(name).and_then(|s| s.inline_codegen_hook),
            Some(*hook),
            "{name} must carry the {hook:?} inline hook"
        );
    }
    // Subcommand-keyed hooks.
    assert_eq!(
        registry
            .get("info")
            .and_then(|s| s.subcommand("exists"))
            .and_then(|s| s.inline_codegen_hook),
        Some(H::InfoExists)
    );
    assert_eq!(
        registry
            .get("dict")
            .and_then(|s| s.subcommand("get"))
            .and_then(|s| s.inline_codegen_hook),
        Some(H::DictGet)
    );
    assert_eq!(
        registry
            .get("string")
            .and_then(|s| s.subcommand("match"))
            .and_then(|s| s.inline_codegen_hook),
        Some(H::StringMatch)
    );
    // The `string` subcommands 8.6 gave compile procedures carry their
    // hooks as windows from that release.
    for (sub, hook) in [
        ("is", H::StringIs),
        ("replace", H::StringReplace),
        ("trim", H::StringTrim),
        ("trimleft", H::StringTrimLeft),
        ("trimright", H::StringTrimRight),
    ] {
        let windows = registry
            .get("string")
            .and_then(|s| s.subcommand(sub))
            .map_or_default(|s| s.inline_codegen_hook_windows);
        assert_eq!(
            windows.iter().map(|w| w.value).collect::<Vec<_>>(),
            [hook],
            "string {sub}"
        );
    }
    let expected_subcommands = EXPECTED_INLINE_SUBCOMMANDS;
    for (command, member, hook) in expected_subcommands {
        assert_eq!(
            registry
                .get(command)
                .and_then(|spec| spec.subcommand(member))
                .and_then(|sub| sub.inline_codegen_hook),
            Some(hook),
            "{command} {member}",
        );
    }
    // Stamping any further hook requires extending this audited inventory.
    let expected_cmds: std::collections::HashSet<&str> = expected.iter().map(|(n, _)| *n).collect();
    let names: Vec<String> = registry.command_names().map(str::to_owned).collect();
    for name in &names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        if spec.inline_codegen_hook.is_some() {
            assert!(
                expected_cmds.contains(name.as_str()),
                "unexpected command-level inline hook on {name}"
            );
        }
        for sub in spec.subcommands {
            if sub.inline_codegen_hook.is_some() {
                assert!(
                    expected_subcommands.iter().any(|(command, member, hook)| {
                        name == command
                            && sub.name == *member
                            && sub.inline_codegen_hook == Some(*hook)
                    }),
                    "unexpected subcommand-level inline hook on {name} {}",
                    sub.name
                );
            }
        }
    }
}

/// Catch-body-only hooks use generic invocation in value position.
#[test]
fn catch_body_only_hooks_stay_generic_in_value_position() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[break]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(
        !ops.contains(&Op::BREAK),
        "no inline break in value position"
    );
    assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke expected");
}

#[test]
fn a_literal_insert_index_past_int_is_respelt_where_the_release_compiles_it() {
    // tclsh 8.6.18: `linsert $l end+2147483647 X` in a proc appends, as
    // `TclCompileLinsertCmd` encodes the index after the end; read at run
    // time it wraps and prepends. The VM's `LREPLACE4` reads its index at
    // run time, so a compiled command pushes it respelt `2147483647`.
    let registry = CommandRegistry::build_default();
    for (dialect, direct, pushed) in [
        ("tcl8.6", false, "2147483647"),
        ("tcl8.6", true, "end+2147483647"),
        ("tcl8.5", false, "end+2147483647"),
        ("tcl9.0", false, "end+2147483647"),
    ] {
        let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
        ctx.dialect = tcl_dialect::DialectProfile::find(dialect);
        if direct {
            let span = tcl_lexer::Span::new(0, 1);
            ctx.direct_commands.push(span);
            ctx.set_command_source_span(span);
        }
        ctx.emit_inline_cmd_subst("[linsert ${lst} end+2147483647 X]");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == pushed),
            "{dialect}, direct: {direct}: {:?}",
            ctx.literals.entries()
        );
    }
}

/// 8.6 compiles a literal `end` offset past `INT_MAX` after the end, which
/// tclsh 8.6.18 shows as `lrange {a b c} 0 end+2147483647` in a proc
/// ranging the whole list; 8.5.19 wraps it before the first element
/// either way, so its command reads it. Only a command the release
/// compiles is fixed: one of the script file `tclsh` runs is read as run.
#[test]
fn a_literal_end_offset_past_int_is_fixed_where_the_release_compiles_it() {
    let registry = CommandRegistry::build_default();
    for (dialect, direct, last) in [
        ("tcl8.6", false, Some(i32::MAX)),
        ("tcl8.6", true, None),
        ("tcl8.5", false, None),
    ] {
        let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
        ctx.dialect = tcl_dialect::DialectProfile::find(dialect);
        if direct {
            let span = tcl_lexer::Span::new(0, 1);
            ctx.direct_commands.push(span);
            ctx.set_command_source_span(span);
        }
        ctx.emit_inline_cmd_subst("[lrange ${lst} 0 end+2147483647]");
        let range = ctx
            .instructions
            .iter()
            .find(|instruction| instruction.op == Op::LIST_RANGE_IMM);
        assert_eq!(
            range.map(|instruction| instruction.operands[1].clone()),
            last.map(Operand::Imm),
            "{dialect}, direct: {direct}"
        );
    }
}

/// A `::`-qualified head word naming a global builtin compiles as the
/// builtin does: C's compiler finds `::expr` and compiles it with
/// `TclCompileExprCmd`.
#[test]
fn qualified_head_word_compiles_as_the_builtin() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_inline_cmd_subst("[::expr {$x+2}]");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(
        ops.contains(&Op::ADD),
        "qualified ::expr inlines, got {ops:?}"
    );
    assert!(
        !ops.contains(&Op::INVOKE_STK1),
        "qualified ::expr compiles, got {ops:?}"
    );
}
