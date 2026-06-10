"""Tests for the shared CFG edge-routing model (``tooling/explorer/cfg_layout``).

The web SVG router and the CLI/TUI ASCII gutter both consume this model, so the
lane assignment is single-sourced here.  These tests pin the routing contract:
lanes never collide, the algorithm matches the (byte-identical) web JS loop, and
the ASCII gutter draws the routed lanes as box-drawing characters.
"""

from __future__ import annotations

from tooling.cli.serialise import serialise_result
from tooling.explorer._render import _cfg_gutter
from tooling.explorer.cfg_layout import assign_lanes, build_cfg_edges
from tooling.explorer.pipeline import run_pipeline

_BRANCHY = (
    "proc f {x} {\n"
    "    set total 0\n"
    "    for {set i 0} {$i < $x} {incr i} {\n"
    "        if {$i % 2 == 0} { incr total $i } else { incr total 1 }\n"
    "    }\n"
    "    return $total\n"
    "}\n"
)


def _spans_overlap(a: tuple[int, int], b: tuple[int, int]) -> bool:
    lo1, hi1 = sorted(a)
    lo2, hi2 = sorted(b)
    return not (hi1 < lo2 or lo1 > hi2)


class TestAssignLanes:
    def test_disjoint_spans_share_lane_zero(self):
        # Sequential, non-overlapping spans all nest into the innermost lane.
        assert assign_lanes([(0, 1), (2, 3), (4, 5)]) == [0, 0, 0]

    def test_touching_endpoints_force_distinct_lanes(self):
        # An edge into block 2 and out of block 2 must not share a lane.
        assert assign_lanes([(0, 2), (2, 4)]) == [0, 1]

    def test_shortest_span_gets_innermost_lane(self):
        # Longest span is processed last → outer lane.
        lanes = assign_lanes([(0, 5), (1, 2)])
        assert lanes[1] < lanes[0]

    def test_no_two_same_lane_edges_overlap(self):
        import random

        rng = random.Random(7)
        for _ in range(500):
            spans = [(rng.randint(0, 9), rng.randint(0, 9)) for _ in range(rng.randint(1, 8))]
            lanes = assign_lanes(list(spans))
            by_lane: dict[int, list[tuple[int, int]]] = {}
            for span, lane in zip(spans, lanes):
                for other in by_lane.get(lane, []):
                    assert not _spans_overlap(span, other), (spans, lanes)
                by_lane.setdefault(lane, []).append(span)


class TestBuildCfgEdges:
    def test_branch_kinds_and_lanes(self):
        result = run_pipeline(_BRANCHY)
        snap = next(s for s in result.snapshots if s.name == "::f")
        edges = build_cfg_edges(snap.cfg)
        # The loop header is a conditional branch → one true + one false edge.
        kinds = {(e.src, e.kind) for e in edges if e.src.startswith("for_header")}
        assert ("for_header_2", "true") in kinds
        assert ("for_header_2", "false") in kinds
        # Every successor edge is routed (lane assigned, no collisions).
        assert all(e.lane >= 0 for e in edges)
        by_lane: dict[int, list[tuple[int, int]]] = {}
        for e in edges:
            for other in by_lane.get(e.lane, []):
                assert not _spans_overlap((e.src_pos, e.dst_pos), other)
            by_lane.setdefault(e.lane, []).append((e.src_pos, e.dst_pos))


class TestCfgGutter:
    def test_no_edges_is_blank(self):
        assert _cfg_gutter(3, []) == ["", "", ""]

    def test_down_edge_routes_corner_to_arrow(self):
        # One forward edge from row 0 to row 2, innermost lane (width 1).
        gutter = _cfg_gutter(3, [(0, 2, 0, "goto")])
        assert gutter == ["┌", "│", "▶"]

    def test_back_edge_routes_upward(self):
        # A loop back-edge (row 2 → row 0) turns up, arrowhead on the top row.
        gutter = _cfg_gutter(3, [(2, 0, 0, "goto")])
        assert gutter == ["▶", "│", "└"]

    def test_width_follows_max_lane(self):
        gutter = _cfg_gutter(4, [(0, 3, 0, "goto"), (1, 2, 1, "goto")])
        assert all(len(row) == 2 for row in gutter)


class TestSerialisedEdges:
    def test_cfg_edges_carry_lanes(self):
        payload = serialise_result(run_pipeline(_BRANCHY))
        for view in ("cfgPreSsa", "cfgPostSsa"):
            func = next(f for f in payload[view] if f["name"] == "::f")
            assert func["edges"], f"{view} has no edges"
            for e in func["edges"]:
                assert {"from", "to", "fromPos", "toPos", "kind", "lane"} <= e.keys()
                assert isinstance(e["lane"], int) and e["lane"] >= 0
