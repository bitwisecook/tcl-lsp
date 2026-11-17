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

//! Folding an assigned value word: literals, variable references, reads in value position and pure command substitutions.

use super::*;

/// `[set x]` in value position reads `x` through the cell-write route;
/// `[set x 10]` writes storage a value position cannot land
/// (`EffectFreeOnly`), so its host widens.
#[test]
fn evaluate_def_set_read_in_value_position_folds() {
    // tclsh 8.6.18 and 9.0.4 print `<hello> <10 10> <CHANGED>` for a
    // nested read, a nested write, and a nested read of a place whose
    // read trace replaces it.
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set x hello; set r [set x]; return $r}",
            "r"
        ),
        text("hello")
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set x hello; set r [set x 10]; return $r}",
            "r"
        ),
        LatticeValue::Const(ConstValue::Int(10))
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc t args {}\nproc p {} {set x hello; trace add variable x read t; set r [set x]; return $r}",
            "r"
        ),
        LatticeValue::Overdefined
    );
    assert_eq!(
        settled("tcl8.6", "proc p {x} {set r [set x]; return $r}", "r"),
        LatticeValue::Overdefined
    );
}

/// `[string range "a\tb" 0 1]` in value position reads its words as Tcl
/// substitutes them: `a` and a tab, where the raw spelling gave `a\`. A
/// braced word keeps its backslash, and a braced `$x` is text, not a
/// read (tclsh 8.4 to 9.1 print `a` and a tab, `a\`, and `$x`).
#[test]
fn string_range_in_value_position_reads_cooked_escapes() {
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let fold = |word: &str| {
        settled(
            "tcl8.6",
            &format!("proc p {{}} {{set xy abc; set r [string range {word} 0 1]; return $r}}"),
            "r",
        )
    };
    assert_eq!(fold("\"a\\tb\""), text("a\t"));
    assert_eq!(fold("{a\\tb}"), text("a\\"));
    assert_eq!(fold("{$xy}"), text("$x"), "the braced word is text");
}

/// `[string range …]` in value position runs the registry's route: the
/// index numerals read under the target's grammar. `f5-irules` reads
/// them as its declared 8.4 base does (ruling 8; tclsh 8.4 prints
/// `ijkl`), and the lenient `tcl` profile, naming no release, declines.
#[test]
fn evaluate_def_assign_value_folds_string_range_under_the_release() {
    let under =
        |dialect: &str| settled_body(dialect, "set s [string range abcdefghijkl 010 end]", "s");
    assert_eq!(
        under("tcl8.6"),
        LatticeValue::Const(ConstValue::String("ijkl".into()))
    );
    assert_eq!(
        under("tcl9.0"),
        LatticeValue::Const(ConstValue::String("kl".into()))
    );
    assert_eq!(
        under("f5-irules"),
        LatticeValue::Const(ConstValue::String("ijkl".into()))
    );
    assert_eq!(under("tcl"), LatticeValue::Overdefined);
    assert_eq!(
        settled_body("tcl8.6", "set t [string range { a } 0 end]", "t"),
        LatticeValue::Const(ConstValue::String(" a ".into()))
    );
}

#[test]
fn evaluate_def_assign_value_plain_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_value_integer_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
}

#[test]
fn evaluate_def_assign_value_resolves_var_ref() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "y", "$x", 1);
    let x = ssa.intern_var("x");
    stmt.uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(7))
    );
}

#[test]
fn evaluate_def_assign_value_folds_list_cmd() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "a b c"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn evaluate_def_assign_value_folds_llength_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(4))
    );
}

#[test]
fn evaluate_def_assign_value_folds_string_length() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn string_length_fold_counts_in_the_selected_dialects_character_model() {
    // U+1D11E written in a UTF-8 source file: tclsh 9.0 and 9.1 decode
    // the source as UTF-8 and count one scalar; 8.4 to 8.6 decode it in
    // the system encoding and print 4. The registry's route answers
    // under the dialect's release and declines where the source's
    // decoding is ambiguous — 8.x and no selected release.
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
    let fold = |dialect: &str| {
        let folds = folds_for(dialect);
        evaluate_fixture_def_with_folds(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::from_registry(folds.registry),
            Some(folds),
            tcl_lexer::LexerConfig::default(),
        )
    };
    assert_eq!(fold("tcl9.0"), LatticeValue::Const(ConstValue::Int(1)));
    assert_eq!(fold("tcl8.6"), LatticeValue::Overdefined);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined,
        "no selected release leaves the source's decoding ambiguous"
    );

    // A string both models count identically still folds with no selected
    // release — declining those would drop an ordinary optimisation.
    let mut ssa = bare_ssa();
    let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(
            &ascii,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), None)
        ),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn evaluate_def_assign_value_folds_expr_cmd_subst() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[expr {1 + 2}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn evaluate_def_assign_value_folds_format_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
    match evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "1-2"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn quoted_expr_with_string_var_does_not_fold() {
    // `set r [expr "$a == $b"]` with a="alpha", b="beta": Tcl substitutes
    // the values textually before parsing, so `expr "alpha == beta"`
    // errors (`invalid bareword`). The fold must bail rather than treat
    // the strings as operands and return 0.
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a == $b\"]", 1);
    let a = ssa.intern_var("a");
    let b = ssa.intern_var("b");
    stmt.uses.insert(a, 1);
    stmt.uses.insert(b, 1);
    let mut values = HashMap::new();
    values.insert(
        (a, 1),
        LatticeValue::Const(ConstValue::String("alpha".into())),
    );
    values.insert(
        (b, 1),
        LatticeValue::Const(ConstValue::String("beta".into())),
    );
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_assign_value_unknown_cmd_widens() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_assign_value_llength_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "n", "[llength $lst]", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn parse_literal_value_prefers_int() {
    assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
    assert_eq!(parse_literal_value("-5"), ConstValue::Int(-5));
    assert_eq!(parse_literal_value("0"), ConstValue::Int(0));
    assert_eq!(
        parse_literal_value("hello"),
        ConstValue::String("hello".into())
    );
    // Leading-zero and non-canonical integer forms do not round-trip, so
    // they stay strings (lets SCCP apply the per-dialect leading-zero rule).
    assert_eq!(parse_literal_value("08"), ConstValue::String("08".into()));
    assert_eq!(parse_literal_value("010"), ConstValue::String("010".into()));
    assert_eq!(parse_literal_value("+5"), ConstValue::String("+5".into()));
    assert_eq!(parse_literal_value("-0"), ConstValue::String("-0".into()));
}
