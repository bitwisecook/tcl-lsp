"""Tests for the progressive Python API — ``f5q.q``, ``f5q.load``, ``Sources``.

Covers:

- :func:`f5q.q` argument disambiguation (expression vs file path vs
  Sources / QueryRun / in-memory data) and the polymorphic call shapes
  the README documents.
- :func:`f5q.load` — staging without running; inline parser callable;
  Sources merging.
- :meth:`QueryRun.q` — method-form progressive chaining.
- :meth:`QueryRun.render` — inline callable support alongside the
  registered-name path.
- :meth:`QueryRun.out` — plain-Python coercion of ObjectRef / PathRef /
  Stream / nested containers.
- Chain semantics — single prior, multiple priors merged, mixed
  prior + file (priors as ``$_chain``).
"""

from __future__ import annotations

from pathlib import Path

import pytest

import f5q

SAMPLES = Path(__file__).resolve().parents[1] / "samples" / "for_f5_query"
LTM_CONF = SAMPLES / "ltm.conf"


# ---------------------------------------------------------------------------
# Argument disambiguation
# ---------------------------------------------------------------------------


def test_q_one_shot_expr_plus_file() -> None:
    """The most common call shape: expression first, file second."""
    r = f5q.q(".ltm.virtual[] | .name", str(LTM_CONF))
    assert "web_vs" in r.values()


def test_q_expr_plus_path_object() -> None:
    """pathlib.Path is accepted alongside string paths."""
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    assert "web_vs" in r.values()


def test_q_no_expression_runs_identity() -> None:
    """``q(file)`` is equivalent to ``q('.', file)`` — identity over the corpus."""
    r = f5q.q(str(LTM_CONF))
    # Identity against a BIG-IP config returns the root; we don't
    # pin the exact shape, just that the call succeeded and gave
    # back a QueryRun.
    assert isinstance(r, f5q.QueryRun)


def test_q_rejects_empty_args() -> None:
    with pytest.raises(ValueError, match="at least one argument"):
        f5q.q()


def test_q_rejects_two_expression_strings() -> None:
    with pytest.raises(ValueError, match="two expression strings"):
        f5q.q(".a", ".b")


def test_q_rejects_unknown_string() -> None:
    """A string that's neither an expression nor an existing file should
    fail fast rather than silently becoming an expression with no input."""
    # The disambiguator treats the first non-file string as the
    # expression; the second non-file string then errors as
    # "two expression strings".
    with pytest.raises(ValueError, match="two expression strings"):
        f5q.q(".x", "/nonexistent/file.conf")


def test_q_rejects_unsupported_input_type() -> None:
    with pytest.raises(TypeError, match="unsupported input type"):
        f5q.q(".x", 42)


# ---------------------------------------------------------------------------
# load() — pre-staging
# ---------------------------------------------------------------------------


def test_load_single_file() -> None:
    data = f5q.load(str(LTM_CONF))
    assert isinstance(data, f5q.Sources)
    assert str(LTM_CONF) in data.text_by_uri


def test_load_multiple_files() -> None:
    data = f5q.load(str(LTM_CONF), LTM_CONF)
    # Same file twice — dedupes by URI.
    assert len(data.text_by_uri) == 1


def test_load_in_memory_value() -> None:
    data = f5q.load([{"x": 1}, {"x": 2}])
    # In-memory inputs become a JSON-spec source under a synthetic URI.
    assert len(data.text_by_uri) == 1
    uri = next(iter(data.text_by_uri))
    assert data.input_specs[uri].kind == "json"


def test_load_with_inline_parser() -> None:
    """A callable parser is wired in as a one-shot input format."""

    def fake_parser(source, *, uri, options=()):
        return [line.upper() for line in source.splitlines() if line]

    data = f5q.load(str(LTM_CONF), parser=fake_parser)
    uri = str(LTM_CONF)
    assert uri in data.inline_parsers
    assert data.input_specs[uri].kind.startswith("<inline-parser:")


def test_load_rejects_unknown_string() -> None:
    """A string that isn't a real file path is a typo, not data."""
    with pytest.raises(ValueError, match="neither an existing file"):
        f5q.load("not-a-real-file.conf")


def test_load_then_query_round_trip() -> None:
    data = f5q.load(str(LTM_CONF))
    r = f5q.q(".ltm.pool[] | .name", data)
    assert "web_pool" in r.values()


def test_load_inline_parser_round_trip(tmp_path: Path) -> None:
    """Custom file format → load() → q() — the canonical use case."""
    src = tmp_path / "items.lines"
    src.write_text("alpha\nbeta\ngamma\n")

    def parse_lines(source, *, uri, options=()):
        return [line for line in source.splitlines() if line.strip()]

    data = f5q.load(str(src), parser=parse_lines)
    r = f5q.q(".[]", data)
    assert r.values() == ["alpha", "beta", "gamma"]


# ---------------------------------------------------------------------------
# Progressive chaining (function form and method form)
# ---------------------------------------------------------------------------


def test_progressive_function_form() -> None:
    """f5q.q(expr, prev) feeds prior values as the new primary input."""
    names = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    filtered = f5q.q('.[] | select(contains(., "web"))', names)
    assert filtered.values() == ["web_vs", "web_secure_vs"]


def test_progressive_method_form() -> None:
    """run.q(expr) is identical to f5q.q(expr, run)."""
    names = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    filtered = names.q('.[] | select(contains(., "_vs"))')
    assert "web_vs" in filtered.values()


def test_progressive_method_form_chains() -> None:
    """run.q(...).q(...) composes left-to-right."""
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF).q('.[] | select(contains(., "_vs"))').q("count")
    assert r.values() == [6]


def test_chain_treats_prior_as_whole_list() -> None:
    """``.`` is the whole prior list (jq semantic); iterate via .[]."""
    names = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    # Without .[], select() runs against the whole list.
    whole = f5q.q("length", names)
    assert whole.values() == [len(names.values())]


def test_chain_multiple_priors_combine() -> None:
    """Multiple QueryRun priors flatten into one combined list."""
    virtuals = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    pools = f5q.q(".ltm.pool[] | .name", LTM_CONF)
    merged = f5q.q(".[]", virtuals, pools)
    assert "web_vs" in merged.values()
    assert "web_pool" in merged.values()
    assert len(merged.values()) == len(virtuals.values()) + len(pools.values())


def test_chain_priors_with_file_bind_as_chain_var() -> None:
    """Priors + file/sources input: priors land under ``$_chain``,
    file acts as primary."""
    keep_names = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    matched = f5q.q(
        ".ltm.virtual[] | select(contains($_chain, .name)) | .name",
        keep_names,
        LTM_CONF,
    )
    # Every virtual matches itself, so the result is identical to
    # the source list (modulo order).
    assert set(matched.values()) == set(keep_names.values())


# ---------------------------------------------------------------------------
# In-memory data
# ---------------------------------------------------------------------------


def test_in_memory_list() -> None:
    r = f5q.q(".[] | select(.x > 5)", [{"x": 1}, {"x": 10}, {"x": 20}])
    assert r.values() == [{"x": 10}, {"x": 20}]


def test_in_memory_dict() -> None:
    r = f5q.q(".name", {"name": "alpha", "count": 5})
    assert r.values() == ["alpha"]


def test_in_memory_combined_with_file_via_prior() -> None:
    """The "join in-memory list against a config" pattern uses a prior
    QueryRun (bound as ``$_chain``) rather than naming an in-memory
    Sources directly — in-memory data through load() gets a synthetic
    auto-name that's not useful in a query, so the natural path is to
    wrap the data in a Result first."""
    keep = f5q.q(".", ["web_pool", "api_pool"])
    matched = f5q.q(
        ".ltm.pool[] | select(contains($_chain, .name)) | .name",
        keep,
        LTM_CONF,
    )
    assert set(matched.values()) <= {"web_pool", "api_pool"}


# ---------------------------------------------------------------------------
# render() — registered name AND inline callable
# ---------------------------------------------------------------------------


def test_render_via_registered_name() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    out = r.render("mermaid")
    assert out.startswith("graph LR")


def test_render_via_inline_callable() -> None:
    """An inline lambda is accepted alongside the registry."""
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)

    def stub(values, **opts):
        return f"count={len(values)}\n"

    assert r.render(stub) == f"count={len(r.values())}\n"


def test_render_inline_callable_receives_opts() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    captured = {}

    def stub(values, **opts):
        captured.update(opts)
        return ""

    r.render(stub, tag="x", count=3)
    assert captured == {"tag": "x", "count": 3}


# ---------------------------------------------------------------------------
# out() — plain-Python coercion
# ---------------------------------------------------------------------------


def test_out_coerces_object_ref() -> None:
    r = f5q.q(".ltm.virtual[]", LTM_CONF)
    plain = r.out()
    assert isinstance(plain, list)
    assert all(isinstance(p, dict) for p in plain)
    assert "full-path" in plain[0]
    assert "fields" in plain[0]


def test_out_coerces_path_ref() -> None:
    r = f5q.q(".ltm.virtual[] | .pool", LTM_CONF)
    plain = r.out()
    # PathRef values flatten to their full_path string.
    assert all(isinstance(p, str) or p is None for p in plain)


def test_out_leaves_scalars_alone() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    plain = r.out()
    assert plain == r.values()
    assert all(isinstance(p, str) for p in plain)


def test_out_round_trips_through_json() -> None:
    """The whole point of out(): the result must JSON-serialise cleanly."""
    import json

    r = f5q.q(".ltm.virtual[]", LTM_CONF)
    text = json.dumps(r.out())
    assert "/Common/web_vs" in text


# ---------------------------------------------------------------------------
# QueryRun list-likeness
# ---------------------------------------------------------------------------


def test_indexing_returns_value() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    assert r[0] == "web_vs"


def test_iteration_matches_values() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    assert list(r) == r.values()


def test_len_and_bool() -> None:
    r = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    assert len(r) == len(r.values())
    assert bool(r) is (len(r) > 0)


# ---------------------------------------------------------------------------
# Immutability
# ---------------------------------------------------------------------------


def test_chain_does_not_mutate_prior() -> None:
    """Each q() returns a new QueryRun; the prior is unchanged."""
    a = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    before = a.values()
    _ = a.q('.[] | select(contains(., "web"))')
    after = a.values()
    assert before == after


def test_branching_from_one_prior() -> None:
    """Two different chains off one prior produce independent results."""
    a = f5q.q(".ltm.virtual[] | .name", LTM_CONF)
    web = a.q('.[] | select(contains(., "web"))').values()
    api = a.q('.[] | select(contains(., "api"))').values()
    assert "web_vs" in web
    assert "api_vs" in api
    assert set(web).isdisjoint(set(api))
