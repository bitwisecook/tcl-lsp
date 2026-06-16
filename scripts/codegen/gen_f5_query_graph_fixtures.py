#!/usr/bin/env python3
"""Golden fixtures for the Rust query *graph* layer over a real config.

Like ``gen_f5_query_projection_fixtures.py``, this builds a **BIG-IP**
``Root`` from the real fixture config
``rust/tcl-bigip/tests/fixtures/bigip.conf`` and runs a matrix of
config-rooted queries through the Python DSL end to end — parse → evaluate
against the projected ``Container`` tree (which walks the reference graph for
``refs`` / ``referenced_by`` / ``references_to`` / ``check_partition_visibility``
and the synthesised rule ``.refs`` sub-object) → ``output.render`` — recording
the rendered string (or the error message). The Rust ``graph_parity`` test
mirrors the same pipeline and asserts byte equality, so the graph-backed
builtins and the rule ``.refs`` projection stay faithful to Python.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_graph_fixtures.py
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.query.errors import QueryError
from dialects.f5.query.evaluator import EvalContext, evaluate
from dialects.f5.query.output import render
from dialects.f5.query.parser import parse_query
from dialects.f5.query.runner import _active_root, _build_root

# The fixture config the Rust test parses via `parse_bigip_conf`.
FIXTURE = (
    Path(__file__).resolve().parents[2]
    / "rust"
    / "tcl-bigip"
    / "tests"
    / "fixtures"
    / "bigip.conf"
)
URI = "bigip.conf"

# (query, mode). Every query roots at the BIG-IP `<root>` container and
# exercises the graph-backed reference surface.
CASES: list[tuple[str, str]] = [
    # ---- rule .refs projection: whole sub-object + each slot ----------
    (".ltm.rule[].refs", "json"),
    (".ltm.rule[].refs.pools", "raw"),
    (".ltm.rule[] | .refs.pools", "raw"),
    (".ltm.rule[].refs.persists", "raw"),
    (".ltm.rule[].refs.data-groups", "raw"),
    (".ltm.rule[] | {name, refs}", "json"),
    (".ltm.rule[] | {name, pools: .refs.pools}", "json"),
    ('.ltm.rule["/Common/route_by_host"].refs', "json"),
    ('.ltm.rule["/Common/route_by_host"].refs.data-groups', "raw"),
    # ---- cookbook example #8 (no real pool ⇒ empty, but must run) -----
    (
        '.ltm.rule[] | select(contains(.refs.pools, "/Common/old_pool")) | .name',
        "raw",
    ),
    (
        '.ltm.rule[] | select(contains(.refs.data-groups, "/Common/blocked_ips")) | .name',
        "raw",
    ),
    # ---- refs(.) forward edges, per kind ------------------------------
    (".ltm.virtual[] | refs(.)", "raw"),
    ('.ltm.virtual["/Common/www_vs"] | refs(.)', "raw"),
    (".ltm.rule[] | refs(.)", "raw"),
    (".ltm.pool[] | refs(.)", "raw"),
    ("refs(.ltm.virtual[])", "raw"),
    ("[refs(.ltm.virtual[])] | length", "json"),
    (".ltm.virtual[] | refs(.) | sort", "raw"),
    # ---- referenced_by(.) reverse edges, per kind ---------------------
    (".ltm.pool[] | referenced_by(.)", "raw"),
    (".ltm.virtual[] | referenced_by(.)", "raw"),
    (".ltm.monitor[] | referenced_by(.)", "raw"),
    (".ltm.rule[] | referenced_by(.)", "raw"),
    ("referenced_by(.ltm.pool[])", "raw"),
    # orphan-pool cleanup idiom
    (".ltm.pool[] | select(referenced_by(.) | count == 0) | .name", "raw"),
    # ---- references_to(path) reverse-by-string ------------------------
    ('references_to("/Common/web_pool")', "raw"),
    ('references_to("/Common/api_pool")', "raw"),
    ('references_to("/Common/route_by_host")', "raw"),
    ('references_to("")', "json"),
    ('references_to("/Common/does_not_exist")', "raw"),
    ('count(references_to("/Common/web_pool"))', "json"),
    # ---- check_partition_visibility() (single-partition ⇒ clean) ------
    ("check_partition_visibility()", "json"),
    ("count(check_partition_visibility())", "json"),
    # ---- type / stream error cases ------------------------------------
    ("refs(.ltm.pool[].name)", "raw"),
    ("referenced_by(.ltm.virtual[].name)", "raw"),
    ("references_to(.ltm.pool[].full-path)", "raw"),
]


def run(query: str, mode: str, source: str) -> dict:
    root = _build_root(URI, source)
    ctx = EvalContext(root=root)
    try:
        prog = parse_query(query)
        # `refs` / `referenced_by` resolve the originating config through the
        # runner's active-root context, so wrap evaluation the way
        # `run_query` does for a single file.
        with _active_root(root):
            values = evaluate(prog, ctx)
            out = render(values, mode=mode)
        return {"query": query, "mode": mode, "kind": "ok", "output": out}
    except (QueryError, ValueError) as exc:
        return {"query": query, "mode": mode, "kind": "err", "output": str(exc)}


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = (
        repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "graph.json"
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    source = FIXTURE.read_text(encoding="utf-8")
    cases = [run(q, mode, source) for (q, mode) in CASES]
    out_path.write_text(
        json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
    )
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
