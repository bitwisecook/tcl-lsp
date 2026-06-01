"""Tests for the shared annotation backend.

Pins the contract that both the CLI text renderer and the web GUI JSON
consumer rely on: a single typed list with backend-assigned severity,
deterministic sort order, and ``--max-annotations`` truncation.
"""

from __future__ import annotations

from tooling.explorer.annotations import (
    Annotation,
    Severity,
    collect_annotations,
    shimmer_severity,
    taint_severity,
)
from tooling.explorer.pipeline import ALL_VIEWS, run_pipeline


def _all(source: str):
    result = run_pipeline(source, dialect="tcl8.6")
    return collect_annotations(result, views=ALL_VIEWS, max_annotations=-1)


class TestSeverityClassification:
    def test_taint_T1_is_error(self):
        assert taint_severity("T1001") is Severity.ERROR
        assert taint_severity("T1") is Severity.ERROR

    def test_taint_T3001_T3004_is_error(self):
        for code in ("T3001", "T3002", "T3003", "T3004"):
            assert taint_severity(code) is Severity.ERROR, code

    def test_other_taint_is_warning(self):
        assert taint_severity("T2001") is Severity.WARNING
        assert taint_severity("IRULE3103") is Severity.WARNING

    def test_shimmer_S102_is_error(self):
        assert shimmer_severity("S102") is Severity.ERROR

    def test_other_shimmer_is_warning(self):
        assert shimmer_severity("S100") is Severity.WARNING
        assert shimmer_severity("S101") is Severity.WARNING


class TestAnnotationCollection:
    def test_empty_source_yields_no_annotations(self):
        anns, omitted = _all("")
        assert anns == []
        assert omitted == 0

    def test_each_annotation_carries_severity(self):
        # Provoke an optimisation finding via a foldable constant call.
        source = "proc add {a b} { return [expr {$a + $b}] }\nputs [add 2 3]\n"
        anns, _ = _all(source)
        # Every annotation has a valid severity classification — renderers
        # rely on this without re-deriving from the warning code.
        for a in anns:
            assert isinstance(a, Annotation)
            assert a.severity in (Severity.ERROR, Severity.WARNING, Severity.INFO)
            assert a.kind  # non-empty

    def test_view_filter_drops_off_topic_annotations(self):
        source = "proc add {a b} { return [expr {$a + $b}] }\nputs [add 2 3]\n"
        # Only requesting the IR view excludes opt findings.
        ir_only, _ = collect_annotations(
            run_pipeline(source), views=frozenset({"ir", "cfg", "ssa"}), max_annotations=-1
        )
        # No opt findings when the opt view is not requested.
        assert all(a.kind != "optimisation" for a in ir_only)

    def test_max_annotations_truncates_and_reports_omitted(self):
        # Build a script with several barrier-inducing constructs to
        # produce many annotations cheaply.
        source = (
            "proc p {x} { uplevel #0 { set ::g 1 } }\n"
            "proc q {} { eval {set ::h 2} }\n"
            "proc r {} { return [unknownCmd] }\n"
            "p 1; q; r\n"
        )
        full, full_omitted = _all(source)
        assert full_omitted == 0
        # Cap to one and check the omitted count.
        capped, omitted = collect_annotations(
            run_pipeline(source), views=ALL_VIEWS, max_annotations=1
        )
        if len(full) > 1:
            assert len(capped) == 1
            assert omitted == len(full) - 1

    def test_sort_is_stable_by_offset_then_priority(self):
        # Two findings on the same offset must come out in a deterministic
        # order (priority ascending) so the CLI snapshot output is stable.
        source = "proc f {x} { return [expr {$x + 1}] }\nputs [f 5]\n"
        anns, _ = _all(source)
        for prev, curr in zip(anns, anns[1:]):
            if prev.range.start.offset == curr.range.start.offset:
                # priority then span width.
                assert (prev.priority, prev.range.end.offset - prev.range.start.offset) <= (
                    curr.priority,
                    curr.range.end.offset - curr.range.start.offset,
                )
            else:
                assert prev.range.start.offset < curr.range.start.offset


class TestRendererIndependence:
    """The two adapters (CLI text, JSON serialiser) consume the same list."""

    def test_serialiser_emits_severity_on_each_annotation(self):
        from tooling.cli.serialise import serialise_result

        source = "proc add {a b} { return [expr {$a + $b}] }\nputs [add 2 3]\n"
        payload = serialise_result(run_pipeline(source))
        for a in payload["annotations"]:
            assert "severity" in a
            assert a["severity"] in ("error", "warning", "info")
            assert "kind" in a

    def test_serialiser_pre_groups_by_line(self):
        from tooling.cli.serialise import serialise_result

        source = "proc add {a b} { return [expr {$a + $b}] }\nputs [add 2 3]\n"
        payload = serialise_result(run_pipeline(source))
        # Pre-grouped by line; renderer doesn't rebuild a line index.
        if payload["annotations"]:
            by_line = payload["annotationsByLine"]
            assert isinstance(by_line, dict)
            # Every annotation index appears in exactly one bucket.
            indices_in_buckets = sorted(i for v in by_line.values() for i in v)
            assert indices_in_buckets == list(range(len(payload["annotations"])))

    def test_serialiser_emits_meta_block(self):
        from tooling.cli.serialise import serialise_result

        payload = serialise_result(run_pipeline("puts hi"))
        meta = payload["meta"]
        # The dialect dropdown and view tab list are data-driven.
        assert "dialects" in meta and len(meta["dialects"]) > 0
        assert "views" in meta and len(meta["views"]) > 0
        assert meta["severities"] == ["error", "warning", "info"]
