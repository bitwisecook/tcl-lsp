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
use tcl_compiler::cfg::{Block, Function as CfgFunction, InlineBodyErrorSite, Terminator};
use tcl_compiler::ir::Statement;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

fn sp() -> Span {
    Span::new(0, 0)
}

fn trivial_cfg() -> CfgFunction {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    cfg
}

#[test]
fn generate_empty_toplevel_terminates() {
    let cfg = trivial_cfg();
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let asm = generate(&mut ctx, &cfg);
    assert_eq!(asm.name, "::top");
    assert_ne!(asm.instructions, [] as [tcl_bytecode::Instruction; 0]);
    // Top-level scripts terminate with RETURN_IMM or DONE
    let last = asm.instructions.last().unwrap().op;
    assert!(
        matches!(last, Op::DONE | Op::RETURN_IMM),
        "expected DONE or RETURN_IMM, got {last:?}"
    );
}

#[test]
fn original_empty_script_uses_the_selected_native_literal_producer() {
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_string::NativeStringProtocol;
    for (engine, version) in [
        ("tcl8.4", TclVersion::V8_4),
        ("tcl8.5", TclVersion::V8_5),
        ("tcl8.6", TclVersion::V8_6),
        ("tcl9.0", TclVersion::V9_0),
        ("tcl9.1", TclVersion::V9_1),
    ] {
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for source in ["", " \t\n", "# comment\n"] {
            let module = tcl_compiler::lowering::lower_to_ir_with_config(source, registry, config);
            let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry(
                &module, false, registry,
            );
            for is_proc in [false, true] {
                let mut ctx = CodegenCtx::new(is_proc, &[], registry);
                ctx.source_string_protocol = Some(NativeStringProtocol::C(version));
                let asm = generate(&mut ctx, &cfg.top_level);
                assert_eq!(asm.instructions.len(), 2, "{engine} {source:?}");
                assert_eq!(asm.instructions[0].op, Op::PUSH1);
                assert_eq!(asm.instructions[1].op, Op::DONE);
                assert_eq!(asm.literals.entries().len(), 1);
                assert_eq!(
                    matches!(
                        asm.literals.entries()[0].allocation(),
                        tcl_bytecode::NativeLiteralAllocation::Unshared
                    ),
                    version == TclVersion::V8_5,
                    "{engine} {source:?} procedure={is_proc}",
                );
            }
        }
    }
}

#[test]
fn analysis_completion_graph_is_rejected_before_any_emission() {
    let mut cfg = trivial_cfg();
    cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Complete {
        route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::CompletionCode::Error,
        ),
        span: Some(sp()),
    });
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| generate(&mut ctx, &cfg)));
    assert!(result.is_err());
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
}

#[test]
fn inline_body_error_region_uses_semantic_context_not_command_text() {
    let source = "renamed-body-command {error boom}";
    let span = Span::new(0, u32::try_from(source.len()).unwrap());
    let mut cfg = trivial_cfg();
    cfg.inline_body_error_sites.push(InlineBodyErrorSite {
        span,
        context: tcl_compiler::cfg::BodyErrorContext::Registry(
            InlineBodyErrorContext::SameFrameScriptEvaluation,
        ),
        region: span,
        compiled: tcl_compiler::cfg::CompiledAt::NEVER,
    });
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.set_source(source);

    let asm = generate(&mut ctx, &cfg);

    let [region] = asm.error_regions.as_slice() else {
        panic!("semantic inline-body context should emit exactly one error region");
    };
    assert_eq!(
        region.frame,
        tcl_bytecode::ErrorRegionFrame::Body("eval".to_owned())
    );
    assert_eq!(region.cmd_text, source);

    let mut source_less = CodegenCtx::new(false, &[], &registry);
    let source_less_asm = generate(&mut source_less, &cfg);
    assert!(
        source_less_asm.error_regions.is_empty(),
        "a source-less context retains the previous no-region behaviour",
    );
}

#[test]
fn generate_stamps_statement_source_span() {
    // A statement with a non-trivial span: its lowered instructions
    // (push / store) must carry that span; the synthetic trailing
    // DONE carries none.
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let stmt_span = Span::new(4, 10);
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: stmt_span,
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        });
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let asm = generate(&mut ctx, &cfg);

    // The store-into-x instruction came from the AssignConst statement.
    let store = asm
        .instructions
        .iter()
        .find(|i| i.op == Op::STORE_STK)
        .expect("store emitted");
    assert_eq!(
        store.source_span,
        Some(stmt_span),
        "statement-driven op carries its source span"
    );
    // The synthetic terminator (Return with no span) leaves none.
    let done = asm
        .instructions
        .iter()
        .rev()
        .find(|i| matches!(i.op, Op::DONE | Op::RETURN_IMM))
        .expect("terminator emitted");
    assert_eq!(done.source_span, None, "synthetic terminator has no span");
}

#[test]
fn generate_simple_set() {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        });
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let asm = generate(&mut ctx, &cfg);
    // Should contain push and store (tail pop is folded away)
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    assert!(ops.contains(&Op::PUSH1));
    assert!(ops.contains(&Op::STORE_STK));
}

#[test]
fn generate_proc_empty_body() {
    let cfg = CfgFunction::new("::foo", "entry_0");
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    // Give the entry block a return terminator
    let mut cfg = cfg;
    let entry = cfg.entry;
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let asm = generate(&mut ctx, &cfg);
    // Proc with empty body should push "" and emit done
    assert_eq!(asm.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn generate_if_diamond() {
    use tcl_compiler::expr_ast::ExprNode;
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let then = cfg.intern_block("if_then_1");
    cfg.blocks.insert(then, Block::new("if_then_1"));
    let els = cfg.intern_block("if_else_1");
    cfg.blocks.insert(els, Block::new("if_else_1"));
    let end = cfg.intern_block("if_end_1");
    cfg.blocks.insert(end, Block::new("if_end_1"));

    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
        condition: ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        },
        true_target: then,
        false_target: els,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "1".into(),
            name_braced: false,
            value_span: None,
        });
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks
        .get_mut(&els)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "r".into(),
            value: "2".into(),
            name_braced: false,
            value_span: None,
        });
    cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
        target: end,
        span: None,
    });
    cfg.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let registry = CommandRegistry::build_default();

    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let asm = generate(&mut ctx, &cfg);
    let ops: Vec<Op> = asm.instructions.iter().map(|i| i.op).collect();
    // Should include a conditional jump (maybe shrunk to 1-byte)
    assert!(
        ops.contains(&Op::JUMP_FALSE4) || ops.contains(&Op::JUMP_FALSE1),
        "expected conditional jump, got {ops:?}"
    );
}
