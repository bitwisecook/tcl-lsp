"""Tests for the M2 analysis verbs: stats, graph, explain, diff."""

from __future__ import annotations

import json
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from explorer.f5_cli import main


SAMPLE_CONF = Path(__file__).resolve().parent.parent / "samples" / "bigip" / "bigip.conf"


SMALL = textwrap.dedent(
    """
    ltm node /Common/n1 {
        address 10.0.0.1
    }
    ltm pool /Common/p1 {
        members {
            /Common/n1:80 {
                address 10.0.0.1
            }
        }
        load-balancing-mode round-robin
    }
    ltm rule /Common/r1 {
        when HTTP_REQUEST {
            pool /Common/p1
        }
    }
    ltm virtual /Common/vs_app {
        destination /Common/10.0.0.5:80
        pool /Common/p1
        rules {
            /Common/r1
        }
    }
    """
).strip() + "\n"


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _write(tmp_path: Path, name: str, body: str) -> Path:
    path = tmp_path / name
    path.write_text(body)
    return path


# ── stats ────────────────────────────────────────────────────────────


def test_stats_text_default(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["stats", str(p)], capsys)
    assert code == 0
    assert "objects:" in out
    assert "pool" in out
    assert "virtual" in out


def test_stats_json_counts(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["stats", str(p), "--json"], capsys)
    assert code == 0
    payload = json.loads(out)
    assert payload["objectCounts"]["pool"] == 1
    assert payload["objectCounts"]["virtual"] == 1
    assert payload["objectCounts"]["rule"] == 1
    assert payload["totalObjects"] >= 4


def test_stats_summary_alias(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, _out, _err = _run(["summary", str(p), "--json"], capsys)
    assert code == 0


def test_stats_against_sample_config(capsys):
    code, out, _err = _run(["stats", str(SAMPLE_CONF), "--json"], capsys)
    assert code == 0
    payload = json.loads(out)
    assert payload["totalObjects"] > 0


# ── graph ────────────────────────────────────────────────────────────


def test_graph_dot_output(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["graph", str(p)], capsys)
    assert code == 0
    assert out.startswith("digraph bigip")
    assert "->" in out


def test_graph_json_format(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["graph", str(p), "--format", "json"], capsys)
    assert code == 0
    payload = json.loads(out)
    assert "nodes" in payload and "edges" in payload
    assert any("vs_app" in n["identifier"] for n in payload["nodes"])


def test_graph_mermaid_format(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["graph", str(p), "--format", "mermaid"], capsys)
    assert code == 0
    assert out.startswith("graph LR")


def test_graph_seed_subgraph(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(
        ["graph", str(p), "--format", "json", "--seed", "/Common/vs_app"], capsys
    )
    assert code == 0
    payload = json.loads(out)
    identifiers = {n["identifier"] for n in payload["nodes"]}
    assert "/Common/vs_app" in identifiers


def test_graph_seed_with_no_match_yields_empty(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(
        ["graph", str(p), "--format", "json", "--seed", "/Common/nope"], capsys
    )
    assert code == 0
    payload = json.loads(out)
    assert payload["nodes"] == []


# ── explain ──────────────────────────────────────────────────────────


def test_explain_virtual_text(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["explain", "virtual", "/Common/vs_app", str(p)], capsys)
    assert code == 0
    assert "explain virtual" in out
    assert "default pool" in out
    assert "/Common/p1" in out
    assert "iRules" in out
    assert "HTTP_REQUEST" in out


def test_explain_pool_resolves_short_name(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["explain", "pool", "p1", str(p)], capsys)
    assert code == 0
    assert "/Common/p1" in out
    assert "/Common/n1:80" in out


def test_explain_auto_finds_virtual_first(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["explain", "auto", "vs_app", str(p)], capsys)
    assert code == 0
    assert "explain virtual" in out


def test_explain_missing_target_exits_one(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, _out, _err = _run(["explain", "virtual", "nonexistent", str(p)], capsys)
    assert code == 1


def test_explain_json(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["explain", "virtual", "/Common/vs_app", str(p), "--json"], capsys)
    assert code == 0
    payload = json.loads(out)
    assert payload["found"] is True
    assert payload["kind"] == "virtual"
    headings = {s["heading"] for s in payload["sections"]}
    assert {"profiles", "iRules", "default pool"} <= headings


# ── diff ─────────────────────────────────────────────────────────────


def test_diff_no_changes(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    b = _write(tmp_path, "b.conf", SMALL)
    code, out, _err = _run(["diff", str(a), str(b)], capsys)
    assert code == 0
    assert "+0 added" in out


def test_diff_added_object(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    extra = SMALL + "\nltm pool /Common/p2 { }\n"
    b = _write(tmp_path, "b.conf", extra)
    code, out, _err = _run(["diff", str(a), str(b)], capsys)
    assert code == 1
    assert "+1 added" in out
    assert "/Common/p2" in out


def test_diff_removed_object(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL + "\nltm pool /Common/p3 { }\n")
    b = _write(tmp_path, "b.conf", SMALL)
    code, out, _err = _run(["diff", str(a), str(b)], capsys)
    assert code == 1
    assert "-1 removed" in out
    assert "/Common/p3" in out


def test_diff_modified_field(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    after = SMALL.replace("destination /Common/10.0.0.5:80", "destination /Common/10.0.0.99:443")
    b = _write(tmp_path, "b.conf", after)
    code, out, _err = _run(["diff", str(a), str(b)], capsys)
    assert code == 1
    assert "~1 modified" in out
    assert "destination" in out
    assert "10.0.0.99" in out


def test_diff_json(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    b = _write(tmp_path, "b.conf", SMALL + "\nltm pool /Common/q { }\n")
    code, out, _err = _run(["diff", str(a), str(b), "--json"], capsys)
    assert code == 1
    payload = json.loads(out)
    assert payload["summary"]["added"] == 1
    assert payload["summary"]["removed"] == 0
    assert any(c["fullPath"] == "/Common/q" for c in payload["added"])


def test_diff_irule_body_change(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    new_body = SMALL.replace(
        "pool /Common/p1\n", 'pool /Common/p1\n            log local0. "hi"\n'
    )
    b = _write(tmp_path, "b.conf", new_body)
    code, out, _err = _run(["diff", str(a), str(b), "--json"], capsys)
    assert code == 1
    payload = json.loads(out)
    modified_paths = {c["fullPath"] for c in payload["modified"]}
    assert "/Common/r1" in modified_paths


def test_diff_irule_whitespace_only_change_is_ignored(tmp_path, capsys):
    a = _write(tmp_path, "a.conf", SMALL)
    # Only-whitespace change inside the rule body — should not register.
    new_body = SMALL.replace("pool /Common/p1\n", "pool /Common/p1   \n")
    b = _write(tmp_path, "b.conf", new_body)
    code, _out, _err = _run(["diff", str(a), str(b)], capsys)
    assert code == 0
