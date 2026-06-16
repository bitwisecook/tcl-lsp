#!/usr/bin/env python3
"""Golden fixtures for the Rust query *projection* layer over a real config.

Unlike the other query generators (which root the DSL at an external-JSON
value), this one builds a **BIG-IP** ``Root`` from the real fixture config
``rust/tcl-bigip/tests/fixtures/bigip.conf`` and runs a matrix of
config-rooted queries through the Python DSL end to end — parse → evaluate
against the projected ``Container`` tree → ``output.render`` — recording the
rendered string (or the error message). The Rust ``projection_parity`` test
mirrors the same pipeline (``parse_query`` →
``evaluate(Root::bigip(...))`` → ``output::render``) and asserts byte
equality, so the projection layer (``Container`` / ``ObjectRef`` /
``PathRef`` navigation over the typed ``tcl-bigip`` model) stays faithful.

The query matrix deliberately stays on the fields the Rust projection
covers byte-for-byte (the core LTM kinds). It avoids ``--raw`` over the
handful of structured typed values (``MonitorExpression`` / ``ProfileType``
enum) that the Python DSL refuses to render as scalars but the Rust port
projects as plain strings — those navigations are out of scope for this
read-only increment.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_projection_fixtures.py
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.query.errors import QueryError
from dialects.f5.query.evaluator import EvalContext, evaluate
from dialects.f5.query.output import render
from dialects.f5.query.parser import parse_query
from dialects.f5.query.runner import _build_root

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

# (query, mode). Every query roots at the BIG-IP `<root>` container.
CASES: list[tuple[str, str]] = [
    # ---- kind iteration + .name across the covered kinds --------------
    (".ltm.virtual[].name", "json"),
    (".ltm.virtual[].name", "raw"),
    (".ltm.virtual[].name", "auto"),
    (".ltm.pool[].name", "raw"),
    (".ltm.node[].name", "raw"),
    (".ltm.rule[].name", "raw"),
    (".ltm.monitor[].name", "raw"),
    (".ltm.profile[].name", "raw"),
    (".ltm.persistence[].name", "raw"),
    (".ltm.snatpool[].name", "raw"),
    (".ltm.data-group[].name", "raw"),
    (".ltm.virtual[].full-path", "raw"),
    # ---- scalar / typed field projection ------------------------------
    (".ltm.virtual[].destination", "raw"),
    (".ltm.virtual[].destination", "scf"),
    (".ltm.virtual[] | .pool", "json"),
    (".ltm.node[].address", "raw"),
    (".ltm.virtual[] | .source-address-translation", "raw"),
    (".ltm.pool[] | .load-balancing-mode", "raw"),
    (".ltm.profile[] | .insert-xforwarded-for", "raw"),
    (".ltm.persistence[] | .type", "raw"),
    (".ltm.monitor[] | .type", "raw"),
    # ---- pool members (sub-objects) -----------------------------------
    (".ltm.pool[] | .members[].address", "raw"),
    (".ltm.pool[] | .members[].name", "raw"),
    (".ltm.pool[] | {name, members: [.members[].name]}", "json"),
    # ---- object literals + select over real objects -------------------
    (".ltm.virtual[] | {name, pool}", "json"),
    ('.ltm.virtual[] | select(.pool == "/Common/web_pool") | .name', "raw"),
    ('.ltm.pool[] | select(.name | startswith("api")) | .name', "raw"),
    # ---- subscript: full-path, partition-shorthand, index, regex ------
    ('.ltm.pool["/Common/web_pool"]', "json"),
    ('.ltm.pool["/Common/web_pool"]', "scf"),
    ('.ltm.pool["/Common/web_pool"]', "auto"),
    ('.ltm.pool["web_pool"].name', "raw"),
    ('.ltm.virtual["api_vs"].name', "raw"),
    (".ltm.virtual[0].name", "raw"),
    (".ltm.virtual[-1].name", "raw"),
    ('.ltm.virtual["~ssl"] | .name', "raw"),
    ('.ltm.pool["~pool"] | .name', "raw"),
    ('.ltm.pool[]["~pool"] | .name', "raw"),
    # ---- whole-object JSON dumps per kind -----------------------------
    ('.ltm.virtual["/Common/api_vs"]', "json"),
    ('.ltm.node["/Common/web1"]', "json"),
    ('.ltm.data-group["/Common/blocked_ips"]', "json"),
    ('.ltm.data-group["/Common/allowed_hosts"]', "json"),
    ('.ltm.snatpool["/Common/my_snatpool"]', "json"),
    ('.ltm.persistence["/Common/my_cookie_persist"]', "json"),
    ('.ltm.profile["/Common/my_http_profile"]', "json"),
    ('.ltm.monitor["/Common/my_http_monitor"]', "json"),
    # ---- structured list fields rendered to JSON ----------------------
    ('.ltm.virtual["/Common/www_vs"].rules', "json"),
    ('.ltm.virtual["/Common/www_vs"].profiles', "json"),
    ('.ltm.virtual["/Common/www_vs"].persist', "json"),
    # ---- PathRef dereference (VS -> pool -> members) ------------------
    ('.ltm.virtual["/Common/www_vs"].pool.name', "raw"),
    ('.ltm.virtual["/Common/www_vs"].pool.members[].address', "raw"),
    ('.ltm.virtual["/Common/www_vs"].pool.load-balancing-mode', "raw"),
    # ---- aggregation over real objects --------------------------------
    ("[.ltm.virtual[]] | length", "json"),
    ("[.ltm.pool[]] | length", "json"),
    ("[.ltm.rule[]] | length", "json"),
    ("[.ltm.virtual[].name]", "json"),
    ("[.ltm.pool[].name] | sort", "json"),
    ("[.ltm.virtual[].pool] | unique", "json"),
    # ---- paths-only output --------------------------------------------
    (".ltm.virtual[]", "paths"),
    (".ltm.pool[]", "paths"),
    # ---- navigation error cases ---------------------------------------
    ('.ltm.pool["nonexistent"]', "json"),
    ('.ltm.virtual["does_not_exist"].name', "raw"),
    (".ltm.virtual[99].name", "raw"),
    (".ltm.virtual[] | .nope", "raw"),
]


def run(query: str, mode: str, source: str) -> dict:
    root = _build_root(URI, source)
    ctx = EvalContext(root=root)
    try:
        prog = parse_query(query)
        values = evaluate(prog, ctx)
        out = render(values, mode=mode)
        return {"query": query, "mode": mode, "kind": "ok", "output": out}
    except (QueryError, ValueError) as exc:
        return {"query": query, "mode": mode, "kind": "err", "output": str(exc)}


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = (
        repo_root
        / "rust"
        / "tcl-bigip-query"
        / "tests"
        / "fixtures"
        / "projection.json"
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
