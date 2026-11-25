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

//! Folding the operand of a `return`: the replacement word, constant variables, expressions and the bindings that withdraw the fold.

use super::*;

#[test]
fn retained_builtin_return_uses_the_checked_result_word() {
    let source = "proc p {} {return [llength {a b c}]}\nputs [p]";
    let optimisations = run_pass(source);
    assert!(
        optimisations.iter().any(|optimisation| {
            optimisation.code == DiagCode::O129
                && optimisation.replacement == "return 3"
                && !optimisation.hint_only
        }),
        "{optimisations:?}"
    );

    let shadowed =
        "proc llength {args} {return 99}\nproc p {} {return [llength {a b c}]}\nputs [p]";
    assert!(run_pass(shadowed).iter().all(|optimisation| {
        optimisation.code != DiagCode::O129 || optimisation.replacement != "return 3"
    }));
}

#[test]
fn return_literal_replacement_keeps_original_head_and_result_word_owner() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    for (source, expected) in [
        ("::return $result", "::return {Hello World}"),
        ("return [::expr {2 + 3}]", "return {Hello World}"),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let statement = module.top_level.statements.last().unwrap();
        assert!(matches!(
            statement,
            Statement::Return {
                value_word: Some(_),
                ..
            }
        ));
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        ctx.ir_module = Some(&module);
        let (span, replacement) =
            original_return_value_replacement(&ctx, statement, tokens, "Hello World").unwrap();
        assert_eq!(
            span,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap())
        );
        assert_eq!(replacement, expected);
        ctx.source = "return foreign";
        assert!(
            original_return_value_replacement(&ctx, statement, tokens, "Hello World").is_none()
        );
    }
}

#[test]
fn return_source_word_constructor_keeps_head_and_rejects_multiple_words() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    for (source, expected) in [
        ("::return $result", "::return [::expr {$x}]"),
        ("return [::expr {2 + 3}]", "return [::expr {$x}]"),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let statement = module.top_level.statements.last().unwrap();
        assert!(matches!(
            statement,
            Statement::Return {
                value_word: Some(_),
                ..
            }
        ));
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        ctx.ir_module = Some(&module);
        let (span, replacement) =
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]").unwrap();
        assert_eq!(
            span,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap())
        );
        assert_eq!(replacement, expected);
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]; unwanted")
                .is_none()
        );
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "{*}[::expr {$x}]").is_none()
        );
        ctx.source = "return foreign";
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]").is_none()
        );
    }
}

#[test]
fn return_literal_and_compound_names_do_not_borrow_scalar_constants() {
    for source in [
        "proc f {} {set x 42; return {$x}}",
        "proc f {} {set a 42; set a:b 99; return $a:b}",
        "proc f {} {set arr 42; return ${arr(k)}}",
    ] {
        let optimisations = run_pass(source);
        assert!(
            optimisations.iter().all(|optimisation| {
                optimisation.code != DiagCode::O100
                    || !optimisation.replacement.starts_with("return ")
            }),
            "{source}: {optimisations:?}"
        );
    }
}

#[test]
fn return_terminator_folds_constant_variable() {
    let opts = run_pass("proc ::f {} { set x 42; return $x }");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
        "expected O100 folding return $x to return 42, got {opts:?}",
    );
}

#[test]
fn return_expr_does_not_fold_stale_pre_loop_constant() {
    // `x` is overwritten by the loop, so its exit version at the return is
    // a non-Const phi (Overdefined). `::f` is pure but has no
    // argument-independent constant return, so `[::f]` reaches the
    // argument-sensitive fold's reading of a returned `expr`
    // (`interprocedural::exit_value`). That
    // fold must NOT leak the pre-loop `set x 0`: tclsh returns 3 (x ends
    // at 2), so folding `[::f]` to 1 would be a miscompile.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc ::f {} { set x 0; foreach v {1 2} { set x $v }; return [expr {$x + 1}] }\nputs [::f]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O103),
        "loop-overwritten var must not fold from a stale pre-loop constant, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn return_expr_after_unresolved_handler_does_not_fold_seeded_argument() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc p {x} {missing_command; return [expr {$x + 1}]}\nputs [p 5]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O103),
        "a handler can change the seeded argument: {:?}",
        ctx.optimisations
    );
}

#[test]
fn return_expr_folds_argument_sensitive_constant() {
    // Control: the argument-sensitive Path 3 still folds when the return
    // expr genuinely depends only on the constant call arguments — the
    // documented `[::add 2 4]` → 6 case (params bound at version 0).
    use tcl_registry::CommandRegistry;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(
        "proc ::add {a b} { return [expr {$a + $b}] }\nputs [::add 2 4]",
        &registry,
        false,
        profile,
    )
    .with_interprocedural(&registry, Some(profile));
    let mut ctx = crate::optimiser::manager::build_pass_context(&cu, &registry, Some(profile));
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "6"),
        "expected O103 folding [::add 2 4] to 6, got {:?}; {}",
        ctx.optimisations,
        proc_fold_context(&cu, "::add"),
    );
}

#[test]
fn shadowed_mathfunc_return_value_not_folded() {
    // FP guard: `return [expr {…}]` folding must also respect the
    // math-function shadow gate.
    use tcl_registry::CommandRegistry;
    let reg = CommandRegistry::build_default();
    let source =
        "proc ::tcl::mathfunc::abs {x} { return 999 }\nproc f {} { return [expr {abs(-5)}] }";
    let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&reg);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a shadowed math function in a return value: {:?}",
        ctx.optimisations,
    );
}

#[test]
fn renamed_expr_return_value_not_folded() {
    // FP guard: `return [expr {…}]` folding (try_fold_return_terminator)
    // must also respect the trust gate.
    use tcl_registry::CommandRegistry;
    let reg = CommandRegistry::build_default();
    let source = "rename expr real_expr\nproc f {} { return [expr {1 + 2}] }";
    let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a renamed expr in a return value: {:?}",
        ctx.optimisations,
    );
}
