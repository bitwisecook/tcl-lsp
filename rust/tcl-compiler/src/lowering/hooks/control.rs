//! Lower the `expr` and `return` commands to typed IR statements.
//!
//! Mirrors `core/compiler/lowering_hooks/_control.py`.
//!
//! `expr <one-word>` lowers to [`Statement::ExprEval`] when the
//! single argument is a single token (typically a braced
//! expression). Multi-arg forms and `{*}` expansion fall through
//! to a generic [`Statement::Call`] so the runtime sees the
//! original argument list.
//!
//! `return ?value?` lowers to [`Statement::Return`] when the call
//! is the simple `return` or `return value` shape. `return -code`
//! / `return -level` and other option-bearing forms emit a
//! [`Statement::Barrier`] so downstream passes do not assume a
//! particular value slot, and `{*}` expansion does the same.
//! When the value is a `[expr {…}]` command substitution (or one
//! of the configured expr aliases such as `=`), the parsed
//! expression is attached to the [`Statement::Return`] so codegen
//! can emit the value directly.

use crate::alias::{expr_alias_names, CommandAliasMap};
use crate::expr_parser::parse_expr;
use crate::ir::Statement;
use crate::lowering_hooks::{
    extract_single_expr_arg, has_expansion, ArgTokenKind, LoweringCommand,
};

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
    let expr = parse_expr(&cmd.args[0], None);
    Some(Statement::ExprEval {
        span: cmd.span,
        expr,
    })
}

/// Lower `return` to [`Statement::Return`], or to
/// [`Statement::Barrier`] when the command form is not the simple
/// `return ?value?` shape (option-bearing or `{*}`-expanded).
#[must_use]
pub fn try_lower_return(cmd: &LoweringCommand<'_>, aliases: &CommandAliasMap) -> Statement {
    if has_expansion(cmd) {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with expansion".into(),
            command: cmd.name.into(),
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }
    if !cmd.args.is_empty() && cmd.args[0].starts_with('-') {
        return Statement::Barrier {
            span: cmd.span,
            reason: "return with options".into(),
            command: cmd.name.into(),
            args: cmd.args.to_vec(),
            tokens: cmd.tokens.clone(),
        };
    }

    let value = cmd.args.first().cloned();
    let mut expr = None;
    let mut braced = false;

    if value.is_some()
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
                let alias_names = expr_alias_names(aliases);
                if let Some(expr_arg) = extract_single_expr_arg(inner, &alias_names) {
                    expr = Some(parse_expr(&expr_arg, None));
                }
            }
            _ => {}
        }
    }

    Statement::Return {
        span: cmd.span,
        value,
        expr,
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

    // ── expr — end-to-end via lower_to_ir ─────────────────────────

    #[test]
    fn expr_braced_lowers_to_expr_eval() {
        // Mirrors `tests/test_ir_lowering.py::test_standalone_expr`.
        let m = lower_to_ir("expr {$x + 1}", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::ExprEval { .. }
        ));
    }

    #[test]
    fn expr_multi_arg_falls_back_to_call() {
        // Mirrors `tests/test_ir_lowering.py::test_standalone_expr_multiarg_fallback`.
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

    // ── return — end-to-end via lower_to_ir ───────────────────────

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
        let m = lower_to_ir("return [expr {$x + 1}]", &reg());
        match &m.top_level.statements[0] {
            Statement::Return { expr, .. } => {
                assert!(expr.is_some(), "expected expr attached, got None");
            }
            other => panic!("expected Return, got {other:?}"),
        }
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
        let m = lower_to_ir("return {*}$args", &reg());
        match &m.top_level.statements[0] {
            Statement::Barrier {
                reason, command, ..
            } => {
                assert_eq!(reason, "return with expansion");
                assert_eq!(command, "return");
            }
            other => panic!("expected Barrier, got {other:?}"),
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
        assert!(m
            .top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::ExprEval { .. })));
    }

    // ── Unit-level coverage of the hook entry points ──────────────

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
            args,
            single_token_word: single,
            expand_word: expand,
            tokens: None,
            arg_kinds: kinds,
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
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases) {
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
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases) {
            Statement::Return { value, .. } => assert!(value.is_none()),
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn unit_return_dash_first_arg_is_barrier() {
        let args = vec!["-code".to_string(), "error".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc];
        let aliases = CommandAliasMap::new();
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        assert!(matches!(
            try_lower_return(&cmd, &aliases),
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
        let cmd = make_cmd("return", &args, &single, &kinds, Some(&expand));
        match try_lower_return(&cmd, &aliases) {
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
        let cmd = make_cmd("return", &args, &single, &kinds, None);
        match try_lower_return(&cmd, &aliases) {
            Statement::Return { braced, .. } => assert!(braced),
            other => panic!("expected Return, got {other:?}"),
        }
    }
}
