#!/usr/bin/env python3
"""Golden fixtures for the Rust query *time/date-category builtins*.

For a list of `query` strings (each over a `null` JSON root, mode ``json``)
runs the Python query DSL end to end — parse → evaluate → ``output.render``
— and records the rendered string (or the ``error:`` message). The Rust
``time_parity`` test mirrors the same pipeline and asserts byte-equality, so
the time builtins stay byte-faithful to the Python source. Self-contained:
the fixtures are captured once here and checked in.

``TZ=UTC`` is pinned BEFORE importing the DSL so the ``localtime`` /
``mktime`` / ``strftime`` builtins (which follow the process timezone) are
deterministic. The non-deterministic ``now`` builtin is deliberately
excluded — the Rust side asserts its type/range in a unit test instead.

Run from the repo root:

    PYTHONPATH=. TZ=UTC python3 scripts/codegen/gen_f5_query_time_fixtures.py
"""

from __future__ import annotations

import os
import time

os.environ["TZ"] = "UTC"
time.tzset()

import json  # noqa: E402
from pathlib import Path  # noqa: E402

from dialects.f5.bigip.model import BigipConfig  # noqa: E402
from dialects.f5.query.errors import QueryError  # noqa: E402
from dialects.f5.query.evaluator import EvalContext, evaluate  # noqa: E402
from dialects.f5.query.output import render  # noqa: E402
from dialects.f5.query.parser import parse_query  # noqa: E402
from dialects.f5.query.source_map import SourceMap  # noqa: E402
from dialects.f5.query.values import Root  # noqa: E402


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


# Every time builtin ported into `builtins/time_dt.rs` EXCEPT the
# non-deterministic `now` (covered by a Rust unit test). Fixed input
# timestamps (1700000000 == "2023-11-14T22:13:20Z") and ISO strings keep
# every case deterministic; `strftime`/`strptime` only use specifiers that
# are byte-identical between Python's C `strftime` and chrono
# (`%Y %m %d %H %M %S %j %w %A %a %B %b %p %U %W %%`). Root is always `null`.
QUERIES: list[str] = [
    # todate / todateiso8601 / date (UTC, second precision)
    "todate(0)",
    "todate(1700000000)",
    "todate(1700000000.9)",
    "todate(-1)",
    "todate(1000000000)",
    "todateiso8601(0)",
    "todateiso8601(1700000000)",
    "date(0)",
    "date(1700000000)",
    "todate(true)",
    'todate("notanumber")',
    # fromdate / fromdateiso8601 (ISO-8601 UTC → float epoch)
    'fromdate("1970-01-01T00:00:00Z")',
    'fromdate("2023-11-14T22:13:20Z")',
    'fromdate("2023-11-14T22:13:20+00:00")',
    'fromdate("2023-11-14T22:13:20.5Z")',
    'fromdate("2023-11-14T22:13:20.123456Z")',
    'fromdate("2023-11-14T22:13")',
    'fromdate("2023-11-14")',
    'fromdate("2023-11-14 22:13:20")',
    'fromdateiso8601("1970-01-01T00:00:00Z")',
    'fromdateiso8601("2023-11-14T22:13:20Z")',
    'fromdate("notadate")',
    'fromdate("garbage stuff")',
    "fromdate(42)",
    "fromdate(true)",
    # gmtime (UTC broken-down array)
    "gmtime(0)",
    "gmtime(1700000000)",
    "gmtime(1700000000.9)",
    "gmtime(1000000000)",
    "gmtime(-1)",
    'gmtime("x")',
    # localtime (TZ=UTC → equals gmtime)
    "localtime(0)",
    "localtime(1700000000)",
    "localtime(1000000000)",
    # mktime (inverse of gmtime, float result)
    "mktime([70, 0, 1, 0, 0, 0])",
    "mktime([123, 10, 14, 22, 13, 20])",
    "mktime([100, 0, 1, 0, 0, 0])",
    "mktime([70, 0, 1, 0, 0, 30.9])",
    "mktime([123, 10, 14, 22, 13, 20, 2, 317])",
    "mktime([70, 0, 1])",
    'mktime("x")',
    "mktime([1, 2, 3, 4, 5])",
    "mktime(42)",
    # strftime (UTC; byte-identical specifiers only)
    'strftime(0, "%Y-%m-%d")',
    'strftime(1700000000, "%Y-%m-%dT%H:%M:%S")',
    'strftime(1700000000, "%j %w %A %a %B %b %p")',
    'strftime(1700000000, "%H%% %U %W")',
    'strftime(1000000000, "%Y/%m/%d %H:%M")',
    'strftime(1700000000.9, "%Y-%m-%dT%H:%M:%S")',
    'strftime(true, "%Y")',
    "strftime(0, 42)",
    # strptime (→ broken-down array)
    'strptime("2025-01-01", "%Y-%m-%d")',
    'strptime("2023-11-14 22:13:20", "%Y-%m-%d %H:%M:%S")',
    'strptime("1970-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S")',
    'strptime("bad", "%Y-%m-%d")',
    'strptime(42, "%Y-%m-%d")',
    # dateadd / datesub (int-preserving arithmetic)
    "dateadd(1700000000, 86400)",
    "datesub(1700000000, 3600)",
    "dateadd(1.5, 2)",
    "datesub(10, 4)",
    "dateadd(1700000000, -1700000000)",
    "datesub(1700000000.5, 0.5)",
    "dateadd(0, 0)",
    "dateadd(true, 1)",
    'datesub(1, "x")',
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "time.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, None, "json") for q in QUERIES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
