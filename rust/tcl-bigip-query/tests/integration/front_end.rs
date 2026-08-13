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

//! Golden differential test for the query-DSL front-end.
//!
//! The fixtures in `tests/fixtures/front_end.json` are a captured corpus: for a
//! curated matrix of query strings they record the exact token stream and
//! AST `tokenise` / `parse_query` produce, plus the lexer /
//! parser error for a set of malformed inputs.
//!
//! This test re-derives the same JSON from the front-end and asserts equality,
//! so the lexer + parser stay byte-for-byte faithful to that corpus. It is
//! fully self-contained — no external reference at test time.

use serde_json::{Value, json};
use tcl_bigip_query::ast::{Expr, LitValue, PathStep, Program};
use tcl_bigip_query::errors::QueryError;
use tcl_bigip_query::lexer::{Token, tokenise};
use tcl_bigip_query::parser::parse_query;

fn value_json(v: &LitValue) -> Value {
    match v {
        LitValue::Null => json!({"t": "null"}),
        LitValue::Bool(b) => json!({"t": "bool", "v": b}),
        LitValue::Int(i) => json!({"t": "int", "v": i}),
        LitValue::Float(f) => json!({"t": "float", "v": f}),
        LitValue::Str(s) => json!({"t": "str", "v": s}),
    }
}

fn token_json(t: &Token) -> Value {
    json!({
        "kind": t.kind.name(),
        "text": t.text,
        "offset": t.offset,
        "value": value_json(&t.value),
    })
}

fn step_json(step: &PathStep) -> Value {
    match step {
        PathStep::Field {
            name,
            optional,
            offset,
        } => json!({
            "node": "Field",
            "name": name,
            "optional": optional,
            "offset": offset,
        }),
        PathStep::Subscript {
            stream,
            index,
            regex,
            offset,
            optional,
        } => json!({
            "node": "Subscript",
            "stream": stream,
            "index": opt_node(index.as_deref()),
            "regex": regex,
            "offset": offset,
            "optional": optional,
        }),
    }
}

fn opt_node(node: Option<&Expr>) -> Value {
    match node {
        Some(inner) => node_json(inner),
        None => Value::Null,
    }
}

// `too_many_lines`: one match arm per `Expr` variant — a flat JSON encoder that
// reads top-to-bottom; splitting it would only add indirection to a test helper.
#[allow(clippy::too_many_lines)]
fn node_json(node: &Expr) -> Value {
    match node {
        Expr::Literal { value, offset } => json!({
            "node": "Literal",
            "value": value_json(value),
            "offset": offset,
        }),
        Expr::Identity { offset } => json!({"node": "Identity", "offset": offset}),
        Expr::Variable { name, offset } => json!({
            "node": "Variable",
            "name": name,
            "offset": offset,
        }),
        Expr::ObjectLiteral { entries, offset } => json!({
            "node": "ObjectLiteral",
            "entries": entries
                .iter()
                .map(|(k, v)| json!([k, node_json(v)]))
                .collect::<Vec<_>>(),
            "offset": offset,
        }),
        Expr::ListLiteral { inner, offset } => json!({
            "node": "ListLiteral",
            "inner": opt_node(inner.as_deref()),
            "offset": offset,
        }),
        Expr::PathExpr {
            steps,
            offset,
            head,
        } => json!({
            "node": "PathExpr",
            "steps": steps.iter().map(step_json).collect::<Vec<_>>(),
            "offset": offset,
            "head": opt_node(head.as_deref()),
        }),
        Expr::Call { name, args, offset } => json!({
            "node": "Call",
            "name": name,
            "args": args.iter().map(node_json).collect::<Vec<_>>(),
            "offset": offset,
        }),
        Expr::BinOp {
            op,
            lhs,
            rhs,
            offset,
        } => json!({
            "node": "BinOp",
            "op": op,
            "lhs": node_json(lhs),
            "rhs": node_json(rhs),
            "offset": offset,
        }),
        Expr::UnaryOp {
            op,
            operand,
            offset,
        } => json!({
            "node": "UnaryOp",
            "op": op,
            "operand": node_json(operand),
            "offset": offset,
        }),
        Expr::Pipe { lhs, rhs, offset } => json!({
            "node": "Pipe",
            "lhs": node_json(lhs),
            "rhs": node_json(rhs),
            "offset": offset,
        }),
        Expr::LetBinding {
            source,
            name,
            body,
            offset,
        } => json!({
            "node": "LetBinding",
            "source": node_json(source),
            "name": name,
            "body": node_json(body),
            "offset": offset,
        }),
        Expr::Assignment {
            target,
            op,
            rhs,
            offset,
            source,
        } => json!({
            "node": "Assignment",
            "target": node_json(target),
            "op": op,
            "rhs": node_json(rhs),
            "offset": offset,
            "source": match source {
                Some(s) => node_json(s),
                None => Value::Null,
            },
        }),
        Expr::CommaStream { parts, offset } => json!({
            "node": "CommaStream",
            "parts": parts.iter().map(node_json).collect::<Vec<_>>(),
            "offset": offset,
        }),
        Expr::IfThenElse {
            cond,
            then_body,
            elifs,
            else_body,
            offset,
        } => json!({
            "node": "IfThenElse",
            "cond": node_json(cond),
            "then_body": node_json(then_body),
            "elifs": elifs
                .iter()
                .map(|(c, b)| json!([node_json(c), node_json(b)]))
                .collect::<Vec<_>>(),
            "else_body": match else_body {
                Some(e) => node_json(e),
                None => Value::Null,
            },
            "offset": offset,
        }),
    }
}

fn program_json(program: &Program) -> Value {
    json!({
        "node": "Program",
        "statements": program.statements.iter().map(node_json).collect::<Vec<_>>(),
    })
}

fn load_fixture() -> Value {
    let raw = include_str!("../fixtures/front_end.json");
    serde_json::from_str(raw).expect("fixture is valid JSON")
}

#[test]
fn token_streams() {
    let fixture = load_fixture();
    let cases = fixture["tokens"].as_array().expect("tokens array");
    assert!(!cases.is_empty(), "fixture has no token cases");
    for case in cases {
        let query = case["query"].as_str().expect("query string");
        let expected = &case["tokens"];
        let toks = tokenise(query).unwrap_or_else(|e| panic!("tokenise({query:?}) failed: {e}"));
        let actual = Value::Array(toks.iter().map(token_json).collect());
        assert_eq!(
            &actual, expected,
            "token stream mismatch for query {query:?}"
        );
    }
}

#[test]
fn asts() {
    let fixture = load_fixture();
    let cases = fixture["ast"].as_array().expect("ast array");
    assert!(!cases.is_empty(), "fixture has no AST cases");
    for case in cases {
        let query = case["query"].as_str().expect("query string");
        let expected = &case["ast"];
        let program =
            parse_query(query).unwrap_or_else(|e| panic!("parse_query({query:?}) failed: {e}"));
        let actual = program_json(&program);
        assert_eq!(&actual, expected, "AST mismatch for query {query:?}");
    }
}

#[test]
fn errors() {
    let fixture = load_fixture();
    let cases = fixture["errors"].as_array().expect("errors array");
    assert!(!cases.is_empty(), "fixture has no error cases");
    for case in cases {
        let query = case["query"].as_str().expect("query string");
        let kind = case["error"].as_str().expect("error kind");
        let message = case["message"].as_str().expect("error message");
        let offset =
            usize::try_from(case["offset"].as_u64().expect("error offset")).expect("offset fits");

        let err =
            parse_query(query).expect_err(&format!("expected parse_query({query:?}) to error"));
        match (&err, kind) {
            (QueryError::Lex { .. }, "lex") | (QueryError::Parse { .. }, "parse") => {}
            _ => panic!("error-kind mismatch for query {query:?}: got {err:?}, want {kind}"),
        }
        assert_eq!(
            err.message(),
            message,
            "error message mismatch for {query:?}"
        );
        assert_eq!(err.offset(), offset, "error offset mismatch for {query:?}");
    }
}
