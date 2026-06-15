#!/usr/bin/env python3
"""Generate golden differential fixtures for the Rust ``tcl-bigip-query`` port.

Captures, for a curated matrix of query strings, the exact token stream
and AST that the Python query DSL front-end
(:mod:`dialects.f5.query.lexer` / :mod:`dialects.f5.query.parser`)
produces, plus the lexer / parser error messages for a set of malformed
inputs.  The Rust front-end golden test re-derives the same JSON and
asserts equality, so the fixtures are captured once (here) and checked
in — the Rust test never shells out to Python.

Run from the repo root::

    python3 scripts/codegen/gen_f5_query_fixtures.py

Writes ``rust/tcl-bigip-query/tests/fixtures/front_end.json``.
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.query.ast import (
    Assignment,
    BinOp,
    Call,
    CommaStream,
    Field,
    Identity,
    IfThenElse,
    LetBinding,
    ListLiteral,
    Literal,
    ObjectLiteral,
    PathExpr,
    Pipe,
    Program,
    Subscript,
    UnaryOp,
    Variable,
)
from dialects.f5.query.errors import LexError, ParseError
from dialects.f5.query.lexer import Token, tokenise
from dialects.f5.query.parser import parse_query


def value_json(v: object) -> dict:
    """Tag a literal/token value so int vs float vs str stay distinct."""
    if v is None:
        return {"t": "null"}
    if isinstance(v, bool):
        return {"t": "bool", "v": v}
    if isinstance(v, int):
        return {"t": "int", "v": v}
    if isinstance(v, float):
        return {"t": "float", "v": v}
    if isinstance(v, str):
        return {"t": "str", "v": v}
    raise TypeError(f"unexpected value type {type(v)!r}")


def token_json(t: Token) -> dict:
    return {
        "kind": t.kind.name,
        "text": t.text,
        "offset": t.offset,
        "value": value_json(t.value),
    }


def node_json(node: object) -> object:
    if node is None:
        return None
    if isinstance(node, Program):
        return {"node": "Program", "statements": [node_json(s) for s in node.statements]}
    if isinstance(node, Literal):
        return {"node": "Literal", "value": value_json(node.value), "offset": node.offset}
    if isinstance(node, Identity):
        return {"node": "Identity", "offset": node.offset}
    if isinstance(node, Variable):
        return {"node": "Variable", "name": node.name, "offset": node.offset}
    if isinstance(node, ObjectLiteral):
        return {
            "node": "ObjectLiteral",
            "entries": [[k, node_json(v)] for (k, v) in node.entries],
            "offset": node.offset,
        }
    if isinstance(node, ListLiteral):
        return {"node": "ListLiteral", "inner": node_json(node.inner), "offset": node.offset}
    if isinstance(node, Field):
        return {
            "node": "Field",
            "name": node.name,
            "optional": node.optional,
            "offset": node.offset,
        }
    if isinstance(node, Subscript):
        return {
            "node": "Subscript",
            "stream": node.stream,
            "index": node_json(node.index),
            "regex": node.regex,
            "offset": node.offset,
            "optional": node.optional,
        }
    if isinstance(node, PathExpr):
        return {
            "node": "PathExpr",
            "steps": [node_json(s) for s in node.steps],
            "offset": node.offset,
            "head": node_json(node.head),
        }
    if isinstance(node, Call):
        return {
            "node": "Call",
            "name": node.name,
            "args": [node_json(a) for a in node.args],
            "offset": node.offset,
        }
    if isinstance(node, BinOp):
        return {
            "node": "BinOp",
            "op": node.op,
            "lhs": node_json(node.lhs),
            "rhs": node_json(node.rhs),
            "offset": node.offset,
        }
    if isinstance(node, UnaryOp):
        return {
            "node": "UnaryOp",
            "op": node.op,
            "operand": node_json(node.operand),
            "offset": node.offset,
        }
    if isinstance(node, Pipe):
        return {
            "node": "Pipe",
            "lhs": node_json(node.lhs),
            "rhs": node_json(node.rhs),
            "offset": node.offset,
        }
    if isinstance(node, LetBinding):
        return {
            "node": "LetBinding",
            "source": node_json(node.source),
            "name": node.name,
            "body": node_json(node.body),
            "offset": node.offset,
        }
    if isinstance(node, Assignment):
        return {
            "node": "Assignment",
            "target": node_json(node.target),
            "op": node.op,
            "rhs": node_json(node.rhs),
            "offset": node.offset,
            "source": node_json(node.source),
        }
    if isinstance(node, CommaStream):
        return {
            "node": "CommaStream",
            "parts": [node_json(p) for p in node.parts],
            "offset": node.offset,
        }
    if isinstance(node, IfThenElse):
        return {
            "node": "IfThenElse",
            "cond": node_json(node.cond),
            "then_body": node_json(node.then_body),
            "elifs": [[node_json(c), node_json(b)] for (c, b) in node.elifs],
            "else_body": node_json(node.else_body),
            "offset": node.offset,
        }
    raise TypeError(f"unexpected node type {type(node)!r}")


# Curated matrix: every syntactic form in the grammar, plus the
# cookbook examples (examples.py) so the parser is exercised on the
# exact queries the help surface advertises.
QUERIES: list[str] = [
    # identity / fields
    ".",
    ".name",
    '."full-path"',
    ".data-group",
    ".ltm.virtual",
    # subscripts
    ".ltm.virtual[]",
    '.ltm.pool["/Common/web"]',
    '.ltm.virtual["~/vs_prod_"]',
    '.ltm.virtual["~"]',
    ".x[0]",
    ".x[3].y",
    ".[0]",
    ".foo?",
    ".[]?",
    ".x[0]?",
    '.["~^/Common/"]',
    # pipes / comma
    ".ltm.virtual[] | .name",
    ".ltm.virtual[] | .pool",
    "a, b",
    ".a, .b | .c",
    # literals
    "5",
    "3.5",
    "-7",
    '"hello"',
    '"tab\\tnewline\\n\\"quote\\\\"',
    "true",
    "false",
    "null",
    # variables
    "$ltm",
    "$gtm.gtm.wideip[]",
    "$ltm.ltm.virtual[].name",
    # calls
    "length",
    "not",
    "f()",
    'select(.a == "x")',
    "f(a, b)",
    "f((a, b))",
    "f(.x | basename(.))",
    "in_cidr(., \"10.0.0.0/8\")",
    # binops / unary / precedence
    "1 + 2 * 3",
    "1 * 2 + 3",
    "1 - 2 - 3",
    ".a == .b and .c != .d",
    ".a or .b and .c",
    "not .a",
    ".x | not",
    "(1 | not) - 1",
    ".a <= 3",
    ".a >= .b",
    "- .x",
    # object / list literals
    "{name, destination, pool}",
    '{a: .x, "b-c": .y}',
    "{}",
    "[.ltm.virtual[].name]",
    "[]",
    "[.a, .b, .c]",
    # assignments
    '.ltm.pool["/Common/old"].name = "/Common/new"',
    '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)',
    '.ltm.virtual[] | .rules += "/Common/log_rule"',
    ".x -= 1",
    '$ltm.ltm.virtual["/Common/vs_app"].destination = "/Common/192.168.1.1:443"',
    ". = 1",
    # let binding
    ".ltm.virtual[] as $vs | $vs.pool.members[] | $vs.name",
    # if/then/else
    'if type == "string" then ascii_downcase else . end',
    "if .a then 1 elif .b then 2 else 3 end",
    "if .a then 1 end",
    # head paths (primary-with-postfix)
    "refs(.)[]",
    "f(.).y[0]",
    # semicolons
    ".a ; .b",
    ".a ; .b ;",
    # comments
    "# comment\n.name",
    # the cookbook examples
    ".ltm.virtual[] | .pool",
    '.ltm.virtual["~/vs_prod_"] | .name',
    '.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name',
    '.ltm.virtual[] | select(any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))) | .name',
    '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)',
    '.ltm.pool["/Common/old_pool"].name = "/Common/new_pool"',
    '.ltm.virtual[] | select(not contains(.rules, "/Common/log_rule")) | .rules += "/Common/log_rule"',
    '.ltm.rule[] | select(contains(.refs.pools, "/Common/old_pool")) | .name',
    ".ltm.virtual[].pool |= basename(.)",
    'rename("/Common/old_pool", "/Common/new_pool")',
    'rename_partition("Tenant_A", "Tenant_B")',
    '.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")',
    ".ltm.virtual[] | .destination |= with_route_domain(., 7)",
    '[.ltm.virtual[]] | group_by(partition(."full-path")) | map({partition: (.[0]."full-path" | partition(.)), count: length})',
    '.ltm.virtual[] | select(contains(.name, "_dev_")) | .destination |= sub(., ":[0-9]+$", ":0")',
    "$ltm.ltm.virtual[].name",
    ".ltm.pool[] | referenced_by(.)",
    "[.ltm.virtual[].pool] | dupes",
    "[.ltm.virtual[]] | sort_by(.pool.members | length) | map(.name)",
    "[.ltm.pool[]] | max_by(.members | length)",
    '.ltm.virtual.web_vs | walk(if type == "string" then ascii_downcase else . end)',
    "[.ltm.virtual[]] | INDEX(.name)",
]

# Malformed inputs: each must raise a LexError or ParseError.  We
# capture the message + offset so the Rust error types stay parity.
ERROR_QUERIES: list[str] = [
    '"unterminated',
    "!",
    "!x",
    "$",
    "$ ",
    "@",
    "1 +",
    ".a (",
    "(.a",
    "[.a",
    "{",
    "{a:",
    "{1: .x}",
    ".a as .b | .c",
    ".a as $b .c",
    "1 = 2",
    "f(.a",
    "if .a then 1",
    "if .a 1 end",
    "select(.a,)",
    ".ltm.virtual[",
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "front_end.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    tokens_cases = []
    ast_cases = []
    for q in QUERIES:
        tokens_cases.append({"query": q, "tokens": [token_json(t) for t in tokenise(q)]})
        ast_cases.append({"query": q, "ast": node_json(parse_query(q))})

    error_cases = []
    for q in ERROR_QUERIES:
        try:
            parse_query(q)
        except LexError as exc:
            error_cases.append(
                {"query": q, "error": "lex", "message": exc.message, "offset": exc.offset}
            )
            continue
        except ParseError as exc:
            error_cases.append(
                {"query": q, "error": "parse", "message": exc.message, "offset": exc.offset}
            )
            continue
        raise SystemExit(f"expected an error for {q!r} but parse_query succeeded")

    payload = {
        "tokens": tokens_cases,
        "ast": ast_cases,
        "errors": error_cases,
    }
    out_path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(f"wrote {out_path} ({len(QUERIES)} queries, {len(ERROR_QUERIES)} error cases)")


if __name__ == "__main__":
    main()
