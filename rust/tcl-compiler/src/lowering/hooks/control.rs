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

//! Lower the `expr` and `return` commands to typed IR statements.
//!
//! `expr <one-word>` lowers to [`Statement::ExprEval`] when the
//! single argument is a single *literal* token (typically a braced
//! expression). Multi-arg forms, a lone substitution (`expr $e`)
//! and `{*}` expansion fall through to a generic
//! [`Statement::Call`] so the runtime sees the original argument
//! list.
//!
//! `return ?value?` lowers to [`Statement::Return`] when the call
//! is the simple `return` or `return value` shape. A lone `-code` is also
//! result data; option/value pairs and other multiword forms emit a
//! [`Statement::Barrier`] so downstream passes do not assume a
//! particular value slot, and `{*}` expansion does the same.
//! When the value is a `[expr {…}]` command substitution (or one
//! of the configured expr aliases such as `=`), the parsed
//! expression is attached to the [`Statement::Return`] so codegen
//! can emit the value directly.

use crate::alias::CommandAliasMap;
use crate::expr_parser::parse_expr_for_profile;
use crate::ir::Statement;
use crate::lowering_hooks::{
    ArgTokenKind, LoweringCommand, extract_proved_expr_arg, has_expansion,
};
use tcl_runtime_api::CommandBindingIdentity;

/// Lower `expr` to [`Statement::ExprEval`] when the call is the
/// single-arg form, or `None` to fall back to [`Statement::Call`].
#[must_use]
pub fn try_lower_expr(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if has_expansion(cmd) {
        return None;
    }
    if cmd.args.len() != 1 {
        return None;
    }
    if cmd.single_token_word.len() < 2 || !cmd.single_token_word[1] {
        return None;
    }
    // Only a *literal* single-token arg — a braced `{…}` or a bare word — is the
    // expression itself. A lone substitution (`expr $e`, `expr [f]`) must be
    // substituted *then* re-evaluated as an expression (`expr $e` with
    // `e == "1+2"` is `3`, not `"1+2"`); inlining `parse_expr("$e")` would treat
    // the variable's *value* as the final result. Defer those to the runtime
    // `expr`, which does the second evaluation. (A braced `{$e}` correctly inlines:
    // there the expression really is the operand `$e`.)
    if !matches!(
        cmd.arg_kinds.first(),
        Some(ArgTokenKind::Str | ArgTokenKind::Esc)
    ) {
        return None;
    }
    let expr = parse_expr_for_profile(&cmd.args[0], cmd.dialect);
    // Anchor the expression text absolutely when the arg word's content is
    // a verbatim source slice (see `word_content_base`) so consumers can map
    // expression-AST leaf offsets to source operand spans.
    let expr_base = cmd.tokens.as_ref().and_then(|t| {
        crate::lowering_hooks::word_content_base(
            *t.argv.get(1)?,
            t.single_token_word.get(1).copied().unwrap_or(false),
            &cmd.args[0],
        )
    });
    Some(Statement::ExprEval {
        span: cmd.span,
        // `try_lower_hook` replaces the provisional target with the
        // registry-resolved canonical implementation. Keeping the source
        // spelling here makes the hook independently well-formed for tests
        // and external callers.
        command_binding: CommandBindingIdentity::in_rooted_namespace(
            cmd.resolution_namespace,
            cmd.name,
            cmd.name,
        )
        .with_namespace_context(
            cmd.tokens
                .as_ref()
                .and_then(crate::registry_invocation::compiled_namespace_context),
        ),
        expr,
        expr_base,
    })
}

/// Lower `return` to [`Statement::Return`], or to
/// [`Statement::Barrier`] when the command form is not the simple
/// `return ?value?` shape (option-bearing or `{*}`-expanded).
#[must_use]
pub fn try_lower_return(
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
    registry: &tcl_registry::CommandRegistry,
    context: Option<&tcl_registry::model::ResolvedContext>,
) -> Statement {
    if has_expansion(cmd) {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with expansion".into(),
            command: cmd.name.into(),
            canonical_command: None,
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }
    if cmd.args.len() > 1 {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with options".into(),
            command: cmd.name.into(),
            canonical_command: None,
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }

    let value = cmd.args.first().cloned();
    let value_word = (cmd.args.len() == 1)
        .then(|| {
            cmd.tokens
                .as_ref()
                .and_then(|tokens| tokens.words().get(1))
                .cloned()
        })
        .flatten();
    let ReturnValueLowering {
        expr,
        expr_base,
        command_binding,
        braced,
    } = lower_return_value(cmd, aliases, registry, context);

    Statement::Return {
        span: cmd.span,
        tokens: cmd.tokens.clone(),
        value,
        value_word,
        expr,
        expr_base,
        command_binding,
        braced,
    }
}

#[derive(Default)]
struct ReturnValueLowering {
    expr: Option<tcl_syntax::expr::ExprNode>,
    expr_base: Option<u32>,
    command_binding: Option<CommandBindingIdentity>,
    braced: bool,
}

fn lower_return_value(
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
    registry: &tcl_registry::CommandRegistry,
    context: Option<&tcl_registry::model::ResolvedContext>,
) -> ReturnValueLowering {
    let mut expr = None;
    let mut expr_base = None;
    let mut command_binding = None;
    let mut braced = false;

    if !cmd.args.is_empty()
        && !cmd.arg_kinds.is_empty()
        && cmd.single_token_word.len() >= 2
        && cmd.single_token_word[1]
    {
        match cmd.arg_kinds[0] {
            ArgTokenKind::Str => braced = true,
            ArgTokenKind::Cmd => {
                let inner = cmd.args[0]
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .unwrap_or(&cmd.args[0]);
                if let Some((expr_cmd, canonical_cmd, expr_arg, relative_base)) =
                    extract_proved_expr_arg(
                        (cmd.tokens.as_ref(), 1),
                        inner,
                        aliases,
                        cmd.resolution_namespace,
                        registry,
                        context,
                        cmd.lexer_config,
                    )
                {
                    expr = Some(parse_expr_for_profile(&expr_arg, cmd.dialect));
                    expr_base = cmd.tokens.as_ref().and_then(|tokens| {
                        let base = crate::lowering_hooks::word_content_base(
                            *tokens.argv.get(1)?,
                            tokens.single_token_word.get(1).copied().unwrap_or(false),
                            inner,
                        )?;
                        base.checked_add(relative_base?)
                    });
                    command_binding = Some(
                        CommandBindingIdentity::in_rooted_namespace(
                            cmd.resolution_namespace,
                            expr_cmd,
                            canonical_cmd,
                        )
                        .with_namespace_context(
                            cmd.tokens
                                .as_ref()
                                .and_then(crate::registry_invocation::compiled_namespace_context),
                        ),
                    );
                }
            }
            ArgTokenKind::ExprSugar => {
                // JimTcl `$(…)`: `return $($a*2)` returns the expression's
                // value, as `return [expr {$a*2}]` does.
                if let Some(body) = cmd.args[0]
                    .strip_prefix("$(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    expr = Some(parse_expr_for_profile(body, cmd.dialect));
                    expr_base = cmd.tokens.as_ref().and_then(|tokens| {
                        let span = *tokens.argv.get(1)?;
                        let raw_len = span.end().checked_sub(span.start())?;
                        let body_len = u32::try_from(body.len()).ok()?;
                        (raw_len == body_len.checked_add(2)?).then(|| span.start() + 2)
                    });
                }
            }
            _ => {}
        }
    }

    ReturnValueLowering {
        expr,
        expr_base,
        command_binding,
        braced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn native_return_module(source: &str) -> crate::ir::Module {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let mut lowerer = crate::lowering::Lowerer::new(registry).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        });
        crate::lowering::lower_to_ir_with(lowerer, source)
    }

    // expr — end-to-end via lower_to_ir

    #[test]
    fn nested_return_expression_retains_original_parser_source_base() {
        let source = "proc p {} {return [expr {abs(-3)}]}; p";
        let module = native_return_module(source);
        let statement = module.procedures["::p"].body.statements.first().unwrap();
        let Statement::Return {
            expr: Some(_),
            expr_base: Some(base),
            ..
        } = statement
        else {
            panic!(
                "missing exact returned expression: {:?}",
                std::mem::discriminant(statement)
            );
        };
        assert_eq!(
            source.get(*base as usize..*base as usize + 7),
            Some("abs(-3)")
        );
        assert!(
            module.procedures["::p"]
                .body
                .implicit_math_invocations
                .iter()
                .any(|proof| proof.site == *base && proof.function == "abs")
        );
    }

    #[test]
    fn expr_braced_lowers_to_expr_eval() {
        let m = lower_to_ir("expr {$x + 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::ExprEval { command_binding, .. }
                if command_binding
                    == &CommandBindingIdentity::new("expr", "expr")
        ));
    }

    #[test]
    fn expr_multi_arg_falls_back_to_call() {
        let m = lower_to_ir("expr $x + 1", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "expr"
        ));
    }

    #[test]
    fn expr_zero_args_falls_back_to_call() {
        let m = lower_to_ir("expr", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "expr"
        ));
    }

    #[test]
    fn expr_with_expansion_falls_back_to_call() {
        let m = lower_to_ir("expr {*}$args", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "expr"
        ));
    }

    // return — end-to-end via lower_to_ir

    #[test]
    fn return_no_args_lowers_to_return() {
        let m = lower_to_ir("return", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Return { value: None, .. }
        ));
    }

    #[test]
    fn return_value_lowers_to_return() {
        let m = lower_to_ir("return 42", &reg());
        match &m.top_level.statements[0] {
            Statement::Return {
                value,
                expr,
                braced,
                ..
            } => {
                assert_eq!(value.as_deref(), Some("42"));
                assert!(expr.is_none());
                assert!(!braced);
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn return_braced_value_marks_braced() {
        let m = lower_to_ir("return {hello world}", &reg());
        match &m.top_level.statements[0] {
            Statement::Return { value, braced, .. } => {
                assert_eq!(value.as_deref(), Some("hello world"));
                assert!(*braced);
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn return_with_expr_substitution_attaches_expr() {
        // `return [expr {$x + 1}]` should attach the parsed expression
        // so codegen can emit the value directly without re-eval.
        let m = native_return_module("proc p {x} {return [expr {$x + 1}]}; p 1");
        match &m.procedures["::p"].body.statements[0] {
            Statement::Return { expr, .. } => {
                assert!(expr.is_some(), "expected expr attached, got None");
            }
            other => panic!("expected Return, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn unentered_or_failed_return_value_does_not_gain_executable_expression_metadata() {
        for source in [
            "proc p {x} {return [expr {$x + 1}]}",
            "proc p {} {return [expr {$missing + 1}]}; p",
            "proc p {x} {return [expr {$x + 1}]}; mystery; p 1",
        ] {
            let module = native_return_module(source);
            let body = &module.procedures["::p"].body;
            assert!(body.implicit_math_invocations.is_empty());
            assert!(body.expression_preparations.is_empty());
            assert!(body.statements.iter().all(|statement| {
                statement
                    .tokens()
                    .and_then(|tokens| tokens.source_binding.as_ref())
                    .is_none_or(|binding| binding.proved_execution_target().is_none())
            }));
        }
    }

    #[test]
    fn sole_return_option_spelling_is_original_result_data() {
        let module = lower_to_ir("return -code", &reg());
        assert!(
            matches!(&module.top_level.statements[0], Statement::Return { value: Some(value), .. } if value == "-code")
        );
    }

    #[test]
    fn return_with_options_emits_barrier() {
        let m = lower_to_ir("return -code error oops", &reg());
        match &m.top_level.statements[0] {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(reason, "return with options");
                assert_eq!(command, "return");
            }
            other => panic!("expected Barrier, got {other:?}"),
        }
    }

    #[test]
    fn return_with_expansion_emits_barrier() {
        // Native evaluation reads args before dispatch. An absent operand
        // reaches no Return handler; successful expansion still lacks the
        // original one-to-one slots required by structured Return lowering.
        for (source, entered) in [
            ("return {*}$args", false),
            ("set args {42}; return {*}$args", true),
        ] {
            let module = native_return_module(source);
            let statement = module.top_level.statements.last().unwrap();
            let Statement::Call {
                command,
                tokens: Some(tokens),
                ..
            } = statement
            else {
                panic!(
                    "expanded Return must retain generic original argv: {:?}",
                    std::mem::discriminant(statement)
                );
            };
            assert_eq!(command, "return");
            assert!(matches!(
                tokens.words().get(1),
                Some(crate::ir::WordExpr::Expand { .. })
            ));
            assert_eq!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .proved_execution_target()
                    .is_some(),
                entered
            );
        }
    }

    #[test]
    fn dispatcher_routes_return_through_try_lower_hook() {
        // The shared dispatcher in ``lowering_hooks::try_lower_hook``
        // must route ``return`` and ``expr`` to this module.
        let m = lower_to_ir("proc f {} { return 1 }\nexpr {1}", &reg());
        assert!(m.procedures.contains_key("::f"));
        let body = &m.procedures["::f"].body.statements;
        assert!(body.iter().any(|s| matches!(s, Statement::Return { .. })));
        assert!(
            m.top_level
                .statements
                .iter()
                .any(|s| matches!(s, Statement::ExprEval { .. }))
        );
    }

    // Unit-level coverage of the hook entry points

    fn make_cmd<'a>(
        name: &'a str,
        args: &'a [String],
        single: &'a [bool],
        kinds: &'a [ArgTokenKind],
        expand: Option<&'a [bool]>,
    ) -> LoweringCommand<'a> {
        LoweringCommand {
            span: Span::new(0, 8),
            name,
            resolution_namespace: "::",
            args,
            single_token_word: single,
            expand_word: expand,
            tokens: None,
            arg_kinds: kinds,
            dialect: None,
            lexer_config: tcl_lexer::LexerConfig::default(),
        }
    }

    #[test]
    fn unit_expr_single_braced_arg_returns_some() {
        let args = vec!["{1 + 2}".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Str];
        let cmd = make_cmd("expr", &args, &single, &kinds, None);
        assert!(matches!(
            try_lower_expr(&cmd),
            Some(Statement::ExprEval { .. })
        ));
    }

    #[test]
    fn unit_expr_multiword_arg_returns_none() {
        let args = vec!["$a".to_string(), "+".to_string(), "$b".to_string()];
        let single = vec![true, true, true, true];
        let kinds = vec![ArgTokenKind::Var, ArgTokenKind::Esc, ArgTokenKind::Var];
        let cmd = make_cmd("expr", &args, &single, &kinds, None);
        assert!(try_lower_expr(&cmd).is_none());
    }

    #[test]
    fn unit_expr_expansion_returns_none() {
        let args = vec!["$args".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let expand = vec![false, true];
        let cmd = make_cmd("expr", &args, &single, &kinds, Some(&expand));
        assert!(try_lower_expr(&cmd).is_none());
    }

    #[test]
    fn unit_return_simple_value() {
        let args = vec!["$result".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let aliases = CommandAliasMap::new();
        let registry = reg();
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases, &registry, None) {
            Statement::Return {
                value,
                expr,
                braced,
                ..
            } => {
                assert_eq!(value.as_deref(), Some("$result"));
                assert!(expr.is_none());
                assert!(!braced);
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn unit_return_no_args() {
        let args: Vec<String> = vec![];
        let single = vec![true];
        let kinds: Vec<ArgTokenKind> = vec![];
        let aliases = CommandAliasMap::new();
        let registry = reg();
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases, &registry, None) {
            Statement::Return { value, .. } => assert!(value.is_none()),
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn unit_return_extra_values_preserve_runtime_arity_error() {
        let args = vec!["first".to_owned(), "second".to_owned()];
        let cmd = make_cmd(
            "return",
            &args,
            &[true, true, true],
            &[ArgTokenKind::Esc, ArgTokenKind::Esc],
            None,
        );
        assert!(matches!(
            try_lower_return(&cmd, &CommandAliasMap::new(), &reg(), None),
            Statement::Barrier { args: retained, .. } if retained == args
        ));
    }

    #[test]
    fn unit_return_dash_first_arg_is_barrier() {
        let args = vec!["-code".to_string(), "error".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc];
        let aliases = CommandAliasMap::new();
        let registry = reg();
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        assert!(matches!(
            try_lower_return(&cmd, &aliases, &registry, None),
            Statement::Barrier { .. }
        ));
    }

    #[test]
    fn unit_return_expansion_is_barrier() {
        let args = vec!["$args".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let expand = vec![false, true];
        let aliases = CommandAliasMap::new();
        let registry = reg();
        let cmd = make_cmd("return", &args, &single, &kinds, Some(&expand));
        match try_lower_return(&cmd, &aliases, &registry, None) {
            Statement::Barrier { reason, .. } => {
                assert_eq!(reason, "return with expansion");
            }
            other => panic!("expected Barrier, got {other:?}"),
        }
    }

    #[test]
    fn unit_return_braced_value_marks_braced() {
        let args = vec!["hello".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Str];
        let aliases = CommandAliasMap::new();
        let registry = reg();
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases, &registry, None) {
            Statement::Return { braced, .. } => assert!(braced),
            other => panic!("expected Return, got {other:?}"),
        }
    }
}
