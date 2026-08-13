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

//! Lower the `incr` command to a typed IR statement.
//!
//! `incr name ?amount?` increments `name` by `amount` (default 1)
//! and returns the new value. Specialises to [`Statement::Incr`]
//! when:
//!
//! 1. There is no `{*}` argument expansion (otherwise the runtime
//!    arg count is unknown and the specialised path is unsafe).
//! 2. There is at least one positional argument (the variable name).
//! 3. There are at most two positional arguments (name + optional
//!    amount).
//!
//! Anything else falls back to a generic [`Statement::Call`] so the
//! runtime sees the original argument list and arity checks fire on
//! the unmodified word vector.

use crate::ir::Statement;
use crate::lowering_hooks::{ArgTokenKind, LoweringCommand, has_expansion, make_call};

/// Lower `incr` to [`Statement::Incr`] or fall back to
/// [`Statement::Call`] when the call shape is not the
/// specialise-able `incr name ?amount?` form.
#[must_use]
pub fn try_lower_incr(cmd: &LoweringCommand<'_>) -> Statement {
    if has_expansion(cmd) || cmd.args.is_empty() || cmd.args.len() > 2 {
        return make_call(cmd);
    }
    // A brace-string name (`incr {a($x)} 3`) suppresses substitution of an
    // array-element key — thread it through like `set` does.
    let name_braced = matches!(cmd.arg_kinds.first(), Some(ArgTokenKind::Str))
        && cmd.single_token_word.get(1).copied().unwrap_or(false);
    Statement::Incr {
        span: cmd.span,
        name: cmd.args[0].clone(),
        name_braced,
        amount: cmd.args.get(1).cloned(),
        // Conservatively `false`: downstream passes treat ``incr``
        // as if it reads the variable first.
        safe_on_uninit: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use crate::lowering_hooks::ArgTokenKind;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn incr_single_arg_lowers_to_incr() {
        let m = lower_to_ir("incr i", &reg());
        assert_eq!(m.top_level.statements.len(), 1);
        match &m.top_level.statements[0] {
            Statement::Incr { name, amount, .. } => {
                assert_eq!(name, "i");
                assert!(amount.is_none());
            }
            other => panic!("expected Incr, got {other:?}"),
        }
    }

    #[test]
    fn incr_with_amount_lowers_to_incr() {
        let m = lower_to_ir("incr i 2", &reg());
        match &m.top_level.statements[0] {
            Statement::Incr { name, amount, .. } => {
                assert_eq!(name, "i");
                assert_eq!(amount.as_deref(), Some("2"));
            }
            other => panic!("expected Incr, got {other:?}"),
        }
    }

    #[test]
    fn incr_negative_amount_lowers_to_incr() {
        let m = lower_to_ir("incr counter -5", &reg());
        match &m.top_level.statements[0] {
            Statement::Incr { name, amount, .. } => {
                assert_eq!(name, "counter");
                assert_eq!(amount.as_deref(), Some("-5"));
            }
            other => panic!("expected Incr, got {other:?}"),
        }
    }

    #[test]
    fn incr_no_args_falls_back_to_call() {
        let m = lower_to_ir("incr", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "incr"
        ));
    }

    #[test]
    fn incr_too_many_args_falls_back_to_call() {
        let m = lower_to_ir("incr i 1 2", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "incr"
        ));
    }

    #[test]
    fn incr_with_brace_expansion_falls_back_to_call() {
        // {*} expansion hides the real arg count — must not take
        // the specialised Statement::Incr path.
        let m = lower_to_ir("incr {*}$args", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, .. } if command == "incr"
        ));
    }

    #[test]
    fn incr_dispatcher_routes_through_try_lower_hook() {
        // The shared dispatcher in ``lowering_hooks::try_lower_hook``
        // must route ``incr`` to this module.
        let m = lower_to_ir("set a 1\nincr a\nputs $a", &reg());
        let stmts = &m.top_level.statements;
        assert_eq!(stmts.len(), 3);
        assert!(matches!(stmts[0], Statement::AssignConst { .. }));
        assert!(matches!(stmts[1], Statement::Incr { .. }));
        assert!(matches!(stmts[2], Statement::Call { .. }));
    }

    // Unit-level coverage of try_lower_incr for shape combinations
    // that lower_to_ir cannot easily reach (e.g. the no-args and
    // expansion cases via direct LoweringCommand construction).

    fn make_cmd<'a>(
        args: &'a [String],
        single: &'a [bool],
        kinds: &'a [ArgTokenKind],
        expand: Option<&'a [bool]>,
    ) -> LoweringCommand<'a> {
        LoweringCommand {
            span: Span::new(0, 8),
            name: "incr",
            args,
            single_token_word: single,
            expand_word: expand,
            tokens: None,
            arg_kinds: kinds,
            dialect: None,
        }
    }

    #[test]
    fn unit_incr_basic() {
        let args = vec!["i".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Esc];
        let cmd = make_cmd(&args, &single, &kinds, None);
        match try_lower_incr(&cmd) {
            Statement::Incr {
                name,
                amount,
                safe_on_uninit,
                ..
            } => {
                assert_eq!(name, "i");
                assert!(amount.is_none());
                assert!(!safe_on_uninit);
            }
            other => panic!("expected Incr, got {other:?}"),
        }
    }

    #[test]
    fn unit_incr_with_amount() {
        let args = vec!["i".to_string(), "5".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc];
        let cmd = make_cmd(&args, &single, &kinds, None);
        match try_lower_incr(&cmd) {
            Statement::Incr { amount, .. } => assert_eq!(amount.as_deref(), Some("5")),
            other => panic!("expected Incr, got {other:?}"),
        }
    }

    #[test]
    fn unit_incr_empty_args_makes_call() {
        let args: Vec<String> = vec![];
        let single = vec![true];
        let kinds: Vec<ArgTokenKind> = vec![];
        let cmd = make_cmd(&args, &single, &kinds, None);
        assert!(matches!(try_lower_incr(&cmd), Statement::Call { .. }));
    }

    #[test]
    fn unit_incr_expansion_makes_call() {
        let args = vec!["i".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let expand = vec![false, true];
        let cmd = make_cmd(&args, &single, &kinds, Some(&expand));
        match try_lower_incr(&cmd) {
            Statement::Call {
                command, args: a, ..
            } => {
                assert_eq!(command, "incr");
                assert_eq!(a, vec!["i"]);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }
}
