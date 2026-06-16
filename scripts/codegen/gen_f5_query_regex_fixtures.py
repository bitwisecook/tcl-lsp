#!/usr/bin/env python3
"""Golden fixtures for the Rust query *regex / string-category builtins*.

For a list of `query` strings (each over a `null` JSON root, mode ``json``)
runs the Python query DSL end to end — parse → evaluate → ``output.render``
— and records the rendered string (or the ``error:`` message). The Rust
``regex_str_parity`` test mirrors the same pipeline and asserts byte-equality,
so the regex/string builtins stay byte-faithful to the Python source.
Self-contained: the fixtures are captured once here and checked in.

Covers ``match`` / ``sub`` / ``gsub`` / ``test`` / ``scan`` / ``capture`` /
``splits`` / ``index`` / ``ltrimstr`` / ``rtrimstr`` / ``explode`` /
``implode`` / ``ascii`` / ``utf8bytelength`` / ``csv`` / ``tsv``.

Inputs are kept ASCII so the ``regex`` crate's byte offsets coincide with
Python ``re``'s code-point offsets, and the regex patterns stay in the
subset both engines agree on (literals, char classes, anchors, groups,
``+ * ? { }``, alternation, named groups).  ``splits`` patterns avoid
capturing groups (Python ``re.split`` would interleave the captures, which
the ``regex`` crate's ``split`` does not).

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_regex_fixtures.py
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


QUERIES: list[str] = [
    # match (boolean predicate — this DSL's match is jq's test)
    'match("vs_prod_web", "^vs_")',
    'match("api_vs", "^vs_")',
    'match("a1b2c3", "[0-9]")',
    'match("hello", "x+")',
    # test (with flags)
    'test("vs_prod", "^vs_")',
    'test("VS_PROD", "^vs_")',
    'test("VS_PROD", "^vs_", "i")',
    'test("abc", "B", "i")',
    'test("one\\ntwo", "^two$", "m")',
    'test("a", "")',
    # sub (first match, with and without backrefs)
    'sub("1.2.3.4:443", ":443$", ":8443")',
    'sub("user@host", "(\\\\w+)@(\\\\w+)", "\\\\2.\\\\1")',
    'sub("foo-bar", "(?P<a>\\\\w+)-(?P<b>\\\\w+)", "\\\\g<b>_\\\\g<a>")',
    'sub("aaa", "a", "b")',
    'sub("VS_dev_web", "^VS_", "vs_", "i")',
    'sub("nomatch", "xyz", "q")',
    # gsub (every match, with and without backrefs)
    'gsub("a.b.c.d", "\\\\.", "-")',
    'gsub("/Common/old_x /Common/old_y", "old_", "new_")',
    'gsub("a1b2c3", "([a-z])([0-9])", "\\\\2\\\\1")',
    'gsub("Mix MIX mix", "mix", "X", "i")',
    'gsub("hello", "z", "Q")',
    # scan (no groups vs groups)
    'scan("a1 b22 c333", "[0-9]+")',
    'scan("a=1 b=2 c=3", "([a-z])=([0-9])")',
    'scan("nope", "[0-9]+")',
    'scan("ABC", "[a-z]+", "i")',
    # capture (named groups)
    'capture("vs_prod_web", "(?<env>prod|dev|qa)_(?<app>.+)")',
    'capture("10.0.0.1:80", "(?<addr>[0-9.]+):(?<port>[0-9]+)")',
    'capture("abc", "(?P<x>a)(?P<y>b)")',
    'capture("foobar", "(?P<g>[0-9]+)")',
    'capture("XYZ", "(?<low>[a-z]+)", "i")',
    # splits (regex separator, no capture groups)
    'splits("a, b ,c,  d", " *, *")',
    'splits("v1.2.3-rc4", "[.-]")',
    'splits("abc", "x")',
    # index (string substring + list element)
    'index("/Common/vs_443", ":443")',
    'index("host:443:x", ":443")',
    'index("nope", "zzz")',
    'index(["http", "tcp", "ssl"], "ssl")',
    'index(["http", "tcp"], "ssl")',
    # ltrimstr / rtrimstr (matching + non-matching)
    'ltrimstr("vs_prod_web", "vs_")',
    'ltrimstr("api_vs", "vs_")',
    'ltrimstr("anything", "")',
    'rtrimstr("web_pool", "_pool")',
    'rtrimstr("web_pool", "_xxx")',
    'rtrimstr("web_pool", "")',
    # explode / implode (round trip)
    'explode("abc")',
    'explode("")',
    'implode([97, 98, 99])',
    'implode([72, 105])',
    'implode([])',
    # ascii
    'ascii(65)',
    'ascii(97)',
    'ascii(48)',
    # utf8bytelength
    'utf8bytelength("hello")',
    'utf8bytelength("")',
    'utf8bytelength("a b c")',
    # csv / tsv rows
    'csv("a", "b", "c")',
    'csv("plain", "has,comma", "has\\"quote")',
    'csv("a", "", "c")',
    'csv("name", 42, true)',
    'csv(null, "x")',
    'tsv("a", "b", "c")',
    'tsv("name", 80, false)',
    'tsv(null, "x")',
    # error cases (wrong-type args + unsupported flag).
    #
    # The regex *guard* errors (nested-quantifier ``(a+)+`` / over-length) and
    # the invalid-pattern error are intentionally NOT covered: the parent
    # ``safe_regex_compile`` interpolates the pattern with Rust's ``{:?}``
    # (double quotes) where Python ``re`` uses ``repr`` (single quotes), and
    # the invalid-pattern reason comes from the ``regex`` crate, not Python
    # ``re`` — neither can be made byte-identical without touching the shared
    # chokepoint, so they are excluded per the divergence policy.
    'test("x", "bad", "z")',
    'match(42, "x")',
    'sub(42, "a", "b")',
    'implode(["a"])',
    'implode([-1])',
    'implode([1114112])',
    'ascii(-1)',
    'ascii(1114112)',
    'index(42, "x")',
    'explode(42)',
    'utf8bytelength(42)',
    'csv({a: 1})',
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "regex_str.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, None, "json") for q in QUERIES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
