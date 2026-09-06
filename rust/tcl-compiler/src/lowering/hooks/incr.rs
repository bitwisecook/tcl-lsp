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
pub fn try_lower_incr(cmd: &LoweringCommand<'_>, safe_on_uninit: bool) -> Statement {
    if has_expansion(cmd) || cmd.args.is_empty() || cmd.args.len() > 2 {
        return make_call(cmd);
    }
    // A *compound* name word computes the name too, and the arg-kind check
    // below cannot see it: only the word's representative token is
    // consulted, so under a grammar with no `{*}` expansion (8.4, iRules)
    // `incr {*}$n` reads as a `Str` word while its value is the literal `*`
    // welded to whatever `$n` holds — the same #1484 hole `lower_set` closed.
    // `Statement::Incr`'s `name` is static by contract and
    // `dynamic_names::scan_statement` never inspects it, so a computed name
    // must stay a generic `Call`, the form `scan_command` does inspect and
    // which raises the dynamic-write barrier the value-motion passes need.
    //
    // Only a word that really substitutes is examined, because
    // `names_a_dynamic_variable` reads text alone: `incr \$x` is one literal
    // token naming the variable `$x`, and `incr {$n}` names `$n`, neither of
    // which substitutes anything. Conversely a computed array *key*
    // (`incr a($i)`) is not a computed name — the array is named statically
    // and codegen builds the key at run time — the same line
    // `names_a_dynamic_variable` already draws for `scan_command`.
    if !cmd.arg_is_static_literal(0) && crate::dynamic_names::names_a_dynamic_variable(&cmd.args[0])
    {
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
        safe_on_uninit,
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

    /// The statement `src` lowers to under `dialect` — the configuration a
    /// real host builds, so the `{*}` grammar matches the document's.
    fn first_stmt_for_dialect(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Statement {
        let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
        let m = crate::lowering::lower_to_ir_with_config(
            src,
            registry,
            tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
        );
        m.top_level.statements[0].clone()
    }

    /// Issue #1487 — a computed name may not wear a static `Incr` shape.
    ///
    /// Under a grammar with no `{*}` expansion the word `{*}$n` is the
    /// literal `*` welded to `$n`, whose *representative* token is the
    /// braced `{*}` — literal enough to slip past the arg-kind check,
    /// computed enough that `Statement::Incr`'s static-name contract is a
    /// lie. It must stay a `Call`, the form `dynamic_names::scan_command`
    /// inspects.
    #[test]
    fn try_lower_incr_refuses_a_computed_name_under_an_expansionless_grammar() {
        for dialect in ["tcl8.4", "f5-irules"] {
            // The welded `{*}$n` shape exists only under tcl8.4: the F5
            // fork's implicit word break splits it into `*` + `$n`
            // (measurements §1/§3 row 6,
            // `docs/design/bigip-irule-parser-measurements.md`), so its
            // name word is the *literal* `*` there, not a computed name.
            let sources: &[&str] = if *dialect == *"tcl8.4" {
                &["incr {*}$n", "incr x$n", "incr pre[f]"]
            } else {
                &["incr x$n", "incr pre[f]"]
            };
            for src in sources {
                let stmt = first_stmt_for_dialect(
                    src,
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(
                    matches!(&stmt, Statement::Call { command, .. } if command == "incr"),
                    "{dialect}: {src:?} must stay a Call, got {stmt:?}",
                );
            }
        }
    }

    /// The gate reads the *word*, not just its text: these names are
    /// spelled out (a backslash-escaped `$`, a brace-quoted `$n`) or name a
    /// static array whose key alone is computed, so they keep their typed
    /// `Statement::Incr`.
    #[test]
    fn try_lower_incr_keeps_the_specialisation_for_a_spelled_out_name() {
        for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
            for src in ["incr x", "incr {$n}", "incr \\$x", "incr a($i)"] {
                let stmt = first_stmt_for_dialect(
                    src,
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(
                    matches!(&stmt, Statement::Incr { .. }),
                    "{dialect}: {src:?} must keep its typed Incr, got {stmt:?}",
                );
            }
        }
    }

    /// Under 9.0/8.6 `{*}$n` is a real expansion, which `has_expansion` has
    /// always rejected — the #1487 gate must not be what decides this case.
    #[test]
    fn try_lower_incr_leaves_the_expanded_name_word_on_its_existing_path() {
        for dialect in ["tcl9.0", "tcl8.6"] {
            let stmt = first_stmt_for_dialect(
                "incr {*}$n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            let Statement::Call {
                command, tokens, ..
            } = &stmt
            else {
                panic!("{dialect}: expected Call, got {stmt:?}");
            };
            assert_eq!(command, "incr");
            assert_eq!(
                tokens.as_ref().and_then(|t| t.expand_word.clone()),
                Some(vec![false, true]),
                "{dialect}: the word must still be marked as expanded",
            );
        }
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
    fn unit_incr_basic() {
        let args = vec!["i".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Esc];
        let cmd = make_cmd(&args, &single, &kinds, None);
        match try_lower_incr(&cmd, false) {
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
        match try_lower_incr(&cmd, false) {
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
        assert!(matches!(
            try_lower_incr(&cmd, false),
            Statement::Call { .. }
        ));
    }

    #[test]
    fn unit_incr_expansion_makes_call() {
        let args = vec!["i".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Var];
        let expand = vec![false, true];
        let cmd = make_cmd(&args, &single, &kinds, Some(&expand));
        match try_lower_incr(&cmd, false) {
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
