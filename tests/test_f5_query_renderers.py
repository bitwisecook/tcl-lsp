"""Tests for the built-in renderer plugins.

Three renderers ship in-tree: ``gantt``, ``ascii-blocks``, and
``mermaid``.  These tests pin their per-shape contracts and validate
that the renderer registry behaves as documented.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from dialects.f5.query import Query
from dialects.f5.query.errors import RendererError
from dialects.f5.query.renderers import RendererSpec, list_renderers, lookup, render, renderer
from dialects.f5.query.renderers.ascii_blocks import render_ascii_blocks
from dialects.f5.query.renderers.gantt import render_gantt

SAMPLES = Path(__file__).resolve().parents[1] / "samples" / "for_f5_query"


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_builtin_renderers_register() -> None:
    names = {s.name for s in list_renderers()}
    assert names >= {"gantt", "ascii-blocks", "mermaid"}


def test_lookup_returns_spec() -> None:
    spec = lookup("gantt")
    assert isinstance(spec, RendererSpec)
    assert spec.name == "gantt"
    assert "Gantt" in spec.summary


def test_lookup_unknown_returns_none() -> None:
    assert lookup("no-such-renderer-anywhere") is None


def test_dispatch_unknown_raises_renderer_error() -> None:
    with pytest.raises(RendererError, match="unknown renderer"):
        render("no-such-renderer-anywhere", [])


def test_decorator_rejects_duplicates() -> None:
    with pytest.raises(RuntimeError, match="duplicate renderer"):

        @renderer("gantt", summary="dup", accepts="x")
        def _r(values, **opts):
            return ""


# ---------------------------------------------------------------------------
# Gantt
# ---------------------------------------------------------------------------


GANTT_FIXTURE_ROWS = [
    ("Mar 14 10:11:09", "/Common/t2_c01_vip:443", "DOWN"),
    ("Mar 14 10:20:09", "/Common/t2_c01_vip:443", "UP"),
    ("Mar 14 10:25:09", "/Common/t2_c02_vip:443", "DOWN"),
    ("Mar 14 11:00:09", "/Common/t2_c02_vip:443", "UP"),
]


def test_gantt_renders_three_tuples() -> None:
    out = render_gantt(GANTT_FIXTURE_ROWS, unit_minutes=5)
    assert "members down/up over time (1 char = 5 min)" in out
    assert "t2_c01_vip:443" in out
    assert "t2_c02_vip:443" in out
    # Each member's state row carries v / # / ^.
    for line in out.splitlines():
        if line.startswith("t2_c01_vip"):
            assert "v" in line and "^" in line and "#" in line


def test_gantt_empty_input_is_clear() -> None:
    out = render_gantt([], unit_minutes=5)
    assert "no events" in out


def test_gantt_via_registry_accepts_tsv_string() -> None:
    tsv = "\n".join("\t".join(row) for row in GANTT_FIXTURE_ROWS)
    out = render("gantt", [tsv])
    assert "t2_c01_vip:443" in out


def test_gantt_via_registry_accepts_dict_rows() -> None:
    rows = [
        {"timestamp": ts, "label": label, "state": state} for ts, label, state in GANTT_FIXTURE_ROWS
    ]
    out = render("gantt", rows)
    assert "t2_c01_vip:443" in out


def test_gantt_accepts_member_alias_in_dict() -> None:
    """The docs say ``label`` is the canonical key but ``member`` works too
    because the upstream ``tsv(...)`` chain often projects the syslog
    field name verbatim."""
    rows = [
        {"timestamp": ts, "member": label, "state": state}
        for ts, label, state in GANTT_FIXTURE_ROWS
    ]
    out = render("gantt", rows)
    assert "t2_c01_vip:443" in out


def test_gantt_rejects_bad_unit_minutes() -> None:
    with pytest.raises(RendererError, match="divisor of 60"):
        render("gantt", [], **{"unit-minutes": "7"})


def test_gantt_rejects_dict_missing_keys() -> None:
    with pytest.raises(RendererError, match="dict rows"):
        render("gantt", [{"timestamp": "x"}])


def test_gantt_matches_multitier_fixture() -> None:
    """End-to-end byte-for-byte against the published sample output.

    Section 7.7 of ``samples/for_f5_query/sysadmin-queries.md`` shows
    the canonical rendering — if this drifts, the docs are wrong.
    """
    query = """
        f5log_load("samples/for_f5_query/multitier/logs/t1-a.log")[]
        | select(.module == "01340011" or .module == "01340012")
        | tsv(.timestamp,
              (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
              (if .module == "01340011" then "DOWN" else "UP" end))
    """
    out = (
        Query(query)
        .run(
            paths=[SAMPLES / "multitier" / "tier1-ltm-ha.conf"],
        )
        .render("gantt")
    )
    lines = out.splitlines()
    assert lines[0] == "members down/up over time (1 char = 5 min)"
    # The 12 t2_c?? members appear in sorted order, each on one line.
    members = [line.split("|")[0].strip() for line in lines if line.startswith("t2_c")]
    assert members == [f"t2_c{i:02d}_vip:443" for i in range(1, 13)]


# ---------------------------------------------------------------------------
# ASCII blocks
# ---------------------------------------------------------------------------


def test_ascii_blocks_renders_single_box() -> None:
    out = render_ascii_blocks({"title": "Hello"}, style="rounded")
    assert "╭" in out and "╮" in out
    assert "Hello" in out


def test_ascii_blocks_renders_nested_tree() -> None:
    out = render_ascii_blocks(
        {
            "title": "Root",
            "rows": [
                {"title": "Child A"},
                {"title": "Child B", "rows": [{"title": "Grandchild"}]},
            ],
        }
    )
    assert "Root" in out
    assert "Child A" in out
    assert "Child B" in out
    assert "Grandchild" in out
    # The tree connector glyphs appear between parent and children.
    assert "├─" in out
    assert "╰─" in out


def test_ascii_blocks_ascii_style() -> None:
    out = render_ascii_blocks({"title": "Plain"}, style="ascii")
    # Pure-ASCII style uses + / - / | so the output survives terminals
    # that can't render box-drawing.
    assert "+" in out and "-" in out and "|" in out
    assert "╭" not in out


def test_ascii_blocks_rejects_unknown_style() -> None:
    with pytest.raises(RendererError, match="unknown style"):
        render("ascii-blocks", [{"title": "x"}], style="neon")


def test_ascii_blocks_rejects_block_without_title() -> None:
    with pytest.raises(RendererError, match="must carry 'title'"):
        render("ascii-blocks", [{"rows": [{"title": "kid"}]}])


def test_ascii_blocks_accepts_synonyms() -> None:
    """``label`` / ``name`` for title; ``children`` / ``items`` for rows."""
    out = render_ascii_blocks(
        {"name": "Root", "children": [{"label": "Kid"}]},
    )
    assert "Root" in out and "Kid" in out


def test_ascii_blocks_empty_returns_marker() -> None:
    out = render("ascii-blocks", [])
    assert "no blocks" in out


# ---------------------------------------------------------------------------
# Mermaid
# ---------------------------------------------------------------------------


def test_mermaid_chain_mode_for_scalars() -> None:
    out = render("mermaid", ["alpha", "beta", "gamma"])
    assert out.startswith("graph LR\n")
    assert '"alpha"' in out
    assert "n0 --> n1" in out
    assert "n1 --> n2" in out


def test_mermaid_object_mode_emits_edges() -> None:
    run = Query(".ltm.virtual[]").run(paths=[SAMPLES / "ltm.conf"])
    out = run.render("mermaid")
    assert out.startswith("graph LR\n")
    # The object-graph mode should walk pool edges.
    assert "ltm_pool" in out or "ltm_node" in out


def test_mermaid_direction_option() -> None:
    out = render("mermaid", ["x", "y"], direction="TB")
    assert out.startswith("graph TB\n")


def test_mermaid_rejects_bad_direction() -> None:
    with pytest.raises(RendererError, match="unknown direction"):
        render("mermaid", ["x"], direction="SIDEWAYS")


def test_mermaid_falls_back_to_chain_for_dicts() -> None:
    out = render("mermaid", [{"title": "Step 1"}, {"title": "Step 2"}])
    assert "Step 1" in out and "Step 2" in out
    assert "n0 --> n1" in out


# ---------------------------------------------------------------------------
# CLI integration — `--render NAME` and `--help-renderers`
# ---------------------------------------------------------------------------


def test_cli_help_renderers_lists_builtins(capsys) -> None:
    import argparse

    from tooling.explorer.verbs.f5.query import _HelpRenderersAction

    parser = argparse.ArgumentParser()
    action = _HelpRenderersAction(option_strings=["--help-renderers"], dest="x", nargs=0)
    with pytest.raises(SystemExit):
        action(parser, argparse.Namespace(), None)
    out = capsys.readouterr().out
    for name in ("gantt", "ascii-blocks", "mermaid"):
        assert name in out
