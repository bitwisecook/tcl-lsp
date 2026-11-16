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

//! Expression typing: operators, math functions, literal operands and deep nesting.

use super::*;

// expr type-inference precision

/// Infer the type of a standalone expression string (no SSA
/// context — variable refs stay `Unknown`), under the Tcl 9.0 numeral
/// grammar.
fn infer_str(src: &str) -> TypeLattice {
    infer_str_dialect(src, None)
}

/// As [`infer_str`] but parses under `dialect` (the iRules string
/// predicates only tokenise as operators in the iRules dialect) and reads
/// numerals under that dialect's grammar.
fn infer_str_dialect(
    src: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> TypeLattice {
    let node = crate::parse_expr_for_profile(src, dialect);
    infer_expr_type(
        &node,
        &HashMap::new(),
        0,
        crate::intervals::numbers_for_dialect(dialect),
    )
}

#[test]
fn decoded_expression_names_keep_literal_sigils_and_element_identity() {
    let profile = tcl_registry::model::ingress::static_context_for("tcl8.6")
        .commands()
        .profile();
    let types = HashMap::from([
        ("b".to_owned(), TypeLattice::of(TclType::List)),
        ("$b".to_owned(), TypeLattice::of(TclType::Int)),
        ("a".to_owned(), TypeLattice::of(TclType::Dict)),
        ("a(k)".to_owned(), TypeLattice::of(TclType::Double)),
    ]);
    for (source, expected) in [("${$b}", TclType::Int), ("${a(k)}", TclType::Double)] {
        let expression = crate::parse_expr_for_profile(source, profile);
        let reference = expression
            .variable_reference(tcl_lexer::LexerConfig::from_grammar(
                profile.unwrap().grammar,
            ))
            .unwrap()
            .unwrap();
        assert!(reference.index.is_none());
        let ExprNode::Var { start, end, .. } = &expression else {
            panic!("variable");
        };
        let occurrence_types = HashMap::from([(
            (source.to_owned(), *start, *end),
            types[std::str::from_utf8(reference.name).unwrap()].clone(),
        )]);
        assert_eq!(
            infer_expr_type(
                &expression,
                &occurrence_types,
                0,
                NumberSyntax::of_profile(profile)
            )
            .tcl_type(),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn expression_contents_types_use_original_element_and_literal_name_reads() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        (
            "proc f {} {set {$b} [expr {7<<1}]; set b [list x]; return [expr {${$b}}]}; f",
            TclType::Int,
        ),
        (
            "proc f {} {set a(k) [expr {7.5}]; return [expr {${a(k)}}]}; f",
            TclType::Double,
        ),
        (
            "proc f {} {set k k; set a(k) [expr {7.5}]; return [expr {$a($k)}]}; f",
            TclType::Double,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        assert_eq!(
            unit.function("::f").unwrap().return_type.tcl_type(),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn written_container_reads_keep_raw_and_braced_variable_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for word in ["$items", "${items}"] {
        let source = format!(
            "proc f {{raw}} {{set items [list [expr {{$raw<<1}}]]; return [lindex {word} 0]}}; f 7"
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
        assert_eq!(
            unit.function("::f").unwrap().return_type.tcl_type(),
            Some(TclType::Int),
            "{source}"
        );
    }
}

#[test]
fn math_function_calls_infer_their_return_type() {
    // (a) sqrt → Double, int → Int, bool → Boolean.
    assert_eq!(infer_str("sqrt(2.0)").tcl_type(), Some(TclType::Double));
    assert_eq!(infer_str("sin($x)").tcl_type(), Some(TclType::Double));
    assert_eq!(infer_str("int($x)").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("wide($x)").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("isnan($x)").tcl_type(), Some(TclType::Boolean));
    // abs is identity: abs(2) keeps the operand's Int type.
    assert_eq!(infer_str("abs(2)").tcl_type(), Some(TclType::Int));
    // max/min join operands: max(1, 2) stays Int.
    assert_eq!(infer_str("max(1, 2)").tcl_type(), Some(TclType::Int));
    // Tcl 9.1 C99 functions (TIP 745): double-valued, except `signbit`.
    for f in [
        "acosh", "cbrt", "exp2", "log2", "trunc", "erf", "expm1", "logb",
    ] {
        assert_eq!(
            infer_str(&format!("{f}($x)")).tcl_type(),
            Some(TclType::Double),
            "{f} should infer Double",
        );
    }
    assert_eq!(infer_str("signbit($x)").tcl_type(), Some(TclType::Boolean));
    // The rest of TIP 521's classification family answers 0 or 1 like
    // `isnan` (tclsh 9.0 and 9.1: `isfinite(1.0)` is 1, `isunordered(1,
    // 2)` is 0); the table this reads classes them as `isnan` is.
    for f in ["isfinite", "isnormal", "issubnormal"] {
        assert_eq!(
            infer_str(&format!("{f}($x)")).tcl_type(),
            Some(TclType::Boolean),
            "{f} should infer Boolean",
        );
    }
    assert_eq!(
        infer_str("isunordered($x, 1)").tcl_type(),
        Some(TclType::Boolean)
    );
    // Unknown function → Numeric (conservative).
    assert_eq!(infer_str("nope($x)").tcl_type(), Some(TclType::Numeric));
}

/// `infer_expr_type` (and its
/// mutually-recursive helper `expr_call_type`) recurse once per
/// `ExprNode` operator-tree level, so both need a depth cap.
/// A long left-associative `1+1+1+…` chain parses *iteratively* (the
/// Pratt parser's own `MAX_EXPR_DEPTH` never trips — it builds the
/// left-nested `Binary` spine in a loop), so it hands this walker an
/// arbitrarily deep tree the parser cap does not bound. Empirically that
/// unguarded walk overflowed the native stack (SIGABRT) in the low
/// thousands of levels on a 2 MiB thread (`cargo test`'s per-test
/// default). 3000 is comfortably past both that crash range and
/// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that inference returns
/// at all, not what it returns. Also exercises `expr_call_type` via the
/// nested-`min(…)` variant.
#[test]
fn deeply_nested_expr_type_inference_survives() {
    let chain = format!("1{}", "+1".repeat(3000));
    // Returns without overflowing the stack; the value is unspecified
    // past the cap (a conservative `overdefined`/`numeric`).
    let _ = infer_str(&chain);

    // Drive the `expr_call_type` side of the same recursion cluster with
    // a tree built directly — nested `min(min(…))` *source* would trip
    // the Pratt parser's own `MAX_EXPR_DEPTH` first (function-call
    // nesting recurses the parser, unlike the iterative binary chain
    // above), so build the deep `Call` spine by hand to reach this
    // walker unbounded.
    let mut node = ExprNode::Literal {
        text: "1".to_owned(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Call {
            function: "abs".to_owned(),
            args: vec![node],
            start: 0,
            end: 1,
        };
    }
    let _ = infer_expr_type(&node, &HashMap::new(), 0, NumberSyntax::Tcl90);
}

/// Companion to `deeply_nested_expr_type_inference_survives`: moderate
/// nesting (well under `MAX_EXPR_NODE_DEPTH`) must still infer exactly as
/// before — the cap changes nothing for realistic input. A 100-deep
/// all-integer additive chain is unambiguously `Int`.
#[test]
fn moderate_depth_expr_type_inference_unchanged() {
    let chain = format!("1{}", "+1".repeat(100));
    assert_eq!(infer_str(&chain).tcl_type(), Some(TclType::Int));
}

#[test]
fn bitwise_and_shift_ops_infer_int() {
    // (c) bitwise / shift force Int even with untyped operands.
    for src in ["$x & $y", "$x | $y", "$x ^ $y", "$x << 2", "$x >> 2"] {
        assert_eq!(
            infer_str(src).tcl_type(),
            Some(TclType::Int),
            "expected Int for `{src}`",
        );
    }
}

#[test]
fn irules_string_predicates_infer_boolean() {
    // (b) the iRules string predicates were falling through to
    // overdefined; they are Boolean.
    for src in [
        "$s contains \"x\"",
        "$s starts_with \"x\"",
        "$s ends_with \"x\"",
        "$s equals \"x\"",
        "$s matches_glob \"x*\"",
        "$s matches_regex \"x.\"",
    ] {
        let t = infer_str_dialect(src, Some(tcl_dialect::DialectProfile::irules()));
        assert_eq!(
            t.tcl_type(),
            Some(TclType::Boolean),
            "expected Boolean for `{src}`, got {t:?}",
        );
    }
}

#[test]
fn arithmetic_promotes_double() {
    // `_arithmetic_result`: int + double → Double (was Numeric).
    assert_eq!(infer_str("3 + 2.0").tcl_type(), Some(TclType::Double));
    // int + int → Int.
    assert_eq!(infer_str("3 + 4").tcl_type(), Some(TclType::Int));
}

#[test]
fn expr_context_literal_typing() {
    // Every integer spelling tokenises to Int in expr context — including
    // octal `0o…`, which the set-statement classifier keeps as String.
    assert_eq!(infer_str("0o17").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("0xff").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("0b1010").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("42").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("3.14").tcl_type(), Some(TclType::Double));
    // The set-statement classifier still keeps octal as String.
    assert_eq!(
        literal_type("0o17", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
    // An unrecognised literal degrades to Numeric in expr context
    // (the set-statement classifier would say String).
    assert_eq!(
        expr_literal_type("nope", NumberSyntax::Tcl90).tcl_type(),
        Some(TclType::Numeric)
    );
}

#[test]
fn bitnot_coerces_to_int() {
    // `~$x` always yields Int, regardless of the operand's type
    // (was leaking the operand type via the identity arm).
    assert_eq!(infer_str("~$x").tcl_type(), Some(TclType::Int));
    assert_eq!(infer_str("~3.5").tcl_type(), Some(TclType::Int));
}
