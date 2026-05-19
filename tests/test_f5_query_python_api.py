"""Tests for the fluent Python API on top of the query engine.

Covers the ``Query`` / ``QueryRun`` / ``QueryRow`` surface re-exported
as :mod:`f5q`.  The engine itself (parsing, evaluation, edit
application) is exercised by ``tests/test_f5_query.py`` — this file
focuses on the wrapper contracts: source coercion, the flat-versus-
tagged shape switch, ``first`` semantics, and the renderer pass-through.
"""

from __future__ import annotations

from pathlib import Path

import pytest

import dialects.f5.query as f5q
from dialects.f5.query import ObjectRef, Query, QueryError, QueryRow, QueryRun, run_query

SAMPLE_LTM = Path(__file__).resolve().parents[1] / "samples" / "for_f5_query" / "ltm.conf"


def _ltm_text() -> str:
    return SAMPLE_LTM.read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# Top-level alias surface
# ---------------------------------------------------------------------------


def test_f5q_re_exports_public_symbols() -> None:
    """The ``f5q`` shim mirrors :mod:`dialects.f5.query` 1:1."""
    import dialects.f5.query as core_query

    # Every name we promise in __all__ resolves to the same object.
    for name in f5q.__all__:
        assert getattr(f5q, name) is getattr(core_query, name), name


def test_renderer_decorator_round_trips() -> None:
    """Plugins register through ``f5q.renderer`` and dispatch through ``f5q.render``."""

    @f5q.renderer("plugin-under-test", summary="for unit tests", accepts="anything")
    def _r(values, **opts):
        return f"got {len(values)} values; {sorted(opts)}\n"

    try:
        out = f5q.render("plugin-under-test", [1, 2, 3], scale="x10")
        assert out == "got 3 values; ['scale']\n"
        assert any(s.name == "plugin-under-test" for s in f5q.list_renderers())
    finally:
        # Tidy up so the next test sees a clean registry.
        from dialects.f5.query.renderers import _REGISTRY

        _REGISTRY.pop("plugin-under-test", None)


# ---------------------------------------------------------------------------
# Query construction
# ---------------------------------------------------------------------------


def test_query_from_string() -> None:
    q = Query(".ltm.virtual[] | .name")
    assert q.text == ".ltm.virtual[] | .name"


def test_query_from_path(tmp_path: Path) -> None:
    f = tmp_path / "q.f5q"
    f.write_text(".ltm.pool[].name\n")
    q = Query(f)
    assert q.text == ".ltm.pool[].name\n"


# ---------------------------------------------------------------------------
# Query.run — source coercion
# ---------------------------------------------------------------------------


def test_run_rejects_empty_inputs() -> None:
    with pytest.raises(ValueError, match="at least one source"):
        Query(".").run()


def test_run_accepts_sources_dict() -> None:
    run = Query(".ltm.virtual[] | .name").run(sources={"inline.conf": _ltm_text()})
    assert isinstance(run, QueryRun)
    assert run.values()


def test_run_accepts_paths() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    assert run.values()


def test_run_sources_override_paths() -> None:
    """When both *sources* and *paths* point at the same URI, the in-memory
    text wins so a script can rewrite a single config without re-staging
    everything else.
    """
    # Use sources with the exact key Path() would produce so the dict
    # entry shadows the on-disk read.
    key = str(SAMPLE_LTM)
    overridden = "ltm virtual /Common/only_one { destination 1.1.1.1:443 }\n"
    run = Query(".ltm.virtual[] | .name").run(
        sources={key: overridden},
        paths=[SAMPLE_LTM],
    )
    assert run.values() == ["only_one"]


# ---------------------------------------------------------------------------
# QueryRun — flat vs tagged shape
# ---------------------------------------------------------------------------


def test_values_are_flat_strings() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    names = run.values()
    assert all(isinstance(n, str) for n in names)
    assert "web_vs" in names


def test_rows_carry_originating_uri() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    rows = run.rows()
    assert all(isinstance(r, QueryRow) for r in rows)
    assert {r.uri for r in rows} == {str(SAMPLE_LTM)}


def test_first_returns_first_match() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    first = run.first()
    assert first == "web_vs"  # first stanza in the fixture


def test_first_returns_default_when_no_match() -> None:
    run = Query('.ltm.virtual["~/no_such_match_anywhere/"] | .name').run(paths=[SAMPLE_LTM])
    assert run.first("FALLBACK") == "FALLBACK"


def test_objects_filters_to_object_refs() -> None:
    run = Query(".ltm.virtual[]").run(paths=[SAMPLE_LTM])
    objects = run.objects()
    assert objects
    assert all(isinstance(o, ObjectRef) for o in objects)


def test_paths_collects_full_paths() -> None:
    """``paths()`` should pull ``full_path`` out of every ObjectRef /
    PathRef without raising on mixed inputs."""
    run = Query(".ltm.virtual[]").run(paths=[SAMPLE_LTM])
    full_paths = run.paths()
    assert "/Common/web_vs" in full_paths


def test_len_and_bool() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    assert len(run) > 0
    assert bool(run)

    empty = Query('.ltm.virtual["~/no_such_match/"] | .name').run(paths=[SAMPLE_LTM])
    assert len(empty) == 0
    assert not empty


def test_iteration_yields_values() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    via_iter = list(run)
    assert via_iter == run.values()


# ---------------------------------------------------------------------------
# QueryRun.render — pluggable renderer dispatch
# ---------------------------------------------------------------------------


def test_render_dispatches_to_registered_renderer() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    out = run.render("mermaid")
    assert out.startswith("graph LR\n")
    assert "web_vs" in out


def test_render_unknown_name_raises_renderer_error() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    with pytest.raises(f5q.RendererError, match="unknown renderer"):
        run.render("definitely-not-a-renderer")


def test_render_forwards_options() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    out = run.render("mermaid", direction="TB")
    assert out.startswith("graph TB\n")


# ---------------------------------------------------------------------------
# Lower-level shape stays available
# ---------------------------------------------------------------------------


def test_underlying_result_is_exposed() -> None:
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    assert run.result.values_per_file
    assert isinstance(run.result.values_per_file, dict)
    assert not run.result.has_mutation


def test_edits_returns_none_for_no_op() -> None:
    """``edits(uri)`` exists for mutating queries; for a read-only run
    every URI maps to ``None``."""
    run = Query(".ltm.virtual[] | .name").run(paths=[SAMPLE_LTM])
    assert run.edits(str(SAMPLE_LTM)) is None


def test_run_query_low_level_still_works() -> None:
    """The historical :func:`run_query` entry must keep its shape so
    existing CLI / MCP callers don't break."""
    result = run_query(".ltm.virtual[] | .name", {"inline.conf": _ltm_text()})
    assert "inline.conf" in result.values_per_file
    assert "web_vs" in result.values_per_file["inline.conf"]


def test_query_error_propagates_unchanged() -> None:
    """Parse errors come back as :class:`QueryError` subclasses so a
    script catching the public base type Just Works."""
    with pytest.raises(QueryError):
        Query("this is not a valid query !!!").run(paths=[SAMPLE_LTM])
