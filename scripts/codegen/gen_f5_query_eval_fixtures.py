#!/usr/bin/env python3
"""Golden fixtures for the Rust query *evaluator* + builtins over JSON roots.

For a matrix of `(query, input, mode)` triples, runs the Python query DSL
end to end — parse → evaluate against a JSON-backed `Root` → `output.render`
— and records the rendered string (or the `error:` message). The Rust
`eval_parity` test mirrors the same pipeline (`parse_query` →
`evaluate(Root::json)` → `output::render`) and asserts equality, so the
evaluator + builtin value-paths stay byte-faithful. Self-contained: the
fixtures are captured once here and checked in.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_eval_fixtures.py
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


# Reusable inputs.
NUMS = [3, 1, 2, 1, 5]
OBJ = {"name": "web", "port": 80, "tags": ["a", "b"], "nested": {"x": 1}}
PEOPLE = [
    {"name": "alice", "age": 30, "team": "blue"},
    {"name": "bob", "age": 25, "team": "red"},
    {"name": "carol", "age": 30, "team": "blue"},
]
MIXED = [None, True, 2, "x", [1], {"k": 1}]

# (query, input, mode)
CASES: list[tuple[str, object, str]] = [
    # identity / field / index
    (".", OBJ, "json"),
    (".name", OBJ, "json"),
    (".nested.x", OBJ, "json"),
    (".tags[0]", OBJ, "json"),
    (".tags[-1]", OBJ, "json"),
    (".tags[]", OBJ, "json"),
    (".[]", NUMS, "json"),
    (".[1]", NUMS, "json"),
    # pipes
    (".tags | length", OBJ, "json"),
    (".[] | . + 1", NUMS, "json"),
    # value/stream builtins
    ("length", NUMS, "json"),
    ("length", OBJ, "json"),
    ("keys", OBJ, "json"),
    ("keys_unsorted", OBJ, "json"),
    ("values", OBJ, "json"),
    ("type", OBJ, "json"),
    ("type", NUMS, "json"),
    ('has("name")', OBJ, "json"),
    ('has("missing")', OBJ, "json"),
    ("add", NUMS, "json"),
    ("add", [["a"], ["b", "c"]], "json"),
    ("sort", NUMS, "json"),
    ("unique", NUMS, "json"),
    ("dupes", NUMS, "json"),
    ("reverse", NUMS, "json"),
    ("first", NUMS, "json"),
    ("last", NUMS, "json"),
    ("count", NUMS, "json"),
    ("min", NUMS, "json"),
    ("max", NUMS, "json"),
    ("range(5)", None, "json"),
    ("range(1, 10, 2)", None, "json"),
    ("range(5, 0, -2)", None, "json"),
    ("empty", NUMS, "json"),
    # arithmetic / comparison / boolean
    ("1 + 2 * 3", None, "json"),
    ("10 / 4", None, "json"),
    ("7 - 3", None, "json"),
    (".port + 1", OBJ, "json"),
    ('.name + "!"', OBJ, "json"),
    ('"n=" + (.port | tostring)', OBJ, "json"),
    (".port > 50", OBJ, "json"),
    (".port == 80 and .name == \"web\"", OBJ, "json"),
    ("not", True, "json"),
    ("-.port", OBJ, "json"),
    ("[1,2,3] - [2]", None, "json"),
    # select / map
    (".[] | select(. > 1)", NUMS, "json"),
    ("map(. + 1)", NUMS, "json"),
    ("map(. * 2)", NUMS, "json"),
    ("[.[] | select(. > 1)]", NUMS, "json"),
    # sort_by / group_by / unique_by / min_by / max_by
    ("sort_by(.age)", PEOPLE, "json"),
    ("group_by(.team)", PEOPLE, "json"),
    ("unique_by(.team)", PEOPLE, "json"),
    ("min_by(.age)", PEOPLE, "json"),
    ("max_by(.age)", PEOPLE, "json"),
    ("min_max(.age)", PEOPLE, "json"),
    ("map(.name)", PEOPLE, "json"),
    # object / list literals
    ("{a: .port, b: .name}", OBJ, "json"),
    ("{name}", OBJ, "json"),
    ("{}", OBJ, "json"),
    ("[.[] | . * 10]", NUMS, "json"),
    ("[]", None, "json"),
    # comma stream
    (".name, .port", OBJ, "json"),
    # if/then/else
    ('if .port > 50 then "big" else "small" end', OBJ, "json"),
    ('.[] | if . > 2 then "hi" elif . > 1 then "mid" else "lo" end', NUMS, "json"),
    # entries
    ("to_entries", {"a": 1, "b": 2}, "json"),
    ("from_entries", [{"key": "a", "value": 1}, {"key": "b", "value": 2}], "json"),
    ("with_entries(.value += 1)", {"a": 1, "b": 2}, "json"),
    # flatten / paths
    ("flatten", [[1, 2], [3, [4]]], "json"),
    ("flatten(2)", [[1, 2], [3, [4]]], "json"),
    ("[paths]", {"a": 1, "b": [2, 3]}, "json"),
    ("[leaf_paths]", {"a": 1, "b": [2, 3]}, "json"),
    ('getpath(["nested", "x"])', OBJ, "json"),
    ('setpath(["port"]; 0)'.replace(";", ","), OBJ, "json"),
    ('del(.port)'.replace("del(.port)", 'del(["port"])'), OBJ, "json"),
    # walk / recurse
    ('walk(if type == "string" then ascii_upcase else . end)', OBJ, "json"),
    ("[recurse]", {"a": {"b": 1}}, "json"),
    # string builtins
    ('startswith("web")', "webserver", "json"),
    ('endswith("ver")', "webserver", "json"),
    ('contains("bs")', "webserver", "json"),
    ('split(",")', "a,b,c", "json"),
    ('join("-")', ["a", "b", "c"], "json"),
    ("ascii_upcase", "abc", "json"),
    ("downcase", "ABC", "json"),
    # number conversion / math
    ("tonumber", "42", "json"),
    ("tonumber", "3.5", "json"),
    ("tostring", 42, "json"),
    ("tostring", [1, 2], "json"),
    ("floor", 2.7, "json"),
    ("ceil", 2.1, "json"),
    ("abs", -5, "json"),
    ("abs", -2.5, "json"),
    # IN / INDEX
    ("IN(1, 2, 3)", 2, "json"),
    ("IN(1, 2, 3)", 9, "json"),
    ("INDEX(.name)", PEOPLE, "json"),
    # let binding
    (". as $x | $x.port", OBJ, "json"),
    (".tags[] as $t | $t + \"!\"", OBJ, "json"),
    # combinations
    ("combinations", [[1, 2], ["a", "b"]], "json"),
    # raw / auto output modes
    (".[]", NUMS, "raw"),
    (".name", OBJ, "raw"),
    ("[.tags[]]", OBJ, "auto"),
    (".[] | select(. > 1)", NUMS, "raw"),
    # mixed-type jq ordering
    ("sort", MIXED, "json"),
    ("unique", [3, 1, 2, 1], "json"),
    # error cases
    (".a + .b", {"a": 1, "b": "x"}, "json"),
    ("keys", 5, "json"),
    ("length", 5, "json"),
    ("nope(.)", OBJ, "json"),
    ('.missing', OBJ, "json"),
    ("1 / 0", None, "json"),
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "eval.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, data, mode) for (q, data, mode) in CASES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
