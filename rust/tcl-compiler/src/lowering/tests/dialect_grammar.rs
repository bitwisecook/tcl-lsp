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

//! Lexical rules that come from the dialect or release: brace continuation, list words, array-index masks and expression sugar.

use super::*;

/// A brace-word line continuation is folded to a space by every build of
/// the Tcl core and kept verbatim by `JimTcl`, so a wrapped parameter or
/// variable list binds *different names* in the two.
///
/// Measured on jimsh 0.84 against tclsh 8.6. The two word kinds resolve
/// the surviving `\<newline>` differently, because only one of them has a
/// `name ?default?` level:
///
/// * **Formal parameters** — `proc p {a b\<newline>c} {}` binds three
///   parameters under Tcl and *two* under Jim: the second element is the
///   specifier `b c`, so the parameter is `b` **defaulting to `c`**
///   (`p 1` leaves `b` as `c`; `p 1 2` sets it to `2`).
/// * **Variable lists** — `foreach {a b\<newline>c} …` has no such level,
///   so Jim binds a variable whose name really is `b c`
///   (`info exists {b c}` is 1 and `info exists b` is 0).
///
/// Jim's `info args` is no help here: unlike Tcl's it reports the raw
/// specifiers (`a {b c}`), not the parameter names.
#[test]
pub(super) fn jim_brace_continuation_binds_different_names() {
    let wrapped = "a b\\\nc";

    assert_eq!(
        parse_formal_param_names(
            wrapped,
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"],
        "C Tcl folds the continuation into a separator"
    );
    assert_eq!(
        parse_formal_param_names(
            wrapped,
            LITERAL_RULES,
            Some(tcl_dialect::ParameterGrammar::Jim)
        )
        .unwrap(),
        vec!["a", "b"],
        "Jim keeps the bytes, so `b c` is one specifier: `b` defaulting to `c`"
    );

    assert_eq!(
        parse_var_list_names(wrapped, FOLD_RULES).unwrap(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        parse_var_list_names(wrapped, LITERAL_RULES).unwrap(),
        vec!["a", "b c"]
    );
}

/// `JimTcl`'s list parser never raises, so a word whose list text is
/// malformed still binds names instead of collapsing lowering to a
/// barrier. `llength "a {b"` is 2 under jimsh 0.84 and an error under
/// tclsh 8.6, and the same split backs `foreach`'s variable list.
#[test]
fn malformed_list_word_binds_names_only_for_jim() {
    assert_eq!(
        parse_var_list_names("a {b", FOLD_RULES),
        None,
        "C Tcl raises, which lowering turns into a barrier"
    );
    assert_eq!(
        parse_var_list_names("a {b", LITERAL_RULES).unwrap(),
        vec!["a", "b"],
        "Jim splits it anyway — the unterminated element runs to the end"
    );
}

/// Both axes reach the helpers from the dialect, so `list_parse` is not
/// silently left at its `Strict` default for a Jim document.
#[test]
fn list_word_rules_come_from_the_dialect() {
    use tcl_dialect::model::{Family, grammar};
    for &release in Family::Jim.releases() {
        let config = tcl_lexer::LexerConfig::from_grammar(grammar(Family::Jim, release));
        assert_eq!(
            WordValueRules::from_config(&config),
            LITERAL_RULES,
            "{release}"
        );
    }
    for &release in Family::Tcl.releases() {
        let config = tcl_lexer::LexerConfig::from_grammar(grammar(Family::Tcl, release));
        assert_eq!(
            WordValueRules::from_config(&config),
            FOLD_RULES,
            "{release}"
        );
    }
}

/// The knob is wired to the dialect, not chosen per call site.
#[test]
fn brace_continuation_comes_from_the_dialect() {
    use tcl_dialect::model::{Family, grammar};
    for &release in Family::Jim.releases() {
        assert_eq!(
            grammar(Family::Jim, release).brace_backslash_newline,
            tcl_dialect::BraceBackslashNewline::Literal,
            "{release}"
        );
    }
    for &release in Family::Tcl.releases() {
        assert_eq!(
            grammar(Family::Tcl, release).brace_backslash_newline,
            tcl_dialect::BraceBackslashNewline::Folds,
            "{release}"
        );
    }
}

#[test]
fn fatal_array_index_source_mask_follows_the_release() {
    let source = "puts $array({key})";
    for dialect in ["tcl8.4", "tcl8.6"] {
        assert_eq!(
            first_fatal_parse_error_with_config(
                source,
                tcl_lexer::LexerConfig::for_dialect(dialect)
            ),
            None,
            "{dialect} accepts the index text"
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        assert_eq!(
            first_fatal_parse_error_with_config(
                source,
                tcl_lexer::LexerConfig::for_dialect(dialect)
            ),
            Some(tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX.to_owned()),
            "{dialect} rejects the raw brace"
        );
    }
}

/// Regression (JimTcl/iRules dialect drift): every re-lex of a *body*
/// this lowering performs takes its grammar from the document's own
/// [`tcl_lexer::LexerConfig`], not the Tcl 9.x default.
///
/// `body_has_dynamic_barrier` re-segments a braced body to find a nested
/// dynamic `eval`/`uplevel`.  Under `f5-irules` the lexer treats `}{` as a
/// word separator, so `foo {a}{eval $x}` is **three** words and its third
/// is a braced script containing a dynamic `eval`; under the default
/// grammar the two braces weld into one compound word whose text is
/// `aeval $x`, whose head `aeval` is nobody's barrier.  Before the config
/// was threaded, the iRules document was re-segmented under Tcl rules and
/// the barrier was missed — the outer `eval` relaxed to inline IR that
/// runs a still-dynamic inner `eval`.
#[test]
fn body_relex_follows_the_document_dialect_not_the_default() {
    let irules = tcl_dialect::DialectProfile::irules();
    let irules_config = tcl_lexer::LexerConfig::from_grammar(irules.grammar);
    let registry = tcl_registry::model::ingress::static_context_for_profile(irules).commands();

    let body = "foo {a}{eval $x}";
    assert!(
        !body_has_dynamic_barrier(body, registry, tcl_lexer::LexerConfig::default()),
        "under the default grammar `}}{{` welds one word, so no nested barrier is visible"
    );
    assert!(
        body_has_dynamic_barrier(body, registry, irules_config),
        "under f5-irules `}}{{` separates words, exposing the nested dynamic `eval`"
    );

    // End to end: the same fact decides whether the outer `eval` relaxes
    // to an inline `Block` or stays a runtime `Barrier`.
    let source = "eval {foo {a}{eval $x}}";
    let irules_module = lower_to_ir_with_config(source, registry, irules_config);
    assert_eq!(irules_module.top_level.statements.len(), 1);
    assert_runtime_opaque_call(&irules_module.top_level.statements[0], registry, "eval");

    // Under the default grammar the weld makes `{a}{eval $x}` a *single*
    // word, and that word is not valid Tcl — C answers `extra characters
    // after close-brace` for this very body, on 8.6 and 9.0 alike. So the
    // relaxation is declined here too, by the parse gate rather than the
    // barrier gate (#1829): inlining would lower the malformed text with
    // the lenient lowering and give it a meaning C never gives it.
    let tcl_module = lower_to_ir_with_config(source, registry, tcl_lexer::LexerConfig::default());
    assert!(
        matches!(
            tcl_module.top_level.statements.as_slice(),
            [Statement::Barrier { .. }]
        ),
        "a body that does not parse keeps the runtime barrier: {:?}",
        tcl_module.top_level.statements
    );

    // Positive control: the relaxation is still alive. A body with no
    // nested barrier that *does* parse must still reach the inline Block,
    // so neither assertion above can pass merely because relaxation
    // stopped happening at all.
    let relaxable = lower_to_ir_with_config(
        "eval {foo a b}",
        registry,
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        matches!(
            relaxable.top_level.statements.as_slice(),
            [Statement::Block { .. }]
        ),
        "a parsing, barrier-free body still relaxes to a Block: {:?}",
        relaxable.top_level.statements
    );
}

/// `JimTcl`'s `$(…)` lowers as the expression it is, end to end from the
/// lexer: `set x $($a*2)` becomes the `AssignExpr` a `[expr {$a*2}]`
/// value produces, with the body anchored two bytes past the `$(` so
/// expression diagnostics land on the right source; `return $($a+1)`
/// carries its expression the same way.
#[test]
fn jim_expr_sugar_lowers_as_an_anchored_expression() {
    let jim = tcl_registry::model::resolve_environment("jim").unit_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(jim).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(jim.grammar);
    let module = lower_to_ir_with_config("set x $($a*2)\nreturn $($a+1)\n", registry, config);
    let statements = module.top_level.statements.as_slice();
    match statements {
        [
            Statement::AssignExpr {
                name,
                expr,
                expr_base,
                command_binding,
                ..
            },
            Statement::Return {
                expr: return_expr, ..
            },
        ] => {
            assert_eq!(name, "x");
            let rendered = format!("{expr:?}");
            assert!(rendered.contains('a'), "operand missing from {rendered}");
            // `set x ` is six bytes, `$(` two more: the body starts at 8.
            assert_eq!(*expr_base, Some(8), "the sugar body's absolute anchor");
            assert!(
                command_binding.is_none(),
                "Jim expression syntax must not manufacture an expr command binding"
            );
            assert!(return_expr.is_some(), "return $(…) carries its expression");
        }
        other => panic!("expected AssignExpr + Return; got {other:?}"),
    }
}
