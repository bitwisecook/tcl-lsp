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

//! Adversarial hardening tests for the query DSL.
//!
//! The query string is untrusted user input, so every one of these inputs
//! must surface as a clean `Err`/marker rather than a stack overflow (which
//! aborts the process) or an arithmetic panic. They are all bounded — a few
//! thousand iterations at most — so the suite stays fast.
//!
//! The recursion-sensitive cases run on a worker thread with a generous
//! stack. The default libtest thread stack is only ~2 MiB, which is both
//! smaller than the production stack the guards are sized against and too
//! small to *build / drop* the adversarial inputs (an `Expr` / `Value` tree
//! drops recursively). A roomy worker thread lets us prove the guard returns
//! before the (much larger) production stack would overflow, without the
//! harness's own teardown faulting.

use tcl_bigip_query::ast::{Expr, LitValue, Program};
use tcl_bigip_query::errors::QueryError;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::jsonfmt;
use tcl_bigip_query::lexer::tokenise;
use tcl_bigip_query::parser::parse_query;
use tcl_bigip_query::value::Value;

/// Run *f* on a worker thread with a 256 MiB stack and propagate its result.
///
/// The stack must hold both the descent down to the guard (a few thousand
/// frames) and the *recursive drop* of the deep `Expr` / `Value` tree the
/// test builds (one frame per level), so it is sized well above either.
fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn worker thread")
        .join()
        .expect("worker thread did not panic / overflow")
}

/// A deeply-nested parenthesised query must be rejected with a parse error,
/// not overflow the parser's recursive descent.
#[test]
fn deeply_nested_parens_return_parse_error() {
    with_big_stack(|| {
        let depth = 50_000;
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        match parse_query(&src) {
            Err(QueryError::Parse { .. }) => {}
            other => panic!("expected a Parse error, got {other:?}"),
        }
    });
}

/// A long run of `not` prefixes self-recurses; it too must hit the depth
/// budget rather than the stack.
#[test]
fn deeply_nested_not_prefixes_return_parse_error() {
    with_big_stack(|| {
        let src = format!("{}.", "not ".repeat(50_000));
        assert!(matches!(parse_query(&src), Err(QueryError::Parse { .. })));
    });
}

/// A long run of unary minus prefixes likewise.
#[test]
fn deeply_nested_unary_minus_returns_parse_error() {
    with_big_stack(|| {
        let src = format!("{}1", "-".repeat(50_000));
        assert!(matches!(parse_query(&src), Err(QueryError::Parse { .. })));
    });
}

/// A query within the depth budget still parses — the guard rejects only the
/// abusive case, not realistic nesting. 20 levels of grouping is already far
/// deeper than any real query yet well under the cap.
#[test]
fn moderately_nested_query_still_parses() {
    let src = format!("{}1{}", "(".repeat(20), ")".repeat(20));
    assert!(parse_query(&src).is_ok());
}

/// A hand-built AST nested far deeper than any parseable query must evaluate
/// to an `Eval` error rather than overflow the native stack. Unary minus adds
/// one `eval` frame per layer without consuming the arithmetic fast paths.
#[test]
fn deeply_nested_ast_returns_eval_error() {
    with_big_stack(|| {
        let mut expr = Expr::Literal {
            value: LitValue::Int(1),
            offset: 0,
        };
        for _ in 0..50_000 {
            expr = Expr::UnaryOp {
                op: "-".to_string(),
                operand: Box::new(expr),
                offset: 0,
            };
        }
        let program = Program {
            statements: vec![expr],
        };
        let root = Root::json("data.json", Value::Int(1));
        let mut ctx = EvalContext::new(root);
        match evaluate(&program, &mut ctx) {
            Err(QueryError::Eval(_)) => {}
            other => panic!("expected an Eval error, got {other:?}"),
        }
    });
}

/// Evaluate *query* against a trivial JSON root, returning its error (if any).
fn eval_err(query: &str) -> Option<QueryError> {
    let program = parse_query(query).expect("query parses");
    let root = Root::json("data.json", Value::Int(0));
    let mut ctx = EvalContext::new(root);
    evaluate(&program, &mut ctx).err()
}

/// Integer addition overflow returns a clean evaluator error, never a debug
/// panic or a silent release-mode wrap.
#[test]
fn integer_add_overflow_returns_eval_error() {
    let err = eval_err("9223372036854775807 + 1").expect("overflow must error");
    assert!(matches!(err, QueryError::Eval(_)));
    assert!(err.to_string().contains("overflow"), "got {err}");
}

/// Integer multiplication overflow returns a clean evaluator error.
#[test]
fn integer_mul_overflow_returns_eval_error() {
    let err = eval_err("9223372036854775807 * 2").expect("overflow must error");
    assert!(matches!(err, QueryError::Eval(_)));
}

/// Integer subtraction overflow (`i64::MIN - 1`, spelt without a literal
/// `i64::MIN`) returns a clean evaluator error.
#[test]
fn integer_sub_overflow_returns_eval_error() {
    // -9223372036854775807 - 2 underflows past i64::MIN.
    let err = eval_err("-9223372036854775807 - 2").expect("underflow must error");
    assert!(matches!(err, QueryError::Eval(_)));
}

/// Arithmetic that stays in range is unaffected by the checked ops.
#[test]
fn in_range_integer_arithmetic_still_works() {
    let program = parse_query("2 + 3 * 4").expect("parses");
    let root = Root::json("data.json", Value::Int(0));
    let mut ctx = EvalContext::new(root);
    let out = evaluate(&program, &mut ctx).expect("evaluates");
    // `Value` is not `PartialEq`, so match the single produced int directly.
    assert_eq!(out.len(), 1);
    match out[0] {
        Value::Int(14) => {}
        ref other => panic!("expected Int(14), got {other:?}"),
    }
}

/// An integer literal too large for `i64` must lex to an error rather than
/// panic in the tokeniser's `i64` parse.
#[test]
fn oversized_integer_literal_returns_lex_error() {
    let err = tokenise("99999999999999999999").expect_err("must not panic");
    assert!(matches!(err, QueryError::Lex { .. }), "got {err:?}");
    // The full pipeline surfaces it too (rather than panicking on the way in).
    assert!(parse_query("99999999999999999999").is_err());
}

/// An in-range integer literal still lexes fine.
#[test]
fn max_i64_literal_lexes() {
    assert!(tokenise("9223372036854775807").is_ok());
}

/// Rendering a pathologically deep value must terminate with a truncation
/// marker instead of overflowing the serialiser's recursion.
#[test]
fn deeply_nested_value_serialises_without_overflow() {
    with_big_stack(|| {
        let mut value = Value::Int(1);
        for _ in 0..50_000 {
            value = Value::List(vec![value]);
        }
        // The point is that these return at all (no stack overflow / abort).
        let pretty = jsonfmt::to_pretty(&value);
        let compact = jsonfmt::to_compact(&value);
        let sorted = jsonfmt::to_compact_sorted(&value);
        assert!(compact.contains("max depth exceeded"));
        assert!(pretty.contains("max depth exceeded"));
        assert!(sorted.contains("max depth exceeded"));
        // Brackets stay balanced: every opened array is closed.
        assert_eq!(
            compact.matches('[').count(),
            compact.matches(']').count(),
            "array brackets must stay balanced",
        );
    });
}

/// A shallow value still round-trips unchanged — the depth guard does not
/// perturb ordinary output.
#[test]
fn shallow_value_serialises_normally() {
    let value = Value::List(vec![Value::Int(1), Value::Int(2)]);
    assert_eq!(jsonfmt::to_compact(&value), "[1,2]");
}

/// The `add` builtin sums integers with `checked_add`: an overflow is a clean
/// builtin error, never a debug panic / release wrap (issue 193).
#[test]
fn add_builtin_integer_overflow_returns_error() {
    let err = eval_err("[9223372036854775807, 1] | add").expect("add overflow must error");
    assert!(matches!(err, QueryError::Builtin(_)), "got {err:?}");
    assert!(err.to_string().contains("overflow"), "got {err}");
}

/// `range` whose stride steps past the i64 boundary terminates cleanly rather
/// than overflow-panicking on `cur += step` (issue 193).
#[test]
fn range_stepping_past_i64_max_does_not_panic() {
    let program =
        parse_query("range(9223372036854775805, 9223372036854775807, 5)").expect("parses");
    let root = Root::json("data.json", Value::Int(0));
    let mut ctx = EvalContext::new(root);
    // The point is that it returns at all (no overflow panic).
    let out = evaluate(&program, &mut ctx).expect("range evaluates without panic");
    assert_eq!(out.len(), 1, "range produces a single stream value");
}

/// Unary negation of `i64::MIN` is a clean error, not an overflow panic
/// (issue 193).
#[test]
fn unary_negation_of_i64_min_returns_error() {
    let program = parse_query("- .").expect("parses");
    let root = Root::json("data.json", Value::Int(i64::MIN));
    let mut ctx = EvalContext::new(root);
    let err = evaluate(&program, &mut ctx).expect_err("negating i64::MIN must error");
    assert!(err.to_string().contains("overflow"), "got {err}");
}
