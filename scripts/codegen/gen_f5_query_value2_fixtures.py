#!/usr/bin/env python3
"""Golden fixtures for the Rust query *value/stream/path + encoding builtins*.

For a matrix of `(query, input, mode)` triples, runs the Python query DSL
end to end — parse → evaluate against a JSON-backed `Root` → ``output.render``
— and records the rendered string (or the ``error:`` message). The Rust
``value2_parity`` test mirrors the same pipeline and asserts byte-equality,
so the value2 (`defined` / `in` / `kind` / `path` / `json_parse` / `all` /
`any` / `halt` / `halt_error` / `inside` / `limit` / `nth` / `basename` /
`partition` / `with_partition`) and encoding (`base64` / `base64d` / `html` /
`uri` / `tojson` / `fromjson` / `sh`) builtins stay byte-faithful. Self-
contained: the fixtures are captured once here and checked in.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_value2_fixtures.py
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.bigip.model import BigipConfig
from dialects.f5.query.errors import QueryError
from dialects.f5.query.evaluator import EvalContext, evaluate
from dialects.f5.query.output import render
from dialects.f5.query.parser import parse_query
from dialects.f5.query.source_map import SourceMap
from dialects.f5.query.values import Root


def run(query: str, data: object, mode: str) -> dict:
    root = Root(
        uri="data.json",
        source="",
        config=BigipConfig(),
        source_map=SourceMap.build(""),
        json_value=data,
    )
    ctx = EvalContext(root=root)
    try:
        prog = parse_query(query)
        values = evaluate(prog, ctx)
        out = render(values, mode=mode)
        return {"query": query, "input": data, "mode": mode, "kind": "ok", "output": out}
    except (QueryError, ValueError) as exc:
        return {"query": query, "input": data, "mode": mode, "kind": "err", "output": str(exc)}


# Each entry is (query, input, mode). Root JSON values feed `.`; queries that
# need a concrete literal embed it. `kind` / `path` are only reachable on the
# error branch over a JSON root (they require an ObjectRef / PathRef, which a
# JSON-backed Root never produces) — covered as errors. `json_parse` /
# `fromjson` error wording differs from CPython, so only success cases for the
# parsers are captured here.
CASES: list[tuple[str, object, str]] = [
    # -- defined (value) — literal args avoid the `.`-over-null root Container --
    ("defined(null)", None, "json"),
    ('defined("")', None, "json"),
    ('defined("x")', None, "json"),
    ("defined(false)", None, "json"),
    ("defined(0)", None, "json"),
    ("defined([])", None, "json"),
    ('defined({"a": 1})', None, "json"),
    ("defined(.)", "x", "json"),
    ("defined(.)", "", "json"),
    # -- in (value) — inverse of has --
    ('"a" | in({"a": 1, "b": 2})', None, "json"),
    ('"c" | in({"a": 1})', None, "json"),
    ("1 | in([10, 20, 30])", None, "json"),
    ("5 | in([10, 20])", None, "json"),
    ('in({"name": 1})', "name", "json"),
    ("in([1, 2, 3])", 0, "json"),
    ("in(null)", "k", "json"),
    ('in("string")', "k", "json"),
    ("in([1, 2])", "notanint", "json"),
    # -- kind / path (value) — error branches over JSON root --
    ("kind(.)", {"name": "web"}, "json"),
    ('kind("x")', None, "json"),
    ("kind(42)", None, "json"),
    ("path(.)", {"name": "web"}, "json"),
    ("path(42)", None, "json"),
    ('path("x")', None, "json"),
    # -- json_parse (value) --
    ('json_parse("{\\"a\\": 1, \\"b\\": [2, 3]}")', None, "json"),
    ('json_parse("[1, 2.5, true, null, \\"s\\"]")', None, "json"),
    ('json_parse("42")', None, "json"),
    ('json_parse("\\"hi\\"")', None, "json"),
    ('json_parse("{\\"z\\": 1, \\"a\\": 2}") | keys_unsorted', None, "json"),
    ("json_parse(42)", None, "json"),
    # -- all / any (stream) --
    ("all", [True, True, 1, "x"], "json"),
    ("all", [True, False], "json"),
    ("all", [], "json"),
    ("any", [False, 0, "", None], "json"),
    ("any", [False, True], "json"),
    ("any", [], "json"),
    # flattened one level: [[a],[b]] -> [a,b]
    ("all", [[True], [True]], "json"),
    ("any", [[False], [True]], "json"),
    ("all", [[True], [False]], "json"),
    ("all", 42, "json"),
    ("any", "notalist", "json"),
    # -- halt / halt_error (stream) — always error --
    ("halt", None, "json"),
    ("halt_error", None, "json"),
    ("halt_error(7)", None, "json"),
    ("halt_error(0)", None, "json"),
    ('halt_error("notanint")', None, "json"),
    # -- inside (stream) — inverse of contains --
    ('"ell" | inside("hello")', None, "json"),
    ('"xyz" | inside("hello")', None, "json"),
    ("2 | inside([1, 2, 3])", None, "json"),
    ("9 | inside([1, 2, 3])", None, "json"),
    ('inside("hello world")', "world", "json"),
    ("inside([1, 2, 3])", 2, "json"),
    ("inside(42)", "x", "json"),
    ("inside([1, 2])", 3, "json"),
    # -- limit (stream) --
    ("limit(., 2)", [1, 2, 3, 4], "json"),
    ("limit(., 0)", [1, 2, 3], "json"),
    ("limit(., -1)", [1, 2, 3], "json"),
    ("limit(., 10)", [1, 2], "json"),
    ("limit(42, 2)", None, "json"),
    ('limit([1, 2], "x")', None, "json"),
    # -- nth (stream) --
    ("nth(., 0)", [10, 20, 30], "json"),
    ("nth(., 2)", [10, 20, 30], "json"),
    ("nth(., -1)", [10, 20, 30], "json"),
    ("nth(., 5)", [10, 20, 30], "json"),
    ("nth(., -5)", [10, 20, 30], "json"),
    ("nth(42, 0)", None, "json"),
    ('nth([1], "x")', None, "json"),
    # -- basename (path) --
    ('basename("/Common/web_pool")', None, "json"),
    ('basename("/Common/iApps/T.app/pool_1")', None, "json"),
    ('basename("barename")', None, "json"),
    ('basename("/")', None, "json"),
    ('basename("")', None, "json"),
    ("basename(42)", None, "json"),
    # -- partition (path) --
    ('partition("/Common/web_pool")', None, "json"),
    ('partition("/Tenant_A/iApps/T.app/p")', None, "json"),
    ('partition("relativename")', None, "json"),
    ('partition("/")', None, "json"),
    ('partition("/Common")', None, "json"),
    ("partition(42)", None, "json"),
    # -- with_partition (path) --
    ('with_partition("/Common/web_pool", "Tenant_A")', None, "json"),
    ('with_partition("barename", "Common")', None, "json"),
    ('with_partition("/Common/iApps/T.app/p1", "X")', None, "json"),
    ('with_partition("/Common/web_pool", "")', None, "json"),
    ("with_partition(42, \"Common\")", None, "json"),
    # ====================== encoding ======================
    # -- base64 / base64d --
    ('base64("hello")', None, "json"),
    ('base64("")', None, "json"),
    ('base64("foob")', None, "json"),
    ('base64("résumé")', None, "json"),
    ('base64d("aGVsbG8=")', None, "json"),
    ('base64d("")', None, "json"),
    ('base64("hello") | base64d', None, "json"),
    ("base64(42)", None, "json"),
    # -- html --
    ('html("<a href=\\"x\\">&\'</a>")', None, "json"),
    ('html("plain text")', None, "json"),
    ("html(42)", None, "json"),
    # -- uri --
    ('uri("hello world")', None, "json"),
    ('uri("a/b?c=d&e=f")', None, "json"),
    ('uri("résumé")', None, "json"),
    ('uri("safe-_.~chars")', None, "json"),
    ("uri(42)", None, "json"),
    # -- tojson --
    ("tojson", {"z": 1, "a": [2, 3]}, "json"),
    ("tojson", [1, "x", True, None], "json"),
    ("tojson", "a string", "json"),
    ("tojson", 42, "json"),
    # -- fromjson --
    ('fromjson("{\\"a\\": 1, \\"b\\": 2}")', None, "json"),
    ('fromjson("[1, 2, 3]")', None, "json"),
    ('fromjson("true")', None, "json"),
    ('fromjson("3.14")', None, "json"),
    ("fromjson(42)", None, "json"),
    # -- sh --
    ('sh("simple")', None, "json"),
    ("sh(\"it's a test\")", None, "json"),
    ('sh("a b c")', None, "json"),
    ("sh", ["a", "b c", "it's"], "json"),
    ("sh", [], "json"),
    ("sh(42)", None, "json"),
    ("sh", [1, 2], "json"),
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "value2.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, data, mode) for (q, data, mode) in CASES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
