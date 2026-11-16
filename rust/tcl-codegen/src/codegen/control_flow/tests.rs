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
use tcl_compiler::cfg::Block;
use tcl_registry::CommandRegistry;

#[test]
fn original_inline_body_tokens_keep_nested_bom_and_parent_point_correspondence() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original source geometry only; this control supplies no Native compilation admission.
    let mut prepared = tcl_compiler::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let source = "catch {\u{feff}unknown argument}";
    let bindings =
        tcl_compiler::realm::document_realm_bindings_with_resolved_input(source, &input, None);
    let command = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    )
    .pop()
    .unwrap();
    let mut parent = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &command,
    );
    bindings
        .source_bindings_ref()
        .stamp_original_tokens(&mut parent);
    let body = "\u{feff}unknown argument";
    let mut context = CodegenCtx::new(true, &[], registry);
    context.ingress_lexer_config = Some(input.lexer_config());
    context.set_source_image(tcl_lexer::SourceImage::document(source));
    context.inline_body_source_base = Some(7);
    context.with_invocation_tokens(Some(&parent), |context| {
        let child = context.inline_body_command_tokens(body, 0).unwrap();
        assert_eq!(child.argv_texts, ["\u{feff}unknown", "argument"]);
        assert_eq!(child.argv[0].start(), 7);
        assert_eq!(
            source.get(child.argv[0].as_range()),
            Some("\u{feff}unknown")
        );
        let site = child
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        assert_eq!(site.offset, 7);
        assert_eq!(site.source.source_image().bytes(), source.as_bytes());
        assert_eq!(context.ingress_lexer_config, Some(input.lexer_config()));
    });
    // A real foreign parent cannot lend its child point to equal-position
    // text from this body's source instance.
    let other = "catch {foreign argument}";
    let bindings =
        tcl_compiler::realm::document_realm_bindings_with_resolved_input(other, &input, None);
    let command = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        other,
        0,
        input.lexer_config(),
    )
    .pop()
    .unwrap();
    let mut foreign = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(other),
        input.lexer_config(),
        &command,
    );
    bindings
        .source_bindings_ref()
        .stamp_original_tokens(&mut foreign);
    context.with_invocation_tokens(Some(&foreign), |context| {
        let child = context.inline_body_command_tokens(body, 0).unwrap();
        assert!(
            child
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_lexer_config_for_tokens(&child))
                .is_none()
        );
    });
}

#[test]
fn original_try_handler_argument_emission_keeps_the_child_source_scope() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let source = "try {set parent READY} on error {message options} {\nset result \"A\\u0000B\"\n}";
    let body = r#"set result "A\u0000B""#;
    let image = tcl_lexer::SourceImage::native(source.as_bytes());
    let map = tcl_lexer::SourceMap::from_image(&image);
    let parent_segments =
        tcl_compiler::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap();
    let parent = tcl_compiler::ir::CommandTokens::from_segmented(&map, config, &parent_segments[0]);
    let offset = u32::try_from(source.find(body).unwrap()).unwrap();
    let child_segments =
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(body, offset, config);
    assert_eq!(child_segments.len(), 1);
    let child = tcl_compiler::ir::CommandTokens::from_segmented(&map, config, &child_segments[0]);
    let mut context = CodegenCtx::new(true, &[], &registry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = dialect.native_source_string_protocol();
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image.clone());
    context.with_invocation_tokens(Some(&parent), |context| {
        context.with_invocation_tokens(Some(&child), |context| {
            context.emit_try_handler_command(body);
        });
        assert_eq!(
            context.invocation_tokens.as_deref().unwrap().words(),
            parent.words()
        );
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
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.op, Op::STORE_SCALAR1 | Op::STORE_SCALAR4))
    );
    assert!(context.invocation_tokens.is_none());

    // An enclosing source/body receipt cannot substitute for the child command.
    let mut missing = CodegenCtx::new(true, &[], &registry);
    missing.invocation_dialect = Some(dialect);
    missing.source_string_protocol = dialect.native_source_string_protocol();
    missing.ingress_lexer_config = Some(config);
    missing.set_source_image(image);
    missing.with_invocation_tokens(Some(&parent), |context| {
        context.with_invocation_tokens(None, |context| context.emit_try_handler_command(body));
    });
    assert!(
        missing
            .literals
            .entries()
            .iter()
            .all(|literal| literal.bytes() != b"A\xc0\x80B")
    );
    assert!(missing.invocation_tokens.is_none());
}

#[test]
fn inline_body_instructions_keep_the_inner_command_source_and_restore_the_parent() {
    let registry = CommandRegistry::build_default();
    let source = "catch {\nset value READY\nerror BODY\n}";
    let body = "\nset value READY\nerror BODY\n";
    for emitter in [InlineBodyEmitter::Catch, InlineBodyEmitter::TryHandler] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.set_source(source);
        let outer_span = Span::new(0, u32::try_from(source.len()).unwrap());
        ctx.set_command_source_span(outer_span);
        ctx.inline_body_source_base = Some(7);
        ctx.emit_segmented_inline_body(body, emitter);
        let failures = ctx
            .instructions
            .iter()
            .filter(|instruction| {
                matches!(instruction.op, Op::RETURN_IMM | Op::INVOKE_STK1)
                    && instruction.source_cmd_text.bytes() == b"error BODY"
            })
            .collect::<Vec<_>>();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].source_span, Some(Span::new(24, 34)));
        assert_eq!(failures[0].source_line, 3);
        assert!(ctx.instructions.iter().any(|instruction| {
            instruction.source_cmd_text.bytes() == b"set value READY"
                && instruction.source_span == Some(Span::new(8, 23))
                && instruction.source_line == 2
        }));
        let trailing = ctx.emit(Op::NOP, vec![]);
        assert_eq!(ctx.instructions[trailing].source_span, Some(outer_span));
        assert_eq!(
            ctx.instructions[trailing].source_cmd_text.bytes(),
            source.as_bytes()
        );
        assert_eq!(ctx.instructions[trailing].source_line, 1);
    }
}

#[test]
fn dictionary_iterations_close_the_same_indexed_search_on_both_exits() {
    let registry = CommandRegistry::build_default();
    for map in [false, true] {
        let mut ctx = CodegenCtx::new(true, &["d"], &registry);
        let emitted = if map {
            ctx.emit_dict_map("k v", "$d", "set result $v")
        } else {
            ctx.emit_dict_for("k v", "$d", "set result $v")
        };
        assert!(emitted);
        let begin = ctx
            .instructions
            .iter()
            .find(|instruction| instruction.op == Op::BEGIN_CATCH4)
            .unwrap();
        assert!(
            begin
                .catch_target
                .as_ref()
                .is_some_and(|label| ctx.label_positions.contains_key(label))
        );
        let slot = ctx
            .instructions
            .iter()
            .find(|instruction| instruction.op == Op::DICT_FIRST)
            .unwrap()
            .operands[0]
            .clone();
        assert_eq!(
            ctx.instructions
                .iter()
                .filter(|instruction| {
                    instruction.op == Op::UNSET_SCALAR
                        && instruction.operands == [Operand::Imm(0), slot.clone()]
                })
                .count(),
            2
        );
        assert_eq!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::END_CATCH)
                .count(),
            2
        );
    }
}

#[test]
fn dictionary_bodies_keep_abrupt_completions_in_their_original_activation() {
    let registry = CommandRegistry::build_default();
    for body in ["error BODY", "break", "continue", "return BODY"] {
        for map in [false, true] {
            let mut ctx = CodegenCtx::new(true, &["d"], &registry);
            assert!(if map {
                ctx.emit_dict_map("k v", "$d", body)
            } else {
                ctx.emit_dict_for("k v", "$d", body)
            });
            assert_eq!(ctx.inline_loop_regions.len(), 1);
            assert!(
                ctx.instructions
                    .iter()
                    .all(|instruction| instruction.op != Op::EVAL_STK)
            );
            assert!(
                ctx.instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::DICT_FIRST)
            );
            assert!(
                ctx.instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::RETURN_STK)
            );
        }
        for with in [false, true] {
            let mut ctx = CodegenCtx::new(true, &["d"], &registry);
            assert!(if with {
                ctx.emit_dict_with("d", body)
            } else {
                ctx.emit_dict_update(&["d".into(), "k".into(), "v".into(), body.into()])
            });
            assert!(
                ctx.instructions
                    .iter()
                    .all(|instruction| instruction.op != Op::EVAL_STK)
            );
            let protected = ctx
                .instructions
                .iter()
                .find(|i| i.op == Op::BEGIN_CATCH4)
                .unwrap();
            assert!(
                protected
                    .catch_target
                    .as_ref()
                    .is_some_and(|label| ctx.label_positions.contains_key(label))
            );
            assert_eq!(
                ctx.instructions
                    .iter()
                    .filter(|i| i.op
                        == if with {
                            Op::DICT_RECOMBINE_IMM
                        } else {
                            Op::DICT_UPDATE_END
                        })
                    .count(),
                2
            );
            assert!(ctx.instructions.iter().any(|i| i.op == Op::RETURN_STK));
        }
    }
}

/// The catch-body `expr` re-parse follows the compile's dialect, and its
/// operator set follows the target release: `catch {expr {2 ** 3}}`
/// compiled for 8.4 must not fold to a push of `8` and report success,
/// since the same source evaluated through `exprStk` is rejected as C Tcl
/// 8.4 rejects it.
#[test]
fn catch_body_expr_follows_the_compile_target_release() {
    let registry = CommandRegistry::build_default();

    let mut old = CodegenCtx::new(true, &[], &registry);
    old.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile());
    old.emit_catch_body("expr {2 ** 3}");
    let ops: Vec<Op> = old.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::EXPR_STK), "{ops:?}");
    assert!(old.literals.entries().iter().any(|l| l == "2 ** 3"));

    let mut modern = CodegenCtx::new(true, &[], &registry);
    modern.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile());
    modern.emit_catch_body("expr {2 ** 3}");
    let ops: Vec<Op> = modern.instructions.iter().map(|i| i.op).collect();
    assert!(!ops.contains(&Op::EXPR_STK), "{ops:?}");
    assert!(modern.literals.entries().iter().any(|l| l == "8"));
}

/// A catch body is a fresh script parse. Its `{*}` marker must reach the
/// expansion-aware command emitter instead of becoming a literal `*`
/// argument beside the value to expand.
#[test]
fn catch_body_preserves_argument_expansion() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["args"], &registry);
    ctx.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());

    ctx.emit_catch_body("Configure {*}$args");

    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::EXPAND_START), "{ops:?}");
    assert!(ops.contains(&Op::EXPAND_STKTOP), "{ops:?}");
    assert!(ops.contains(&Op::INVOKE_EXPANDED), "{ops:?}");
    assert!(!ops.contains(&Op::INVOKE_STK1), "{ops:?}");
    assert!(
        ctx.literals.entries().iter().all(|literal| literal != "*"),
        "the expansion marker must not enter argv: {:?}",
        ctx.literals.entries()
    );
}

/// `dict update` and `dict with` run as commands here, and the VM logs
/// their body frame only where the release would not have compiled them.
/// Measured with `::tcl::unsupported::disassemble`: 8.5.19 compiles
/// `dict update d k v {}` in a procedure body (`dictUpdateStart`) and
/// leaves `dict with` and a `::d` or `a(1)` update to the command; 8.6.18
/// and 9.0.4 also compile `dict with d {...}` and `dict with ::d {...}`
/// in one, but not `dict update` at the top level.
#[test]
fn the_release_compiles_dict_update_and_dict_with_in_a_procedure_body() {
    let registry = CommandRegistry::build_default();
    let compiled = |release: &str, is_proc: bool, args: &[(&str, bool)]| {
        let mut ctx = CodegenCtx::new(is_proc, &[], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment(release).analyser_profile());
        ctx.release_compiled("dict", args)
    };
    let update = [
        ("update", false),
        ("d", false),
        ("k", false),
        ("v", false),
        ("set v 2", true),
    ];
    let with = [("with", false), ("d", false), ("set a 1", true)];
    let in_procedure = Some(tcl_bytecode::ReleaseCompiled {
        script_word: None,
        frame_local_words: 0,
        in_procedure_only: false,
    });
    for release in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_eq!(compiled(release, true, &update), in_procedure, "{release}");
        assert_eq!(
            compiled(release, false, &update).map(|c| c.in_procedure_only),
            Some(true),
            "{release}"
        );
        for (variable, key) in [("::d", "v"), ("d", "a(1)")] {
            let args = [
                ("update", false),
                (variable, false),
                ("k", false),
                (key, false),
                ("set x 1", true),
            ];
            assert_eq!(compiled(release, true, &args), None, "{release} {args:?}");
        }
        assert_eq!(
            compiled(
                release,
                true,
                &[
                    ("update", false),
                    ("d", false),
                    ("k", false),
                    ("v", false),
                    ("$body", false)
                ]
            ),
            None,
            "{release}: a substituted body is left to the command"
        );
    }
    assert_eq!(compiled("tcl8.5", true, &with), None);
    for release in ["tcl8.6", "tcl9.0"] {
        assert_eq!(compiled(release, true, &with), in_procedure, "{release}");
        assert_eq!(
            compiled(
                release,
                true,
                &[("with", false), ("::d", false), ("set x 1", true)]
            ),
            in_procedure,
            "{release}"
        );
    }
}

/// C 8.6 compiles a substituted script word of `catch`, `try` and `for`
/// with the command where its compiler compiles the command, and leaves
/// it to the command otherwise. Measured on tclsh 8.6.18 with a canonical
/// list holding a literal index it compiles apart.
#[test]
fn the_release_compiles_a_substituted_catch_try_or_for_script_word() {
    let registry = CommandRegistry::build_default();
    let compiled = |is_proc: bool, command: &str, args: &[(&str, bool)]| {
        CodegenCtx::new(is_proc, &[], &registry).release_compiled(command, args)
    };
    let word = |is_proc: bool, command: &str, args: &[(&str, bool)]| {
        compiled(is_proc, command, args).and_then(|compiled| compiled.script_word)
    };
    let b = ("$b", false);
    let handler = [("on", false), ("ok", false), ("r", true), ("set r", true)];
    let with_handler = |body| [&[body][..], &handler[..]].concat();
    assert_eq!(word(true, "catch", &[b, ("r", false)]), Some(1));
    assert_eq!(
        word(true, "catch", &[b, ("r", false), ("o", false)]),
        Some(1)
    );
    assert_eq!(word(false, "catch", &[b]), Some(1));
    // Outside a procedure body the variable must already be a local of
    // the frame the script runs in.
    assert_eq!(
        compiled(false, "catch", &[b, ("r", false)]),
        Some(tcl_bytecode::ReleaseCompiled {
            script_word: Some(1),
            frame_local_words: 0b100,
            in_procedure_only: false,
        })
    );
    assert_eq!(
        compiled(true, "catch", &[b, ("r", false)]).map(|c| c.frame_local_words),
        Some(0)
    );
    assert_eq!(word(true, "catch", &[b, ("::r", false)]), None);
    assert_eq!(word(true, "catch", &[b, ("a(x)", false)]), None);
    assert_eq!(word(true, "catch", &[b, ("$r", false)]), None);
    assert_eq!(
        word(true, "catch", &[("set x 1", true), ("r", false)]),
        None
    );
    assert_eq!(word(false, "try", &[b]), Some(1));
    assert_eq!(
        word(false, "try", &[b, ("finally", false), ("", true)]),
        Some(1)
    );
    assert_eq!(word(true, "try", &with_handler(b)), Some(1));
    assert_eq!(word(false, "try", &with_handler(b)), None);
    assert_eq!(
        word(
            true,
            "try",
            &[b, ("trap", false), ("", true), ("", true), ("", true)]
        ),
        None
    );
    assert_eq!(
        word(
            true,
            "try",
            &[b, ("on", false), ("ok", false), ("r", true), ("$h", false)]
        ),
        None
    );
    assert_eq!(
        word(
            true,
            "try",
            &[b, ("on", false), ("ok", false), ("::r", true), ("", true)]
        ),
        None
    );
    assert_eq!(
        word(true, "try", &[b, ("finally", false), ("$f", false)]),
        None
    );
    assert_eq!(
        word(true, "for", &[b, ("0", true), ("", true), ("", true)]),
        Some(1)
    );
    assert_eq!(word(true, "for", &[b, ("0", true), ("", true), b]), None);
    assert_eq!(word(true, "lmap", &[("x", false), ("1", true), b]), None);
    assert_eq!(word(true, "eval", &[b]), None);

    // A compiled `try` binds its matched clause's variables whatever its
    // body: `try {set x 1} on ok {r} - on error {e} {…}` in a procedure.
    let falls_through = [
        ("set x 1", true),
        ("on", false),
        ("ok", false),
        ("r", true),
        ("-", false),
        ("on", false),
        ("error", false),
        ("e", true),
        ("list $r", true),
    ];
    let literal = Some(tcl_bytecode::ReleaseCompiled {
        script_word: None,
        frame_local_words: 0,
        in_procedure_only: false,
    });
    assert_eq!(compiled(true, "try", &falls_through), literal);
    assert_eq!(compiled(false, "try", &falls_through), None);
    let mut dynamic = falls_through;
    dynamic[8] = ("$h", false);
    assert_eq!(compiled(true, "try", &dynamic), None);
}

#[test]
fn catch_inline_emits_begin_end_catch() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_inline("set x 1", Some("result"), None);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
}

fn assert_captured_body_original_source(
    target: tcl_runtime_api::ProcedureCompileTarget<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::CommandRegistry,
) {
    let (_native_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
    let ir = tcl_compiler::lowering::lower_procedure_target_module_for_bytecode_with_options(
        target,
        registry,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
        Some(profile),
        false,
        Some(
            crate::compile_service::BytecodeCompileService::native_entry_options(
                &entry,
                Some(profile),
            ),
        ),
    );
    let statement = &ir.top_level.statements[0];
    let selection = statement.tokens().and_then(|tokens| {
        tokens.source_binding.as_ref().map(|binding| {
            (
                binding.native_compilation_admission_selection(),
                binding
                    .proved_execution_target()
                    .map(|target| target.command.as_str()),
            )
        })
    });
    assert!(
        matches!(statement, tcl_compiler::ir::Statement::Catch { .. }),
        "native catch lowering must survive a suspended child: {selection:?}"
    );
    if let tcl_compiler::ir::Statement::Catch { body, .. } = statement {
        assert_eq!(body.statements.len(), 2, "{body:?}");
        let words = body.statements[1]
            .tokens()
            .expect("original error words")
            .words();
        assert!(
            matches!(words.get(1), Some(tcl_compiler::ir::WordExpr::Template { parts, .. })
                if parts.iter().any(|part| matches!(part, tcl_compiler::ir::WordPart::Variable { source, .. }
                    if target.source.get(source.span.as_range()) == Some("$x")))),
            "error operand must retain its own quoted variable read: {words:?}"
        );
    }
    let prepared = tcl_compiler::cfg_builder::prepare_cfg_context_bundle(&ir, registry);
    let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry_and_context(
        &ir,
        false,
        registry,
        &prepared,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    assert!(
        cfg.top_level
            .blocks
            .values()
            .any(|block| block.name.starts_with("catch_body_")),
        "captured native body must reach its own CFG region: {:?}",
        cfg.top_level
            .blocks
            .values()
            .map(|block| (&block.name, &block.statements))
            .collect::<Vec<_>>()
    );
}

#[test]
fn inline_catch_ignores_a_trailing_unseen_call_marker() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let mut cfg = CfgFunction::new("::p", "entry_0");
    let body = cfg.intern_block("catch_body_1");
    cfg.blocks.insert(
        body,
        Block {
            name: "catch_body_1".into(),
            statements: vec![
                Statement::Call {
                    span: tcl_lexer::Span::new(0, 0),
                    command: "puts".into(),
                    canonical_command: None,
                    args: vec!["body".into()],
                    defs: vec![],
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                },
                Statement::unseen_call_marker(tcl_lexer::Span::new(0, 0)),
            ],
            terminator: None,
        },
    );

    ctx.emit_catch_region_inline(&cfg, "catch_body_1", None, None);

    assert_eq!(
        ctx.cmd_index, 2,
        "only the catch wrapper and body are source commands"
    );
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::END_CATCH),
        "the real final command still supplies the catch result",
    );
}

#[test]
fn catch_inline_multicommand_body_preserves_each_command_boundary() {
    use tcl_runtime_api::CompileService;
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let target = tcl_runtime_api::ProcedureCompileTarget {
        source: "catch {set x [yield a]; error \"boom-$x\"} result",
        namespace: "::",
        parameters: &[],
    };
    assert_captured_body_original_source(target, profile, registry);
    let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
    let (_native_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
    let module = service
        .compile_procedure_with_entry(
            target,
            profile,
            &entry,
            tcl_runtime_api::ProcedureDispatch::Optimised,
        )
        .unwrap();
    let ctx = &module.top_level_body;

    let ops: Vec<Op> = ctx
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert!(ops.contains(&Op::BEGIN_CATCH4), "{ops:?}");
    assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
    assert!(ops.contains(&Op::END_CATCH), "{ops:?}");
    assert!(
        ops.contains(&Op::YIELD)
            && ops
                .iter()
                .any(|op| matches!(op, Op::STORE_SCALAR1 | Op::INVOKE_STK1)),
        "the selected nested yield and enclosing setter must both execute: {ops:?}"
    );
    assert!(
        ops.iter().filter(|&&op| op == Op::START_CMD).count() >= 2,
        "the setter and subsequent error retain separate command boundaries: {:?}",
        ctx.instructions
            .iter()
            .map(|instruction| (
                instruction.op,
                &instruction.operands,
                &instruction.source_cmd_text,
                instruction.source_command_boundary,
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        ctx.literals.entries().iter().all(|literal| literal != ";"),
        "the command separator must not become an argument: {:?}",
        ctx.literals.entries()
    );
    assert!(
        ctx.instructions.iter().any(|instruction| {
            instruction.op == Op::START_CMD && instruction.source_cmd_text == "error \"boom-$x\""
        }),
        "nested replay text must retain the quoted last word: {:?}",
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::START_CMD)
            .map(|instruction| instruction
                .source_cmd_text
                .try_text()
                .expect("Unicode fixture source"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn catch_inline_incomplete_body_uses_the_shared_parse_failure() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_inline("set x {", Some("result"), None);

    let ops: Vec<Op> = ctx
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert!(ops.contains(&Op::SYNTAX), "{ops:?}");
    assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
    let begin = ctx
        .instructions
        .iter()
        .find(|i| i.op == Op::BEGIN_CATCH4)
        .unwrap();
    assert!(
        begin
            .catch_target
            .as_ref()
            .is_some_and(|label| ctx.label_positions.contains_key(label))
    );
}

#[test]
fn catch_inline_runs_complete_prefix_before_fatal_command_tail() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_inline("incr side; set x \"", Some("result"), None);

    let ops: Vec<Op> = ctx
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    let prefix_invoke = ops
        .iter()
        .position(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
        .expect("complete prefix command is emitted");
    let tail_failure = ops
        .iter()
        .position(|op| *op == Op::SYNTAX)
        .expect("malformed tail uses the original script parse failure");
    assert!(prefix_invoke < tail_failure, "{ops:?}");
    assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "missing \""),
        "the exact reached parser message is retained: {:?}",
        ctx.literals.entries()
    );
    assert!(
        ctx.literals
            .entries()
            .iter()
            .all(|literal| literal != "incr side; set x \""),
        "the complete prefix must not be replayed: {:?}",
        ctx.literals.entries()
    );
}

#[test]
fn nested_try_multicommand_phases_keep_live_ranges_and_boundaries() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.dialect =
        Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());
    let normal = ctx.fresh_label("normal");

    ctx.emit_try_on_error_inline(
        "try",
        &[
            ("set x ok; error boom-$x".into(), true),
            ("on".into(), false),
            ("error".into(), false),
            ("message".into(), false),
            (
                "set suffix handled; set message $message/$suffix".into(),
                true,
            ),
        ],
        &normal,
    );
    ctx.place_label(&normal);

    let begin_ranges: Vec<_> = ctx
        .instructions
        .iter()
        .filter(|instruction| instruction.op == Op::BEGIN_CATCH4)
        .collect();
    assert_eq!(begin_ranges.len(), 2);
    assert!(
        begin_ranges
            .iter()
            .all(|instruction| instruction.catch_target.is_some()),
        "both try phases need live VM exception targets: {begin_ranges:?}"
    );
    assert!(
        ctx.literals.entries().iter().all(|literal| literal != ";"),
        "phase separators must not become arguments: {:?}",
        ctx.literals.entries()
    );
    assert!(
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::POP)
            .count()
            >= 2,
        "each phase must discard its non-final command result"
    );
}

#[test]
fn nested_try_binds_both_typed_handler_variables() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    let normal = ctx.fresh_label("normal");

    ctx.emit_try_on_error_inline(
        "try",
        &[
            ("error boom".into(), true),
            ("on".into(), false),
            ("error".into(), false),
            ("message options".into(), true),
            ("list $message [dict get $options -code]".into(), true),
        ],
        &normal,
    );

    let lvt = ctx.lvt.entries();
    assert!(lvt.iter().any(|name| name == "message"), "{lvt:?}");
    assert!(lvt.iter().any(|name| name == "options"), "{lvt:?}");
    assert!(
        ctx.instructions
            .iter()
            .filter(|instruction| instruction.op == Op::STORE_SCALAR1)
            .count()
            >= 4,
        "result/options temps and both handler bindings must be stored"
    );
}

#[test]
fn nested_try_receiver_name_data_retains_generic_output_protocol() {
    let registry = CommandRegistry::build_default();
    for variable_list in [
        "{} options",
        "message {}",
        "{$literal} options",
        "{arr(key)} options",
    ] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let body = format!("try {{error boom}} on error {{{variable_list}}} {{set handled yes}}");
        assert!(ctx.emit_catch_inline(&body, Some("result"), None));
        let ops: Vec<_> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert_eq!(
            ops.iter().filter(|&&op| op == Op::BEGIN_CATCH4).count(),
            1,
            "only the outer catch can specialise: {variable_list}: {ops:?}"
        );
        assert!(ops.contains(&Op::INVOKE_STK1), "{variable_list}: {ops:?}");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == variable_list),
            "original receiver list must reach runtime: {:?}",
            ctx.literals.entries()
        );
        assert!(
            !ctx.lvt
                .entries()
                .iter()
                .any(|name| name == "options" || name == "message"),
            "declined handler outputs must not acquire local slots: {:?}",
            ctx.lvt.entries()
        );
    }
}

#[test]
fn nested_try_declines_unbraced_dynamic_phase_words() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);

    ctx.emit_catch_inline(
        "try $body on error {message options} $handler",
        Some("result"),
        None,
    );

    let ops: Vec<Op> = ctx
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert_eq!(
        ops.iter().filter(|&&op| op == Op::BEGIN_CATCH4).count(),
        1,
        "only the outer catch may specialise a dynamic try: {ops:?}"
    );
    assert!(
        ops.contains(&Op::INVOKE_STK1),
        "generic try invoke: {ops:?}"
    );
    assert!(
        !ctx.lvt.entries().iter().any(|name| name == "message"),
        "a declined handler must not allocate specialised bindings"
    );
}

#[test]
fn catch_inline_stores_result_var() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_inline("set x 1", Some("res"), None);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::STORE_SCALAR1));
}

#[test]
fn catch_inline_with_options_declines_without_selection() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    assert!(!ctx.emit_catch_inline("set x 1", Some("res"), Some("opts")));
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert_eq!(ctx.lvt.entries(), [] as [tcl_runtime_api::NameBytes; 0]);
}

#[test]
fn catch_return_simple() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_return(&[("hello".into(), false)]);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::RETURN_IMM));
}

#[test]
fn catch_return_with_code() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_return(&[
        ("-code".into(), false),
        ("error".into(), false),
        ("oops".into(), false),
    ]);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::RETURN_IMM));
}

#[test]
fn catch_error_emits_return_imm() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_error("error", "error oops", &[("oops".into(), false)]);
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::RETURN_IMM));
}

#[test]
fn detect_div_by_zero() {
    let node = ExprNode::Binary {
        op: BinOp::Div,
        left: Box::new(ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        }),
        right: Box::new(ExprNode::Literal {
            text: "0".into(),
            start: 4,
            end: 5,
        }),
    };
    let result = detect_const_expr_error(&node);
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "divide by zero");
}

#[test]
fn no_error_for_non_zero() {
    let node = ExprNode::Binary {
        op: BinOp::Div,
        left: Box::new(ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        }),
        right: Box::new(ExprNode::Literal {
            text: "2".into(),
            start: 4,
            end: 5,
        }),
    };
    assert!(detect_const_expr_error(&node).is_none());
}

// Registry drift: catch-body classifications.

/// The registry's `NEEDS_START_CMD` set must equal the set pinned here,
/// so a stamping change is a conscious decision rather than a silent
/// bytecode change.
#[test]
fn needs_start_cmd_trait_matches_previous_hardcoded_set() {
    let registry = CommandRegistry::build_default();
    let mut got = registry.commands_with_trait(Traits::NEEDS_START_CMD);
    got.sort_unstable();
    assert_eq!(got, ["break", "continue", "error", "expr", "return"]);
}

/// `break` in a catch body keeps its inline `break` opcode under a
/// `startCommand` wrap (hook + trait both registry-resolved).
#[test]
fn catch_body_wrapping_declines_an_unavailable_compilation_scope() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.resolution_namespace.key = None;
    ctx.emit_catch_body("break");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::INVOKE_STK1)
    );
    assert!(
        !ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::START_CMD)
    );
    assert!(
        !ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::BREAK)
    );
    assert!(ctx.command_binding_requirements.is_empty());
}

#[test]
fn catch_body_break_emits_break_op() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_body("break");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::BREAK), "expected BREAK, got {ops:?}");
    assert!(
        ops.contains(&Op::START_CMD),
        "break needs a startCommand wrap, got {ops:?}"
    );
}

/// `::error` names the global builtin, which C compiles as it compiles
/// `error` (`TclCompileErrorCmd`).
#[test]
fn catch_body_qualified_error_compiles_as_error() {
    let registry = CommandRegistry::build_default();
    let qualified = {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("::error boom");
        ctx.instructions.iter().map(|i| i.op).collect::<Vec<Op>>()
    };
    let bare = {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("error boom");
        ctx.instructions.iter().map(|i| i.op).collect::<Vec<Op>>()
    };
    assert_eq!(qualified, bare);
}

/// `throw` carries `CATCHABLE_THROW` like `error`, but its first
/// argument is the error-code *type*, not the message — it has no
/// `Error` inline hook, so a catch body must keep the generic
/// invoke for it (guarding the throw ≠ error distinction).
#[test]
fn catch_body_throw_stays_generic() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.emit_catch_body("throw {A B} boom");
    let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
    assert!(!ops.contains(&Op::RETURN_IMM), "no inline throw: {ops:?}");
    assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke: {ops:?}");
}
