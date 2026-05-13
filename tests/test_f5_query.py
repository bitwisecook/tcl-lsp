"""Tests for the ``f5 query`` verb and its DSL.

Covers four bands:

1. Parser — every cookbook example parses without error.
2. Evaluator — projection, filtering, builtins, and PathRef auto-deref.
3. Edit application — property assignment, identity rename, conflict
   detection.
4. CLI — exit codes, output modes, and the three custom help actions.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import (
    format_builtins,
    format_examples,
    format_grammar,
    list_builtins,
    list_examples,
    parse_query,
    run_query,
)
from core.bigip.query.errors import EvalError, ParseError
from core.bigip.query.output import render
from explorer.f5_cli import main

SAMPLE_CONF = """ltm node /Common/n1 {
    address 10.0.0.1
}
ltm pool /Common/web_pool {
    members {
        /Common/n1:80 {
            address 10.0.0.1
        }
    }
    monitor /Common/http
}
ltm pool /Common/api_pool {
    members {
        /Common/n1:8080 {
            address 10.0.0.1
        }
    }
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/web_pool
    rules {
        /Common/log_rule
    }
}
ltm virtual /Common/api_vs {
    destination /Common/10.10.0.6:80
    pool /Common/api_pool
}
ltm rule /Common/log_rule {
when HTTP_REQUEST {
    pool /Common/web_pool
    log local0. "hit"
}
}
"""


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def test_parser_accepts_every_cookbook_example():
    for example in list_examples():
        parse_query(example.query)


def test_parser_rejects_assignment_with_non_path_lhs():
    with pytest.raises(ParseError):
        parse_query('"hello" = "world"')


def test_parser_accepts_pipeline_assignment():
    parse_query('.ltm.virtual[] | .destination |= ip("10.0.0.0/24", .)')


def test_parser_accepts_semicolon_statements():
    parse_query(".ltm.virtual[].name ; .ltm.pool[].name")


# ---------------------------------------------------------------------------
# Evaluator: projection and filter
# ---------------------------------------------------------------------------


def _run(query: str, source: str = SAMPLE_CONF):
    return run_query(query, {"mem://1": source})


def test_project_virtual_names():
    result = _run(".ltm.virtual[].name")
    assert result.values_per_file["mem://1"] == ["web_vs", "api_vs"]


def test_project_pool_ref_full_path():
    result = _run(".ltm.virtual.web_vs.pool")
    [pool_ref] = result.values_per_file["mem://1"]
    assert pool_ref.full_path == "/Common/web_pool"


def test_pathref_auto_dereferences_through_field_access():
    result = _run(".ltm.virtual.web_vs.pool.monitor")
    [monitor] = result.values_per_file["mem://1"]
    assert monitor.full_path == "/Common/http"


def test_regex_subscript():
    result = _run('.ltm.virtual["~^/Common/api"] | .name')
    assert result.values_per_file["mem://1"] == ["api_vs"]


def test_partition_shorthand():
    result = _run(".ltm.virtual.web_vs.destination")
    assert result.values_per_file["mem://1"] == ["/Common/10.10.0.5:443"]


def test_select_filters_stream():
    result = _run('.ltm.virtual[] | select(startswith(.name, "api")) | .name')
    assert result.values_per_file["mem://1"] == ["api_vs"]


def test_in_cidr_filter():
    result = _run('.ltm.virtual[] | select(in_cidr(.destination, "10.10.0.0/24")) | .name')
    assert sorted(result.values_per_file["mem://1"]) == ["api_vs", "web_vs"]


def test_refs_builtin_lists_dependencies():
    result = _run(".ltm.virtual.web_vs | refs(.)")
    [refs] = result.values_per_file["mem://1"]
    assert "/Common/web_pool" in refs
    assert "/Common/log_rule" in refs


def test_irule_refs_pools_via_query():
    result = _run(".ltm.rule[].refs.pools")
    [pools] = result.values_per_file["mem://1"]
    assert [p.full_path for p in pools] == ["/Common/web_pool"]


def test_unknown_builtin_raises_eval_error():
    with pytest.raises(EvalError):
        _run("no_such_builtin(.)")


# ---------------------------------------------------------------------------
# Builtins
# ---------------------------------------------------------------------------


def test_ip_rebase_preserves_host_and_port():
    result = _run('.ltm.virtual.web_vs.destination | ip("192.168.9.0/24", .)')
    assert result.values_per_file["mem://1"] == ["/Common/192.168.9.5:443"]


def test_partition_and_basename():
    result = _run('partition("/Common/web_pool")')
    assert result.values_per_file["mem://1"] == ["Common"]
    result = _run('basename("/Common/web_pool")')
    assert result.values_per_file["mem://1"] == ["web_pool"]


def test_with_partition_replaces_partition():
    result = _run('with_partition("/Common/web_pool", "Tenant_A")')
    assert result.values_per_file["mem://1"] == ["/Tenant_A/web_pool"]


def test_contains_matches_lists_and_strings():
    result = _run('.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name')
    assert result.values_per_file["mem://1"] == ["web_vs"]


# ---------------------------------------------------------------------------
# Mutation
# ---------------------------------------------------------------------------


def test_simple_field_assignment_rewrites_source():
    result = _run('.ltm.virtual.web_vs.pool = "/Common/replacement"')
    new_src = result.edits_per_file["mem://1"].new_source
    assert "pool /Common/replacement" in new_src
    # The pool stanza header and the iRule body reference are unchanged
    # — property assignment touches one field, not every reference.
    assert "ltm pool /Common/web_pool" in new_src
    # And the *other* virtual's pool is untouched.
    assert "pool /Common/api_pool" in new_src


def test_update_assignment_uses_current_value():
    result = _run('.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)')
    new_src = result.edits_per_file["mem://1"].new_source
    assert "destination /Common/192.168.9.5:443" in new_src
    assert "destination /Common/192.168.9.6:80" in new_src


def test_identity_rename_rewrites_every_reference():
    result = _run('.ltm.pool["/Common/web_pool"].name = "/Common/new_web_pool"')
    applied = result.edits_per_file["mem://1"]
    assert any(rep.old == "/Common/web_pool" for rep in applied.rename_reports)
    new_src = applied.new_source
    assert "/Common/web_pool" not in new_src
    assert "ltm pool /Common/new_web_pool" in new_src
    # Reference inside the iRule body is rewritten too.
    assert "pool /Common/new_web_pool" in new_src
    # The grand-total occurrence count covers header + every reference.
    assert applied.rename_reports[0].occurrences >= 3


def test_update_assignment_on_identity_field_routes_to_rename():
    result = _run('.ltm.pool["/Common/web_pool"].name |= with_partition(., "Tenant_A")')
    applied = result.edits_per_file["mem://1"]
    assert any(rep.old == "/Common/web_pool" for rep in applied.rename_reports)
    new_src = applied.new_source
    assert "ltm pool /Tenant_A/web_pool" in new_src
    # The VS's `pool` reference moved along with the pool.
    assert "pool /Tenant_A/web_pool" in new_src


# ---------------------------------------------------------------------------
# Partition cascade and route domains
# ---------------------------------------------------------------------------


PARTITION_CONF = """auth partition Common { description default }
ltm pool /Common/web_pool {
    members { /Common/n1%5:80 { address 10.0.0.1%5 } }
    monitor /Common/http
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5%5:443
    pool /Common/web_pool
}
ltm rule /Common/log_rule {
when HTTP_REQUEST {
    pool /Common/web_pool
}
}
"""


def test_rename_builtin_renames_header_and_references():
    result = _run('rename("/Common/web_pool", "/Common/app_pool")')
    applied = result.edits_per_file["mem://1"]
    new_src = applied.new_source
    # Header moved.
    assert "ltm pool /Common/app_pool" in new_src
    # VS reference moved.
    assert "pool /Common/app_pool" in new_src
    # iRule body reference moved.
    assert "/Common/web_pool" not in new_src
    # And a rename_report surfaced so the CLI can print the summary.
    assert any(rep.old == "/Common/web_pool" for rep in applied.rename_reports)


def test_rename_builtin_zero_match_is_a_no_op_not_an_error():
    # Tolerant rename: the CLI surfaces this as a warning + exit 1, the
    # DSL just returns 0.
    result = _run('rename("/Common/no_such_thing", "/Common/whatever")')
    # has_mutation is True because the user attempted a rename, but the
    # source is unchanged so the CLI's _emit_mutation will exit 1.
    assert result.has_mutation
    applied = result.edits_per_file["mem://1"]
    assert applied.new_source == applied.original
    assert applied.rename_reports == ()


def test_rename_builtin_composes_with_property_edit_across_statements():
    result = _run(
        'rename("/Common/web_pool", "/Common/app_pool") ; '
        '.ltm.pool["/Common/app_pool"].monitor = "/Common/tcp"'
    )
    new_src = result.edits_per_file["mem://1"].new_source
    assert "ltm pool /Common/app_pool" in new_src
    assert "monitor /Common/tcp" in new_src


def test_rename_partition_cascades_through_compound_values():
    result = run_query('rename_partition("Common", "Tenant_A")', {"mem://1": PARTITION_CONF})
    new_src = result.edits_per_file["mem://1"].new_source
    # Every /Common/ occurrence — including the destination address
    # prefix and the pool-member identifier — has moved.
    assert "/Common/" not in new_src
    assert "ltm pool /Tenant_A/web_pool" in new_src
    assert "destination /Tenant_A/10.10.0.5%5:443" in new_src  # RD preserved
    assert "/Tenant_A/n1%5:80" in new_src
    # The auth partition stanza header was renamed too.
    assert "auth partition Tenant_A" in new_src
    # iRule body reference picked up the rename.
    assert "pool /Tenant_A/web_pool" in new_src


def test_rename_partition_does_not_rewrite_address_octets():
    # The third octet of every address is 10, which matches "10" as a
    # bare number — we must not rewrite it when renaming "/Common/".
    src = "ltm node /Common/n1 { address 10.10.10.10 }\n"
    result = run_query('rename_partition("Common", "Tenant_A")', {"mem://1": src})
    new_src = result.edits_per_file["mem://1"].new_source
    assert "address 10.10.10.10" in new_src


def test_rename_partition_rejects_empty_or_invalid_names():
    with pytest.raises(Exception):
        run_query('rename_partition("", "X")', {"mem://1": SAMPLE_CONF})
    with pytest.raises(Exception):
        run_query('rename_partition("/Common", "X")', {"mem://1": SAMPLE_CONF})


def test_route_domain_round_trip():
    result = run_query(
        'with_route_domain("/Common/10.0.0.1%5:80", 7) ; '
        'with_route_domain("/Common/10.0.0.1%5:80", "") ; '
        'route_domain("/Common/10.0.0.1%5:80")',
        {"mem://1": SAMPLE_CONF},
    )
    # Only the last statement's values are returned by the runner.
    [rd] = result.values_per_file["mem://1"]
    assert rd == "5"


def test_ip_rebase_preserves_route_domain():
    result = run_query(
        '.ltm.virtual[].destination | ip("192.168.9.0/24", .)',
        {"mem://1": PARTITION_CONF},
    )
    [destination] = result.values_per_file["mem://1"]
    assert destination == "/Common/192.168.9.5%5:443"


def test_with_route_domain_strips_when_passed_empty():
    result = run_query(
        '.ltm.virtual[].destination |= with_route_domain(., "")',
        {"mem://1": PARTITION_CONF},
    )
    new_src = result.edits_per_file["mem://1"].new_source
    assert "destination /Common/10.10.0.5:443" in new_src
    assert "%5:443" not in new_src


def test_semicolon_statements_apply_in_order_last_write_wins():
    # Two statements writing to the same field — the runner applies
    # them in order against the evolving source, so the second
    # assignment lands on top of the first.
    result = _run(
        '.ltm.virtual.web_vs.destination = "1.1.1.1" ; .ltm.virtual.web_vs.destination = "2.2.2.2"'
    )
    new_src = result.edits_per_file["mem://1"].new_source
    assert "destination 2.2.2.2" in new_src
    assert "destination 1.1.1.1" not in new_src


def test_semicolon_lets_prefix_cascade_compose_with_field_edits():
    # Prefix-cascade + field edits in one statement is rejected, but
    # `;` splits them into separate statements that each see the
    # post-rewrite source.
    result = run_query(
        'rename_partition("Common", "Tenant_A") ; '
        '.ltm.virtual["/Tenant_A/web_vs"].destination = "9.9.9.9:443"',
        {"mem://1": PARTITION_CONF},
    )
    new_src = result.edits_per_file["mem://1"].new_source
    assert "destination 9.9.9.9:443" in new_src
    assert "ltm virtual /Tenant_A/web_vs" in new_src


def test_assignment_to_unknown_field_raises():
    with pytest.raises(EvalError):
        _run('.ltm.virtual.web_vs.no_such_field = "x"')


# ---------------------------------------------------------------------------
# Output rendering
# ---------------------------------------------------------------------------


def test_render_raw_strings():
    result = _run(".ltm.virtual[].name")
    rendered = render(result.values_per_file["mem://1"], mode="raw")
    assert rendered == "web_vs\napi_vs\n"


def test_render_paths_for_objects():
    result = _run(".ltm.virtual[]")
    rendered = render(result.values_per_file["mem://1"], mode="paths")
    assert rendered == "/Common/web_vs\n/Common/api_vs\n"


def test_render_scf_includes_header():
    result = _run(".ltm.virtual.web_vs")
    rendered = render(result.values_per_file["mem://1"], mode="scf")
    assert rendered.startswith("ltm virtual /Common/web_vs")


def test_render_json_emits_array():
    result = _run(".ltm.virtual[].name")
    rendered = render(result.values_per_file["mem://1"], mode="json")
    assert json.loads(rendered) == ["web_vs", "api_vs"]


def test_render_raw_rejects_object_refs():
    """``--raw`` is documented as scalars-only; an ObjectRef should
    raise rather than emit a Python ``repr`` that's useless for
    scripting.  Steer the user towards ``--paths-only`` / ``--json``.
    """
    result = _run(".ltm.virtual[]")
    with pytest.raises(ValueError, match=r"--raw cannot render"):
        render(result.values_per_file["mem://1"], mode="raw")


# ---------------------------------------------------------------------------
# Help surfaces
# ---------------------------------------------------------------------------


def test_format_grammar_returns_non_empty_reference():
    text = format_grammar()
    assert "GRAMMAR" in text
    assert "pipeline" in text


def test_format_builtins_lists_every_registered_function():
    text = format_builtins()
    for spec in list_builtins():
        assert spec.name in text


def test_format_builtins_drill_down_by_name():
    text = format_builtins("ip")
    assert "ip(addr: string)" in text
    assert "ip(network: string, source: string)" in text


def test_format_examples_contains_every_cookbook_entry():
    text = format_examples()
    for example in list_examples():
        assert example.title in text


def test_every_builtin_has_signature_summary_and_example():
    for spec in list_builtins():
        assert spec.summary, f"{spec.name}: missing summary"
        assert spec.signatures, f"{spec.name}: missing signatures"
        assert spec.examples, f"{spec.name}: missing examples"


# ---------------------------------------------------------------------------
# CLI verb
# ---------------------------------------------------------------------------


def _cli(args: list[str], capsys) -> tuple[int, str, str]:
    rc = main(args)
    captured = capsys.readouterr()
    return rc, captured.out, captured.err


@pytest.fixture
def sample_conf(tmp_path: Path) -> Path:
    p = tmp_path / "bigip.conf"
    p.write_text(SAMPLE_CONF, encoding="utf-8")
    return p


def test_cli_projects_names(sample_conf, capsys):
    rc, out, _ = _cli(["query", ".ltm.virtual[] | .name", str(sample_conf)], capsys)
    assert rc == 0
    assert "web_vs" in out and "api_vs" in out


def test_cli_dry_run_emits_diff(sample_conf, capsys):
    rc, out, _ = _cli(
        ["query", '.ltm.virtual.web_vs.pool = "/Common/other"', str(sample_conf)],
        capsys,
    )
    assert rc == 0
    assert "--- " in out and "+++ " in out
    # Source on disk is unchanged.
    assert "/Common/other" not in sample_conf.read_text()


def test_cli_write_emits_new_source(sample_conf, capsys):
    rc, out, _ = _cli(
        [
            "query",
            "--write",
            '.ltm.virtual.web_vs.pool = "/Common/other"',
            str(sample_conf),
        ],
        capsys,
    )
    assert rc == 0
    assert "pool /Common/other" in out


def test_cli_in_place_overwrites_file(sample_conf, capsys):
    rc, _, _ = _cli(
        [
            "query",
            "--in-place",
            '.ltm.virtual.web_vs.pool = "/Common/other"',
            str(sample_conf),
        ],
        capsys,
    )
    assert rc == 0
    assert "pool /Common/other" in sample_conf.read_text()


def test_cli_paths_only_mode(sample_conf, capsys):
    rc, out, _ = _cli(["query", "--paths-only", ".ltm.virtual[]", str(sample_conf)], capsys)
    assert rc == 0
    assert "/Common/web_vs" in out


def test_cli_rejects_duplicate_inputs(sample_conf, capsys):
    """A repeated input URI (most importantly ``-``, which would
    silently drain stdin then read empty on the second pass) is
    rejected with a clear error rather than producing a confusing
    empty-projection result.
    """
    rc, _, err = _cli(
        ["query", ".ltm.virtual[].name", str(sample_conf), str(sample_conf)],
        capsys,
    )
    assert rc == 2
    assert "duplicate input" in err


def test_cli_json_mode_skips_multi_file_banners(tmp_path, capsys):
    """``# === uri ===`` per-file banners break JSON output.  In
    multi-file ``--json`` mode the CLI must emit a single top-level
    JSON document keyed by URI — no interleaved comment lines and no
    adjacent arrays (which together would not be valid JSON).
    """
    import json as _json

    a = tmp_path / "a.conf"
    b = tmp_path / "b.conf"
    a.write_text("ltm node /Common/n_a { address 10.0.0.1 }\n", encoding="utf-8")
    b.write_text("ltm node /Common/n_b { address 10.0.0.2 }\n", encoding="utf-8")
    rc, out, _ = _cli(
        ["query", "--json", ".ltm.node[].name", str(a), str(b)],
        capsys,
    )
    assert rc == 0
    assert "# ===" not in out
    # Single top-level JSON document — must round-trip through
    # ``json.loads`` cleanly.  Adjacent ``[…][…]`` arrays would raise
    # ``json.JSONDecodeError``.
    parsed = _json.loads(out)
    assert isinstance(parsed, list)
    assert len(parsed) == 2
    by_uri = {entry["uri"]: entry["values"] for entry in parsed}
    # Each entry's values are the renderer's JSON for that file.
    assert all("uri" in entry and "values" in entry for entry in parsed)
    # The values come back as the renderer's structured output.
    a_values = by_uri[next(uri for uri in by_uri if uri.endswith("a.conf"))]
    assert a_values == ["n_a"]


def test_cli_exit_code_when_no_matches(sample_conf, capsys):
    rc, _, _ = _cli(["query", '.ltm.virtual["~no-match"] | .name', str(sample_conf)], capsys)
    assert rc == 1


def test_cli_parse_error_returns_two(sample_conf, capsys):
    rc, _, err = _cli(["query", "this is not a query", str(sample_conf)], capsys)
    assert rc == 2
    assert "error:" in err


def test_cli_exit_code_one_when_mutating_query_changes_nothing(sample_conf, capsys):
    # `rename_partition` on a partition the source doesn't use produces
    # no textual diff — exit code 1, matching the `f5 rename` "no
    # occurrence" convention.
    rc, _, _ = _cli(
        ["query", 'rename_partition("Nonexistent", "Other")', str(sample_conf)],
        capsys,
    )
    assert rc == 1


def test_tilde_prefix_string_outside_subscript_is_a_plain_string():
    # The lexer used to emit a REGEX token for any string starting with
    # ``~``, which made it impossible to pass a literal starting with
    # ``~`` to a builtin.  The fix lexes it as a plain STRING and only
    # recognises the regex form inside ``[ ... ]``.
    result = _run('contains("~something", "~")')
    [matched] = result.values_per_file["mem://1"]
    assert matched is True


def test_object_cache_disambiguates_kinds_sharing_a_path(tmp_path: Path):
    # A pool and an iRule sharing the same full-path used to collide in
    # the cache; field access on the second kind would return the
    # first kind's ObjectRef and fail with "no field".
    src = (
        "ltm pool /Common/shared {\n"
        "    monitor /Common/http\n"
        "}\n"
        "ltm rule /Common/shared {\n"
        "when HTTP_REQUEST { pool /Common/shared }\n"
        "}\n"
    )
    result = run_query(
        '.ltm.pool["/Common/shared"].monitor ; .ltm.rule["/Common/shared"].body',
        {"mem://1": src},
    )
    # Both kinds resolved cleanly under the same path.
    assert result.values_per_file["mem://1"]


def test_from_file_promotes_expression_when_extra_paths_given(tmp_path: Path, capsys):
    # ``f5 query -f q.fq a.conf b.conf`` used to silently drop ``a.conf``
    # because argparse parked it in ``expression``.  The fix promotes
    # it back into ``paths`` whenever --from-file is present.
    conf_a = tmp_path / "a.conf"
    conf_a.write_text(SAMPLE_CONF, encoding="utf-8")
    conf_b = tmp_path / "b.conf"
    conf_b.write_text(SAMPLE_CONF, encoding="utf-8")
    script = tmp_path / "q.fq"
    script.write_text(".ltm.virtual[].name", encoding="utf-8")

    rc, out, _ = _cli(["query", "-f", str(script), str(conf_a), str(conf_b)], capsys)
    assert rc == 0
    # Both files were processed; output includes per-file headers.
    assert str(conf_a) in out or "a.conf" in out
    assert str(conf_b) in out or "b.conf" in out


def test_cli_help_dsl(capsys):
    with pytest.raises(SystemExit):
        main(["query", "--help-dsl"])
    out = capsys.readouterr().out
    assert "GRAMMAR" in out


def test_cli_help_builtins(capsys):
    with pytest.raises(SystemExit):
        main(["query", "--help-builtins"])
    out = capsys.readouterr().out
    assert "BUILTIN FUNCTIONS" in out
    assert "ip" in out


def test_cli_help_builtins_by_name(capsys):
    with pytest.raises(SystemExit):
        main(["query", "--help-builtins", "ip"])
    out = capsys.readouterr().out
    assert "ip(addr: string)" in out


# ---------------------------------------------------------------------------
# jq-compatibility idioms
# ---------------------------------------------------------------------------


def test_list_literal_collects_a_stream():
    [n] = _run("[.ltm.virtual[]] | length").values_per_file["mem://1"]
    assert n == 2


def test_list_literal_unwraps_for_aggregators():
    result = _run("[.ltm.virtual[].name] | sort | first")
    assert result.values_per_file["mem://1"] == ["api_vs"]


def test_pipe_iterates_streams_not_plain_lists():
    # `.rules` is a list (not Stream) — the pipe passes it whole to
    # length rather than iterating.
    [n] = _run(".ltm.virtual.web_vs.rules | length").values_per_file["mem://1"]
    assert n == 1  # web_vs has one rule


def test_bare_builtin_implicit_dot():
    # `length` with no parens is sugar for `length(.)`.
    [n] = _run(".ltm.virtual.web_vs.rules | length").values_per_file["mem://1"]
    assert n == 1


def test_bare_builtin_in_select_predicate():
    result = _run(".ltm.virtual[] | select(.rules | count > 0) | .name")
    assert result.values_per_file["mem://1"] == ["web_vs"]


def test_postfix_subscript_after_call():
    # `refs(.)[]` iterates the list returned by refs.
    result = _run("[.ltm.virtual.web_vs | refs(.)[]] | sort")
    [deps] = result.values_per_file["mem://1"]
    assert "/Common/web_pool" in deps
    assert "/Common/log_rule" in deps


def test_any_over_stream_no_map_needed():
    # The jq idiom for "any value in CIDR" — pipe iterates the
    # stream, in_cidr produces booleans, any() collapses.  Both
    # VS destinations are in 10.10.0.0/16, so any() is true.
    [hit] = _run('any(.ltm.virtual[].destination | in_cidr(., "10.10.0.0/16"))').values_per_file[
        "mem://1"
    ]
    assert hit is True
    # And false when the network excludes them.
    [miss] = _run('any(.ltm.virtual[].destination | in_cidr(., "192.168.0.0/16"))').values_per_file[
        "mem://1"
    ]
    assert miss is False


def test_empty_list_literal():
    [empty] = _run("[]").values_per_file["mem://1"]
    assert empty == []


# ---------------------------------------------------------------------------
# Auto-rendering for non-object value kinds
# ---------------------------------------------------------------------------


def test_auto_renders_integer_one_per_line():
    result = _run("[.ltm.virtual[]] | length")
    rendered = render(result.values_per_file["mem://1"], mode="auto")
    assert rendered == "2\n"


def test_auto_renders_string_one_per_line():
    result = _run(".ltm.virtual.web_vs.name")
    rendered = render(result.values_per_file["mem://1"], mode="auto")
    assert rendered == "web_vs\n"


def test_auto_renders_pathref_one_per_line():
    result = _run(".ltm.virtual.web_vs.pool")
    rendered = render(result.values_per_file["mem://1"], mode="auto")
    assert rendered == "/Common/web_pool\n"


def test_auto_flattens_list_of_scalars():
    # ``[.X[].name]`` produces one list-of-strings.  Auto mode used
    # to fall through to JSON; now it flattens so the output matches
    # the equivalent stream form ``.X[].name``.
    result = _run("[.ltm.virtual[].name]")
    rendered = render(result.values_per_file["mem://1"], mode="auto")
    assert rendered == "web_vs\napi_vs\n"


def test_null_renders_as_literal_token():
    # jq's ``--raw-output`` semantics: ``null`` is a literal, not an
    # empty line.  Lets users distinguish "no port" from "empty port".
    result = _run('port("/Common/no-port")')
    rendered = render(result.values_per_file["mem://1"], mode="raw")
    assert rendered == "null\n"


# ---------------------------------------------------------------------------
# Rename / partition reporting
# ---------------------------------------------------------------------------


def test_rename_partition_count_includes_header():
    src = (
        "auth partition Common { description default }\n"
        "ltm pool /Common/p1 { monitor /Common/http }\n"
    )
    result = run_query('rename_partition("Common", "Tenant_A")', {"m": src})
    [count] = result.values_per_file["m"]
    # Two ``/Common/`` prefix matches (``/Common/p1`` header,
    # ``/Common/http`` monitor reference) plus one
    # ``auth partition Common`` header rewrite.  Before the fix, only
    # the prefix matches were counted.
    assert count == 3
    # And the on-disk rewrite did land on all three.
    applied = result.edits_per_file["m"]
    assert "/Common/" not in applied.new_source
    assert "auth partition Tenant_A" in applied.new_source


def test_prefix_rewrite_report_does_not_leak_regex_backrefs():
    src = "auth partition Common { description default }\n"
    result = run_query('rename_partition("Common", "Tenant_A")', {"m": src})
    new_strings = [rep.new for rep in result.edits_per_file["m"].rename_reports]
    # The header rewrite uses a regex backref (\g<1>) internally;
    # the report's ``new`` field must show the human-readable form.
    assert all("\\g<" not in s for s in new_strings)
    assert any("auth partition Tenant_A" == s for s in new_strings)


def test_every_builtin_has_a_details_block():
    """Catch regressions where a new builtin is added without prose docs."""
    for spec in list_builtins():
        assert spec.details, f"{spec.name}: missing details block"


# ---------------------------------------------------------------------------
# Reported issues — minimal-SCF regression tests
# ---------------------------------------------------------------------------


# Issue 3 — `sort` in the documented array-collection form.
_SORT_SCF = (
    "ltm virtual /Common/zeta { destination /Common/192.0.2.4:80 pool /Common/p }\n"
    "ltm virtual /Common/alpha { destination /Common/192.0.2.5:80 pool /Common/p }\n"
    "ltm virtual /Common/mu    { destination /Common/192.0.2.6:80 pool /Common/p }\n"
    "ltm pool /Common/p {}\n"
)


def test_sort_array_construct_form_works():
    """The jq-canonical ``[ .X[].name ] | sort`` form returns a sorted list."""
    result = run_query("[ .ltm.virtual[].name ] | sort", {"m": _SORT_SCF})
    [names] = result.values_per_file["m"]
    assert names == ["alpha", "mu", "zeta"]


def test_sort_bare_after_stream_errors_clearly():
    """``.X[].name | sort`` runs ``sort`` per name (each a single string),
    not over the stream.  The current behaviour is a clear ``BuiltinError``
    — better than silently producing one-element lists per item — and the
    docs steer users to the ``[...] | sort`` form above.
    """
    from core.bigip.query.errors import BuiltinError

    with pytest.raises(BuiltinError):
        run_query(".ltm.virtual[].name | sort", {"m": _SORT_SCF})


# Issue 1 — Cross-kind rename collision.
_COLLISION_SCF = (
    "ltm pool /Common/shared_name {\n"
    "    members {\n"
    "        /Common/n1:80 { address 192.0.2.10 }\n"
    "    }\n"
    "    monitor /Common/http\n"
    "}\n"
    "ltm virtual /Common/shared_name {\n"
    "    destination /Common/198.51.100.5:443\n"
    "    ip-protocol tcp\n"
    "    mask 255.255.255.255\n"
    "    pool /Common/shared_name\n"
    "}\n"
)


def test_identity_rename_is_kind_scoped_for_pool():
    """Pool+VS sharing a full-path: renaming the pool must leave the
    VS header alone.  Regression for the v1.9.0-14 bug where the
    bare token-bounded regex rewrote both headers.
    """
    result = run_query(
        '.ltm.pool["/Common/shared_name"].name = "/Common/new_pool"',
        {"m": _COLLISION_SCF},
    )
    applied = result.edits_per_file["m"]
    new_src = applied.new_source
    # Pool stanza header moved.
    assert "ltm pool /Common/new_pool" in new_src
    # VS header is preserved — the rename was kind-scoped.
    assert "ltm virtual /Common/shared_name" in new_src
    # VS's pool reference still updates (it points at the renamed pool).
    assert "pool /Common/new_pool" in new_src
    # Count is 2: pool header + pool reference inside VS body.  The
    # legacy pre-fix value was 3 (header + ref + wrong VS header).
    assert applied.rename_reports[0].occurrences == 2


def test_identity_rename_is_kind_scoped_for_virtual():
    """Symmetric case: renaming the VS leaves the pool header alone."""
    result = run_query(
        '.ltm.virtual["/Common/shared_name"].name = "/Common/new_vs"',
        {"m": _COLLISION_SCF},
    )
    new_src = result.edits_per_file["m"].new_source
    assert "ltm virtual /Common/new_vs" in new_src
    assert "ltm pool /Common/shared_name" in new_src
    # The VS's pool reference matches the pool's name, which has *not*
    # been renamed — so ``pool /Common/shared_name`` stays.
    assert "pool /Common/shared_name" in new_src


def test_bare_rename_builtin_is_unscoped():
    """The ``rename()`` builtin / ``f5 rename`` CLI doesn't know which
    kind the caller meant, so it keeps the legacy global rewrite.
    Users who need kind-scoping reach for the DSL identity-field
    form above.
    """
    result = run_query(
        'rename("/Common/shared_name", "/Common/new")',
        {"m": _COLLISION_SCF},
    )
    new_src = result.edits_per_file["m"].new_source
    # Both stanza headers move under the global rewrite — surprising
    # but that's the documented behaviour of the bare rename.
    assert "ltm pool /Common/new" in new_src
    assert "ltm virtual /Common/new" in new_src


# ---------------------------------------------------------------------------
# ltm policy.rules[] with nested conditions / actions
# ---------------------------------------------------------------------------

_POLICY_SCF = (
    "ltm pool /AS3/app/p_home { }\n"
    "ltm policy /AS3/app/policy_routing {\n"
    "    controls { forwarding }\n"
    "    requires { http }\n"
    "    rules {\n"
    "        HomePage {\n"
    "            actions {\n"
    "                0 {\n"
    "                    forward\n"
    "                    select\n"
    "                    pool /AS3/app/p_home\n"
    "                }\n"
    "            }\n"
    "            conditions {\n"
    "                0 {\n"
    "                    http-uri\n"
    "                    path\n"
    "                    starts-with\n"
    "                    values { /HomePage }\n"
    "                }\n"
    "            }\n"
    "            ordinal 5\n"
    "        }\n"
    "        Default {\n"
    "            actions {\n"
    "                0 {\n"
    "                    shutdown\n"
    "                    connection\n"
    "                }\n"
    "            }\n"
    "            ordinal 1\n"
    "        }\n"
    "    }\n"
    "    strategy /Common/first-match\n"
    "}\n"
)


def test_ltm_policy_projects_rules_with_names_and_ordinals():
    """``.ltm.policy[].rules[]`` exposes each named rule with its
    ``ordinal`` and ``name`` fields.
    """
    result = run_query(".ltm.policy[].rules[].name", {"m": _POLICY_SCF})
    assert sorted(result.values_per_file["m"]) == ["Default", "HomePage"]
    result = run_query(".ltm.policy[].rules[].ordinal", {"m": _POLICY_SCF})
    assert sorted(result.values_per_file["m"]) == [1, 5]


def test_ltm_policy_rule_conditions_expose_operand_and_operator():
    """``rules[].conditions[]`` surfaces the classified operand /
    selector / operator / values for each condition entry."""
    result = run_query(".ltm.policy[].rules[].conditions[].operand", {"m": _POLICY_SCF})
    assert result.values_per_file["m"] == ["http-uri"]
    result = run_query(".ltm.policy[].rules[].conditions[].operator", {"m": _POLICY_SCF})
    assert result.values_per_file["m"] == ["starts-with"]
    result = run_query(".ltm.policy[].rules[].conditions[].values", {"m": _POLICY_SCF})
    [vals] = result.values_per_file["m"]
    assert list(vals) == ["/HomePage"]


def test_ltm_policy_rule_action_pool_pathref_walks_into_ltm_pool():
    """``rules[].actions[].pool`` is a PathRef into ``ltm pool`` —
    chaining ``.full-path`` resolves through the action into the
    target pool's identity field."""
    result = run_query(
        ".ltm.policy[].rules[].actions[].pool.full-path",
        {"m": _POLICY_SCF},
    )
    assert result.values_per_file["m"] == ["/AS3/app/p_home"]


def test_ltm_policy_rule_actions_carry_verb_and_target():
    """Surface each action's classified ``target`` / ``verb`` —
    ``forward select`` for the routing rule, ``shutdown`` (no verb
    captured) for the default deny."""
    result = run_query(".ltm.policy[].rules[].actions[].target", {"m": _POLICY_SCF})
    assert sorted(result.values_per_file["m"]) == ["forward", "shutdown"]


# ---------------------------------------------------------------------------
# net.* — typed projection for the network module
# ---------------------------------------------------------------------------


_NET_SCF = (
    "net vlan /Common/external {\n"
    "    interfaces {\n"
    "        1.1 { }\n"
    "    }\n"
    "    tag 4093\n"
    "}\n"
    "net vlan /Common/internal {\n"
    "    interfaces {\n"
    "        1.2 { }\n"
    "    }\n"
    "    tag 4094\n"
    "}\n"
    "net self /Common/198.51.100.5 {\n"
    "    address 198.51.100.5/24\n"
    "    allow-service {\n"
    "        default\n"
    "    }\n"
    "    traffic-group /Common/traffic-group-local-only\n"
    "    vlan /Common/external\n"
    "}\n"
    "net self /Common/203.0.113.5 {\n"
    "    address 203.0.113.5/24\n"
    "    traffic-group /Common/traffic-group-1\n"
    "    vlan /Common/internal\n"
    "}\n"
    "net route /Common/default_gw {\n"
    "    gw 198.51.100.1\n"
    "    network default\n"
    "}\n"
    "net route-domain /Common/0 {\n"
    "    id 0\n"
    "    vlans {\n"
    "        /Common/external\n"
    "        /Common/internal\n"
    "    }\n"
    "}\n"
    "net port-list /Common/web_ports {\n"
    "    ports {\n"
    "        80 { }\n"
    "        443 { }\n"
    "    }\n"
    "}\n"
)


def test_net_vlan_projects_tag_and_interfaces():
    result = run_query(".net.vlan[].tag", {"m": _NET_SCF})
    assert sorted(result.values_per_file["m"]) == [4093, 4094]
    result = run_query('.net.vlan["/Common/external"].interfaces', {"m": _NET_SCF})
    [ifaces] = result.values_per_file["m"]
    assert list(ifaces) == ["1.1"]


def test_net_self_projects_address_and_traffic_group():
    result = run_query(
        '.net.self[] | select(.traffic-group == "/Common/traffic-group-1") | .address',
        {"m": _NET_SCF},
    )
    assert result.values_per_file["m"] == ["203.0.113.5/24"]


def test_net_self_vlan_pathref_auto_dereferences():
    """The ``.vlan`` field on ``net self`` is a PathRef into ``net vlan``;
    chaining ``.interfaces`` should follow the ref into the VLAN object
    and pick up its ``interfaces`` list.
    """
    result = run_query(".net.self[].vlan.interfaces", {"m": _NET_SCF})
    flat = [item for sub in result.values_per_file["m"] for item in sub]
    # Two selves, each on a one-interface VLAN.
    assert sorted(flat) == ["1.1", "1.2"]


def test_net_route_domain_vlans_is_a_list_of_pathrefs():
    result = run_query('.net.route-domain["/Common/0"].vlans[].tag', {"m": _NET_SCF})
    assert sorted(result.values_per_file["m"]) == [4093, 4094]


def test_net_route_projects_gw_and_network():
    result = run_query(".net.route[].gw", {"m": _NET_SCF})
    assert result.values_per_file["m"] == ["198.51.100.1"]
    result = run_query(".net.route[].network", {"m": _NET_SCF})
    assert result.values_per_file["m"] == ["default"]


def test_net_port_list_projects_ports():
    result = run_query('.net.port-list["/Common/web_ports"].ports', {"m": _NET_SCF})
    [ports] = result.values_per_file["m"]
    assert sorted(ports) == ["443", "80"]


def test_net_self_traffic_group_pathref_walks_into_cm_traffic_group():
    """``net self.traffic-group`` PathRefs into ``cm traffic-group``;
    chaining ``.unit-id`` should pull the trafficked unit id.  This
    is a *cross-module* PathRef (``net`` → ``cm``).
    """
    src = _NET_SCF + (
        "cm traffic-group /Common/traffic-group-1 {\n"
        "    unit-id 1\n"
        "}\n"
        "cm traffic-group /Common/traffic-group-local-only { }\n"
    )
    result = run_query(
        '.net.self[] | select(.traffic-group.full-path == "/Common/traffic-group-1") '
        "| .traffic-group.unit-id",
        {"m": src},
    )
    assert result.values_per_file["m"] == ["1"]


def test_net_identity_rename_is_kind_scoped():
    """``.net.vlan["X"].name = "Y"`` must not touch references in
    other modules — same kind-scope correctness as the ltm/virtual
    pool-collision case (Issue 1)."""
    result = run_query(
        '.net.vlan["/Common/external"].name = "/Common/outside"',
        {"m": _NET_SCF},
    )
    new_src = result.edits_per_file["m"].new_source
    assert "net vlan /Common/outside" in new_src
    # The ``net self`` references update — they're plain pathref tokens.
    assert "vlan /Common/outside" in new_src
    # And the route-domain's list-of-vlans reference moved too.
    assert "/Common/outside\n" in new_src
    # No stray /Common/external left.
    assert "/Common/external" not in new_src


# .net extensions: interface, dns-resolver, tunnels-tunnel, stp.

_NET_EXT_SCF = (
    "net interface 1.1 {\n"
    "    media-fixed 10000T-FD\n"
    "}\n"
    "net interface 1.2 {\n"
    "    media-fixed 1000T-FD\n"
    "}\n"
    "net route-domain /Common/0 {\n"
    "    id 0\n"
    "}\n"
    "net dns-resolver /Common/dns_default {\n"
    "    route-domain /Common/0\n"
    "    forward-zones {\n"
    "        example.test {\n"
    "            nameservers {\n"
    "                192.0.2.53:53 { }\n"
    "            }\n"
    "        }\n"
    "        internal.test {\n"
    "            nameservers {\n"
    "                198.51.100.53:53 { }\n"
    "            }\n"
    "        }\n"
    "    }\n"
    "}\n"
    "ltm profile tcp-forward /Common/tcp-forward { }\n"
    "net tunnels tunnel /Common/http_tunnel {\n"
    "    profile /Common/tcp-forward\n"
    "    local-address 192.0.2.10\n"
    "    remote-address 203.0.113.20\n"
    '    description "Tunnel for http-explicit profile"\n'
    "}\n"
    "net stp /Common/cist {\n"
    "    interfaces {\n"
    "        1.1 { }\n"
    "        1.2 { }\n"
    "    }\n"
    "}\n"
)


def test_net_interface_projects_media_fixed():
    result = run_query(".net.interface[].media-fixed", {"m": _NET_EXT_SCF})
    assert sorted(result.values_per_file["m"]) == ["10000T-FD", "1000T-FD"]


def test_net_interface_indexed_by_bare_slot_port():
    """Interfaces use bare slot/port (``1.1``) — no ``/Common/`` prefix."""
    result = run_query('.net.interface["1.1"].media-fixed', {"m": _NET_EXT_SCF})
    assert result.values_per_file["m"] == ["10000T-FD"]


def test_net_dns_resolver_projects_forward_zones():
    result = run_query(
        '.net.dns-resolver["/Common/dns_default"].forward-zones',
        {"m": _NET_EXT_SCF},
    )
    [zones] = result.values_per_file["m"]
    assert sorted(zones) == ["example.test", "internal.test"]


def test_net_dns_resolver_route_domain_pathref():
    """The ``route-domain`` field is a PathRef into ``net route-domain``;
    chaining ``.id`` should follow the ref into the RD object.
    """
    result = run_query(
        ".net.dns-resolver[].route-domain.id",
        {"m": _NET_EXT_SCF},
    )
    assert result.values_per_file["m"] == [0]


def test_net_tunnel_projects_addresses_and_description():
    result = run_query(
        '.net.tunnels-tunnel["/Common/http_tunnel"].local-address',
        {"m": _NET_EXT_SCF},
    )
    assert result.values_per_file["m"] == ["192.0.2.10"]
    result = run_query(
        '.net.tunnels-tunnel["/Common/http_tunnel"].remote-address',
        {"m": _NET_EXT_SCF},
    )
    assert result.values_per_file["m"] == ["203.0.113.20"]
    result = run_query(
        '.net.tunnels-tunnel["/Common/http_tunnel"].description',
        {"m": _NET_EXT_SCF},
    )
    # Outer quotes are stripped during parse.
    assert result.values_per_file["m"] == ["Tunnel for http-explicit profile"]


def test_net_tunnel_profile_pathref():
    """``net tunnels tunnel`` is a two-word kind; the parser must
    recognise it so the projection sees a populated ``profile`` field.
    """
    result = run_query(
        ".net.tunnels-tunnel[].profile",
        {"m": _NET_EXT_SCF},
    )
    [profile] = result.values_per_file["m"]
    assert profile.full_path == "/Common/tcp-forward"


def test_net_stp_projects_interfaces():
    result = run_query('.net.stp["/Common/cist"].interfaces', {"m": _NET_EXT_SCF})
    [ifaces] = result.values_per_file["m"]
    assert sorted(ifaces) == ["1.1", "1.2"]


# ---------------------------------------------------------------------------
# sys.* — typed projection for the system module
# ---------------------------------------------------------------------------

_SYS_SCF = (
    "sys dns {\n"
    "    name-servers { 192.0.2.53 198.51.100.53 }\n"
    "    search { example.test internal.test }\n"
    "}\n"
    "sys ntp {\n"
    "    servers { 192.0.2.123 }\n"
    "    timezone UTC\n"
    "}\n"
    "sys snmp {\n"
    "    agent-addresses { tcp6:161 udp6:161 }\n"
    "    communities {\n"
    "        /Common/comm-public {\n"
    "            community-name public\n"
    "        }\n"
    "    }\n"
    "}\n"
    "sys global-settings {\n"
    "    gui-setup disabled\n"
    "    hostname host1.example.test\n"
    "    mgmt-dhcp disabled\n"
    "}\n"
    "sys provision ltm {\n"
    "    level nominal\n"
    "}\n"
    "sys provision sslo {\n"
    "    level minimum\n"
    "}\n"
    "sys folder / {\n"
    "    device-group none\n"
    "    hidden false\n"
    "    traffic-group /Common/traffic-group-1\n"
    "}\n"
    "sys folder /Common {\n"
    "    device-group none\n"
    "    hidden false\n"
    "    traffic-group /Common/traffic-group-1\n"
    "}\n"
    "sys file ssl-cert /Common/host1.crt {\n"
    "    cache-path /config/filestore/files_d/Common_d/certificate_d/host1.crt_1\n"
    "    revision 1\n"
    "    source-path file:///config/ssl/ssl.crt/host1.crt\n"
    "}\n"
    "sys file ssl-key /Common/host1.key {\n"
    "    cache-path /config/filestore/files_d/Common_d/certificate_key_d/host1.key_1\n"
    "    passphrase $M$placeholder\n"
    "    revision 1\n"
    "    source-path file:///config/ssl/ssl.key/host1.key\n"
    "}\n"
    "sys management-route /Common/default {\n"
    "    description configured-statically\n"
    "    gateway 192.0.2.1\n"
    "    mtu 1500\n"
    "    network default\n"
    "}\n"
)


def test_sys_dns_singleton_projects_name_servers():
    """``sys dns`` has no full-path; it's a singleton streamed via ``[]``."""
    result = run_query(".sys.dns[].name-servers", {"m": _SYS_SCF})
    [servers] = result.values_per_file["m"]
    assert sorted(servers) == ["192.0.2.53", "198.51.100.53"]


def test_sys_dns_singleton_lookup_by_empty_key():
    """The singleton lives at the empty-string key."""
    result = run_query('.sys.dns[""].search', {"m": _SYS_SCF})
    [search] = result.values_per_file["m"]
    assert sorted(search) == ["example.test", "internal.test"]


def test_sys_ntp_singleton_projects_servers():
    result = run_query(".sys.ntp[].servers", {"m": _SYS_SCF})
    [servers] = result.values_per_file["m"]
    assert list(servers) == ["192.0.2.123"]
    result = run_query(".sys.ntp[].timezone", {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["UTC"]


def test_sys_snmp_singleton_projects_agent_and_communities():
    result = run_query(".sys.snmp[].agent-addresses", {"m": _SYS_SCF})
    [addrs] = result.values_per_file["m"]
    assert sorted(addrs) == ["tcp6:161", "udp6:161"]
    # Communities is a sub-block; we surface the keys as a list.
    result = run_query(".sys.snmp[].communities", {"m": _SYS_SCF})
    [comms] = result.values_per_file["m"]
    assert list(comms) == ["/Common/comm-public"]


def test_sys_global_settings_singleton_projects_hostname():
    result = run_query(".sys.global-settings[].hostname", {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["host1.example.test"]


def test_sys_provision_projects_level():
    result = run_query(".sys.provision[].level", {"m": _SYS_SCF})
    assert sorted(result.values_per_file["m"]) == ["minimum", "nominal"]
    result = run_query('.sys.provision["ltm"].level', {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["nominal"]


def test_sys_folder_projects_traffic_group():
    """``sys folder /`` uses ``/`` as its full-path key — make sure
    indexing works for both the root folder and named partitions.
    ``traffic-group`` is a PathRef into ``cm traffic-group``.
    """
    result = run_query('.sys.folder["/"].traffic-group', {"m": _SYS_SCF})
    [tg] = result.values_per_file["m"]
    assert tg.full_path == "/Common/traffic-group-1"
    result = run_query('.sys.folder["/Common"].name', {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["Common"]


def test_sys_file_ssl_cert_projects_source_path():
    """``sys file ssl-cert`` is a two-word kind; the parser must
    recognise it so the source-path field comes through populated.
    """
    result = run_query(
        '.sys.file-ssl-cert["/Common/host1.crt"].source-path',
        {"m": _SYS_SCF},
    )
    assert result.values_per_file["m"] == ["file:///config/ssl/ssl.crt/host1.crt"]


def test_sys_file_ssl_key_projects_source_path_and_passphrase():
    result = run_query(
        '.sys.file-ssl-key["/Common/host1.key"].source-path',
        {"m": _SYS_SCF},
    )
    assert result.values_per_file["m"] == ["file:///config/ssl/ssl.key/host1.key"]
    result = run_query(
        '.sys.file-ssl-key["/Common/host1.key"].passphrase',
        {"m": _SYS_SCF},
    )
    assert result.values_per_file["m"] == ["$M$placeholder"]


def test_sys_management_route_projects_gateway_and_description():
    result = run_query(".sys.management-route[].gateway", {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["192.0.2.1"]
    result = run_query(".sys.management-route[].description", {"m": _SYS_SCF})
    assert result.values_per_file["m"] == ["configured-statically"]


# ---------------------------------------------------------------------------
# security.* — typed projection for AFM / inspection / device-id
# ---------------------------------------------------------------------------

_SECURITY_SCF = (
    "security firewall port-list /Common/web_ports {\n"
    "    ports {\n"
    "        80 { }\n"
    "        443 { }\n"
    "        8080 { }\n"
    "    }\n"
    "}\n"
    "security firewall rule-list /Common/rl_web {\n"
    "    rules {\n"
    "        allow_http {\n"
    "            action accept\n"
    "            ip-protocol tcp\n"
    "        }\n"
    "        allow_https {\n"
    "            action accept\n"
    "            ip-protocol tcp\n"
    "        }\n"
    "    }\n"
    "}\n"
    "security firewall config-entity-id /Common/uuid_entity_id {\n"
    "    entity-id 8903696776153557482\n"
    "}\n"
    "security ip-intelligence policy /Common/ip-intelligence { }\n"
    "security protocol-inspection compliance-map /Common/map_10426 {\n"
    "    insp-id 10426\n"
    "    key-type int\n"
    "    value-type vector-string\n"
    "}\n"
    "security protocol-inspection compliance-objects /Common/allowed_ip_addresses {\n"
    "    insp-id 11800\n"
    "    type vector-string\n"
    "}\n"
    "security device-id attribute /Common/att01 {\n"
    "    id 1\n"
    "}\n"
    "security device-id attribute /Common/att02 {\n"
    "    id 2\n"
    "}\n"
)


def test_security_firewall_port_list_projects_ports():
    """``security firewall port-list`` is a two-word kind; the parser
    must recognise it so the ``ports`` list comes through.
    """
    result = run_query(
        '.security.firewall-port-list["/Common/web_ports"].ports',
        {"m": _SECURITY_SCF},
    )
    [ports] = result.values_per_file["m"]
    assert sorted(ports) == ["443", "80", "8080"]


def test_security_firewall_rule_list_projects_rule_names():
    result = run_query(
        '.security.firewall-rule-list["/Common/rl_web"].rules',
        {"m": _SECURITY_SCF},
    )
    [rules] = result.values_per_file["m"]
    assert sorted(rules) == ["allow_http", "allow_https"]


def test_security_firewall_config_entity_id_projects_entity_id():
    result = run_query(
        ".security.firewall-config-entity-id[].entity-id",
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["8903696776153557482"]


def test_security_ip_intelligence_policy_projects_name():
    """Empty-body stanza — we still surface name/full-path."""
    result = run_query(
        ".security.ip-intelligence-policy[].name",
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["ip-intelligence"]


def test_security_protocol_inspection_compliance_map_projects_insp_id():
    result = run_query(
        ".security.protocol-inspection-compliance-map[].insp-id",
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["10426"]
    result = run_query(
        ".security.protocol-inspection-compliance-map[].key-type",
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["int"]


def test_security_protocol_inspection_compliance_objects_projects_type():
    result = run_query(
        ".security.protocol-inspection-compliance-objects[].type",
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["vector-string"]


def test_security_device_id_attribute_projects_id():
    result = run_query(".security.device-id-attribute[].id", {"m": _SECURITY_SCF})
    assert sorted(result.values_per_file["m"]) == ["1", "2"]
    result = run_query(
        '.security.device-id-attribute["/Common/att01"].id',
        {"m": _SECURITY_SCF},
    )
    assert result.values_per_file["m"] == ["1"]


# ---------------------------------------------------------------------------
# apm.* — typed projection for Access Policy Manager
# ---------------------------------------------------------------------------

_APM_SCF = (
    "apm ephemeral-auth ssh-security-config /Common/ssh-cfg {\n"
    "    ciphers {\n"
    "        1 {\n"
    "            cipher-name aes256-ctr\n"
    "        }\n"
    "        2 {\n"
    "            cipher-name aes192-ctr\n"
    "        }\n"
    "    }\n"
    "    hmacs {\n"
    "        1 {\n"
    "            hmac-name hmac-sha2-512\n"
    "        }\n"
    "    }\n"
    "    kex-methods {\n"
    "        1 {\n"
    "            kex-method-name ecdh-sha2-nistp256\n"
    "        }\n"
    "    }\n"
    "    compressions {\n"
    "        1 {\n"
    "            compression-name none\n"
    "        }\n"
    "    }\n"
    "}\n"
    "apm oauth db-instance /Common/oauthdb {\n"
    '    description "Default OAuth DB."\n'
    "}\n"
    "apm policy access-policy /Common/policy_default {\n"
    "    default-ending /Common/policy_default_end_deny\n"
    "    items {\n"
    "        /Common/policy_default_act_auth { }\n"
    "        /Common/policy_default_end_allow { }\n"
    "        /Common/policy_default_end_deny { }\n"
    "        /Common/policy_default_ent { }\n"
    "    }\n"
    "    start-item /Common/policy_default_ent\n"
    "}\n"
    "apm policy customization-source /Common/modern { }\n"
    "apm policy customization-source /Common/standard { }\n"
    "apm policy policy-item /Common/policy_default_act_auth {\n"
    "    agents {\n"
    "        /Common/policy_default_act_auth_ag {\n"
    "            type aaa-kerberos\n"
    "        }\n"
    "    }\n"
    '    caption "Kerberos Auth"\n'
    "    color 1\n"
    "    item-type action\n"
    "}\n"
    "apm policy policy-item /Common/policy_default_end_allow {\n"
    "    agents {\n"
    "        /Common/policy_default_end_allow_ag {\n"
    "            type ending-allow\n"
    "        }\n"
    "    }\n"
    "    caption Allow\n"
    "    color 1\n"
    "    item-type ending\n"
    "}\n"
    "apm policy policy-item /Common/policy_default_end_deny {\n"
    "    agents {\n"
    "        /Common/policy_default_end_deny_ag {\n"
    "            type ending-deny\n"
    "        }\n"
    "    }\n"
    "    caption Deny\n"
    "    color 2\n"
    "    item-type ending\n"
    "}\n"
    "apm policy policy-item /Common/policy_default_ent {\n"
    "    caption Start\n"
    "    color 1\n"
    "}\n"
    "apm policy agent ending-allow /Common/policy_default_end_allow_ag { }\n"
    "apm policy agent ending-deny /Common/policy_default_end_deny_ag {\n"
    "    customization-group /Common/policy_default_end_deny_ag\n"
    "}\n"
    "apm policy agent kerberos /Common/policy_default_act_auth_ag { }\n"
    "apm report default-report {\n"
    "    report-name sessionReports/sessionSummary\n"
    "    user /Common/admin\n"
    "}\n"
)


def test_apm_ssh_security_config_projects_ciphers():
    result = run_query(".apm.ssh-security-config[].ciphers", {"m": _APM_SCF})
    [ciphers] = result.values_per_file["m"]
    assert list(ciphers) == ["aes256-ctr", "aes192-ctr"]


def test_apm_ssh_security_config_projects_hmacs_kex_compressions():
    result = run_query(".apm.ssh-security-config[].hmacs", {"m": _APM_SCF})
    [hmacs] = result.values_per_file["m"]
    assert list(hmacs) == ["hmac-sha2-512"]
    result = run_query(".apm.ssh-security-config[].kex-methods", {"m": _APM_SCF})
    [kex] = result.values_per_file["m"]
    assert list(kex) == ["ecdh-sha2-nistp256"]
    result = run_query(".apm.ssh-security-config[].compressions", {"m": _APM_SCF})
    [comp] = result.values_per_file["m"]
    assert list(comp) == ["none"]


def test_apm_oauth_db_instance_projects_description():
    """Description is quoted in the source — outer quotes are stripped."""
    result = run_query(".apm.oauth-db-instance[].description", {"m": _APM_SCF})
    assert result.values_per_file["m"] == ["Default OAuth DB."]


def test_apm_access_policy_projects_items_and_start():
    result = run_query(".apm.access-policy[].items", {"m": _APM_SCF})
    [items] = result.values_per_file["m"]
    assert sorted(p.full_path for p in items) == [
        "/Common/policy_default_act_auth",
        "/Common/policy_default_end_allow",
        "/Common/policy_default_end_deny",
        "/Common/policy_default_ent",
    ]


def test_apm_access_policy_start_item_pathref_auto_dereferences():
    """``.start-item`` is a PathRef into ``apm policy policy-item``;
    chaining ``.caption`` should walk into the target policy-item.
    """
    result = run_query(
        ".apm.access-policy[].start-item.caption",
        {"m": _APM_SCF},
    )
    assert result.values_per_file["m"] == ["Start"]


def test_apm_access_policy_items_chain_walks_to_caption():
    """``.items[].caption`` follows the list-of-PathRefs into the
    target policy-items and pulls each caption.
    """
    result = run_query(
        ".apm.access-policy[].items[].caption",
        {"m": _APM_SCF},
    )
    assert sorted(result.values_per_file["m"]) == [
        "Allow",
        "Deny",
        "Kerberos Auth",
        "Start",
    ]


def test_apm_policy_item_projects_caption_and_agents():
    result = run_query(
        '.apm.policy-item["/Common/policy_default_act_auth"].caption',
        {"m": _APM_SCF},
    )
    assert result.values_per_file["m"] == ["Kerberos Auth"]
    result = run_query(
        '.apm.policy-item["/Common/policy_default_act_auth"].agents',
        {"m": _APM_SCF},
    )
    [agents] = result.values_per_file["m"]
    assert [a.full_path for a in agents] == ["/Common/policy_default_act_auth_ag"]


def test_apm_policy_item_agents_pathref_walks_to_agent_type():
    """``.agents[]`` are PathRefs into ``apm policy agent``; chaining
    ``.agent-type`` should pull the agent's classification.
    """
    result = run_query(
        '.apm.policy-item["/Common/policy_default_act_auth"].agents[].agent-type',
        {"m": _APM_SCF},
    )
    assert result.values_per_file["m"] == ["kerberos"]


def test_apm_policy_agent_three_word_kinds_are_recognised():
    """``apm policy agent <type>`` is a THREE-word kind name — the
    header parser must classify all three variants into the merged
    ``apm policy agent`` container with their ``agent-type``
    distinguishing them.
    """
    result = run_query(".apm.policy-agent[].agent-type", {"m": _APM_SCF})
    assert sorted(result.values_per_file["m"]) == [
        "ending-allow",
        "ending-deny",
        "kerberos",
    ]


def test_apm_policy_customization_source_projects_name():
    result = run_query(".apm.customization-source[].name", {"m": _APM_SCF})
    assert sorted(result.values_per_file["m"]) == ["modern", "standard"]


def test_apm_default_report_singleton_projects_report_name_and_user():
    """``apm report default-report`` is a two-word singleton — three
    header tokens with no full-path."""
    result = run_query(".apm.default-report[].report-name", {"m": _APM_SCF})
    assert result.values_per_file["m"] == ["sessionReports/sessionSummary"]
    result = run_query(".apm.default-report[].user", {"m": _APM_SCF})
    assert result.values_per_file["m"] == ["/Common/admin"]


# ---------------------------------------------------------------------------
# cm.* — typed projection for Cluster Manager (trust + device-group)
# ---------------------------------------------------------------------------

_CM_SCF = (
    "cm cert /Common/dtca.crt {\n"
    "    cache-path /config/filestore/files_d/Common_d/trust_certificate_d/dtca.crt_1\n"
    "    checksum SHA1:1289:b1474cd08e35af965335d20b45993636ea86c627\n"
    "    revision 1\n"
    "}\n"
    "cm cert /Common/dtca-bundle.crt {\n"
    "    cache-path /config/filestore/files_d/Common_d/trust_certificate_d/bundle_1\n"
    "    checksum SHA1:1289:b1474cd08e35af965335d20b45993636ea86c628\n"
    "    revision 1\n"
    "}\n"
    "cm cert /Common/dtdi.crt {\n"
    "    cache-path /config/filestore/files_d/Common_d/trust_certificate_d/dtdi.crt_1\n"
    "    checksum SHA1:1220:b134fef5c52870c01f950a488ead241f25c12ff4\n"
    "    revision 1\n"
    "}\n"
    "cm key /Common/dtca.key {\n"
    "    cache-path /config/filestore/files_d/Common_d/trust_certificate_key_d/dtca.key_1\n"
    "    checksum SHA1:1704:87393ebbae46ef2263ca3812e45cb40960289bca\n"
    "    revision 1\n"
    "}\n"
    "cm key /Common/dtdi.key {\n"
    "    cache-path /config/filestore/files_d/Common_d/trust_certificate_key_d/dtdi.key_1\n"
    "    checksum SHA1:1704:58cf2487d09857a3e15f5940f45dccc4660c8925\n"
    "    revision 1\n"
    "}\n"
    "cm device /Common/host1 {\n"
    "    base-mac 00:50:56:2a:20:8d\n"
    "    build 0.0.6\n"
    "    cert /Common/dtdi.crt\n"
    "    edition Final\n"
    "    hostname host1.example.test\n"
    "    key /Common/dtdi.key\n"
    "    management-ip 192.0.2.31\n"
    '    marketing-name "BIG-IP Virtual Edition"\n'
    "    platform-id Z100\n"
    "    product BIG-IP\n"
    "    self-device true\n"
    "    time-zone UTC\n"
    "    version 17.1.1\n"
    "}\n"
    "cm device-group /Common/device_trust_group {\n"
    "    auto-sync enabled\n"
    "    devices {\n"
    "        /Common/host1 { }\n"
    "    }\n"
    "    hidden true\n"
    "    network-failover disabled\n"
    "}\n"
    "cm traffic-group /Common/traffic-group-1 {\n"
    "    unit-id 1\n"
    "}\n"
    "cm traffic-group /Common/traffic-group-local-only { }\n"
    "cm trust-domain /Common/Root {\n"
    "    ca-cert /Common/dtca.crt\n"
    "    ca-cert-bundle /Common/dtca-bundle.crt\n"
    "    ca-devices { /Common/host1 }\n"
    "    ca-key /Common/dtca.key\n"
    "    guid 89af513f-b16f-4c2c-9e740050562a208d\n"
    "    status standalone\n"
    "    trust-group /Common/device_trust_group\n"
    "}\n"
)


def test_cm_cert_projects_checksum_and_revision():
    result = run_query(
        '.cm.cert["/Common/dtca.crt"].checksum',
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == ["SHA1:1289:b1474cd08e35af965335d20b45993636ea86c627"]
    result = run_query(".cm.cert[].revision", {"m": _CM_SCF})
    assert sorted(result.values_per_file["m"]) == ["1", "1", "1"]


def test_cm_key_projects_cache_path():
    result = run_query(
        '.cm.key["/Common/dtca.key"].cache-path',
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == [
        "/config/filestore/files_d/Common_d/trust_certificate_key_d/dtca.key_1"
    ]


def test_cm_device_projects_core_scalars():
    """The bulky ``active-modules`` etc. lists are intentionally not
    projected; the core identity / placement scalars are what we
    surface."""
    result = run_query(".cm.device[].hostname", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["host1.example.test"]
    result = run_query(".cm.device[].management-ip", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["192.0.2.31"]
    result = run_query(".cm.device[].version", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["17.1.1"]
    # marketing-name strips outer quotes during parse.
    result = run_query(".cm.device[].marketing-name", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["BIG-IP Virtual Edition"]


def test_cm_device_cert_pathref_auto_dereferences():
    """``.cert`` is a PathRef into ``cm cert``; chaining ``.checksum``
    walks into the cert object.
    """
    result = run_query(".cm.device[].cert.checksum", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["SHA1:1220:b134fef5c52870c01f950a488ead241f25c12ff4"]


def test_cm_device_group_devices_is_a_list_of_pathrefs():
    """``.devices[]`` walks into ``cm device`` so chaining ``.hostname``
    pulls the device's hostname.
    """
    result = run_query(
        ".cm.device-group[].devices[].hostname",
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == ["host1.example.test"]


def test_cm_traffic_group_projects_unit_id():
    result = run_query(
        '.cm.traffic-group["/Common/traffic-group-1"].unit-id',
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == ["1"]


def test_cm_trust_domain_projects_ca_cert_and_ca_devices():
    result = run_query(".cm.trust-domain[].ca-cert.revision", {"m": _CM_SCF})
    assert result.values_per_file["m"] == ["1"]
    result = run_query(
        ".cm.trust-domain[].ca-devices[].hostname",
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == ["host1.example.test"]


def test_cm_trust_domain_trust_group_chain_walks_to_devices():
    """Two-hop PathRef chain: ``trust-domain → device-group → devices[]``."""
    result = run_query(
        ".cm.trust-domain[].trust-group.devices[].hostname",
        {"m": _CM_SCF},
    )
    assert result.values_per_file["m"] == ["host1.example.test"]


# ---------------------------------------------------------------------------
# gtm.* — typed projection for Global Traffic Manager
# ---------------------------------------------------------------------------

_GTM_SCF = (
    "gtm datacenter /Common/dc_east {\n"
    '    contact "noc@example.test"\n'
    "    location east\n"
    "}\n"
    "gtm datacenter /Common/dc_west {\n"
    "    location west\n"
    "}\n"
    "gtm server /Common/srv_east {\n"
    "    datacenter /Common/dc_east\n"
    "    devices {\n"
    "        0 {\n"
    "            addresses {\n"
    "                192.0.2.7 { }\n"
    "            }\n"
    "        }\n"
    "    }\n"
    "    monitor /Common/bigip\n"
    "    product bigip\n"
    "    virtual-servers {\n"
    "        0 {\n"
    "            destination 192.0.2.8:5050\n"
    "        }\n"
    "        1 {\n"
    "            destination 192.0.2.9:80\n"
    "        }\n"
    "    }\n"
    "}\n"
    "gtm server /Common/srv_west {\n"
    "    datacenter /Common/dc_west\n"
    "    devices {\n"
    "        0 {\n"
    "            addresses {\n"
    "                198.51.100.7 { }\n"
    "            }\n"
    "        }\n"
    "    }\n"
    "}\n"
    "gtm pool a /AS3/app/p_a1 {\n"
    "    alternate-mode ratio\n"
    "    load-balancing-mode global-availability\n"
    "    members {\n"
    "        /Common/srv_east:0 { }\n"
    "    }\n"
    "    ttl 180\n"
    "}\n"
    "gtm pool a /AS3/app/p_a2 {\n"
    "    load-balancing-mode round-robin\n"
    "    ttl 60\n"
    "}\n"
    "gtm pool mx /AS3/app/p_mx1 {\n"
    "    load-balancing-mode global-availability\n"
    "    ttl 300\n"
    "}\n"
    "gtm wideip a /AS3/app/example.test {\n"
    "    pool-lb-mode global-availability\n"
    "    pools {\n"
    "        /AS3/app/p_a1 {\n"
    "            order 0\n"
    "        }\n"
    "        /AS3/app/p_a2 {\n"
    "            order 1\n"
    "        }\n"
    "    }\n"
    "}\n"
    "gtm wideip mx /AS3/app/mx.example.test {\n"
    "    aliases {\n"
    "        \\?.mx.example.test\n"
    "    }\n"
    "    last-resort-pool mx /AS3/app/p_mx1\n"
    "    pool-lb-mode ratio\n"
    "}\n"
    "gtm prober-pool /Common/pp_default {\n"
    '    description "Default prober pool"\n'
    "    load-balancing-mode round-robin\n"
    "    members {\n"
    "        /Common/srv_east { }\n"
    "        /Common/srv_west { }\n"
    "    }\n"
    "}\n"
    "gtm region /Common/region_sa {\n"
    '    description "South America"\n'
    "    region-members {\n"
    "        continent SA { }\n"
    "        subnet 192.0.2.0/24 { }\n"
    "    }\n"
    "}\n"
    "gtm rule /AS3/app/gtm_rule_log {\n"
    "    when DNS_REQUEST {\n"
    '        log local2. "DNS query received"\n'
    "    }\n"
    "}\n"
)


def test_gtm_datacenter_projects_contact_and_location():
    """Quoted contact value has its outer quotes stripped during parse."""
    result = run_query('.gtm.datacenter["/Common/dc_east"].contact', {"m": _GTM_SCF})
    assert result.values_per_file["m"] == ["noc@example.test"]
    result = run_query(".gtm.datacenter[].location", {"m": _GTM_SCF})
    assert sorted(result.values_per_file["m"]) == ["east", "west"]


def test_gtm_server_flattens_addresses_across_devices():
    """``devices { 0 { addresses { ... } } }`` is flattened into a
    single addresses tuple on the server object.
    """
    result = run_query('.gtm.server["/Common/srv_east"].addresses', {"m": _GTM_SCF})
    [addrs] = result.values_per_file["m"]
    assert list(addrs) == ["192.0.2.7"]


def test_gtm_server_surfaces_virtual_server_destinations():
    """``virtual-servers { 0 { destination ... } 1 { ... } }`` becomes
    a list of destination strings.
    """
    result = run_query('.gtm.server["/Common/srv_east"].virtual-servers', {"m": _GTM_SCF})
    [dests] = result.values_per_file["m"]
    assert list(dests) == ["192.0.2.8:5050", "192.0.2.9:80"]


def test_gtm_server_datacenter_pathref_auto_dereferences():
    """``server.datacenter`` is a PathRef into ``gtm datacenter``."""
    result = run_query(
        '.gtm.server["/Common/srv_east"].datacenter.location',
        {"m": _GTM_SCF},
    )
    assert result.values_per_file["m"] == ["east"]


def test_gtm_pool_record_type_distinguishes_dns_kinds():
    """``gtm pool a|aaaa|cname|mx|...`` is merged into one container;
    ``record-type`` distinguishes them.
    """
    result = run_query(".gtm.pool[].record-type", {"m": _GTM_SCF})
    assert sorted(result.values_per_file["m"]) == ["a", "a", "mx"]


def test_gtm_pool_projects_load_balancing_and_ttl():
    result = run_query(
        '.gtm.pool["/AS3/app/p_a1"].load-balancing-mode',
        {"m": _GTM_SCF},
    )
    assert result.values_per_file["m"] == ["global-availability"]
    result = run_query('.gtm.pool["/AS3/app/p_a1"].ttl', {"m": _GTM_SCF})
    assert result.values_per_file["m"] == ["180"]


def test_gtm_wideip_pools_are_pathrefs_into_pool():
    """``wideip.pools[]`` walks into ``gtm pool``; chaining ``.ttl``
    pulls the TTL of each referenced pool.
    """
    result = run_query(
        '.gtm.wideip["/AS3/app/example.test"].pools[].ttl',
        {"m": _GTM_SCF},
    )
    assert sorted(result.values_per_file["m"]) == ["180", "60"]


def test_gtm_wideip_last_resort_pool_pathref_strips_record_type():
    """``last-resort-pool mx /AS3/app/p_mx1`` carries the record-type
    prefix in the source; the parser strips it so the field holds a
    clean PathRef.
    """
    result = run_query(
        '.gtm.wideip["/AS3/app/mx.example.test"].last-resort-pool.ttl',
        {"m": _GTM_SCF},
    )
    assert result.values_per_file["m"] == ["300"]


def test_gtm_prober_pool_members_walk_to_server_addresses():
    """``prober-pool.members[]`` → ``gtm server`` PathRef chain — two
    server addresses on either side of the prober pool.
    """
    result = run_query(
        ".gtm.prober-pool[].members[].addresses",
        {"m": _GTM_SCF},
    )
    flat = [a for sub in result.values_per_file["m"] for a in sub]
    assert sorted(flat) == ["192.0.2.7", "198.51.100.7"]


def test_gtm_region_projects_region_members():
    result = run_query('.gtm.region["/Common/region_sa"].region-members', {"m": _GTM_SCF})
    [members] = result.values_per_file["m"]
    # ``region-members`` are token sequences, not full-paths.
    assert sorted(members) == ["continent SA", "subnet 192.0.2.0/24"]


def test_gtm_rule_projects_body_with_dns_request_handler():
    result = run_query('.gtm.rule["/AS3/app/gtm_rule_log"].body', {"m": _GTM_SCF})
    [body] = result.values_per_file["m"]
    assert "DNS_REQUEST" in body
    assert "DNS query received" in body


# Issue 2 — Parser must not hang on ``\"...\"`` data-group record keys.
_DG_ESCAPED_KEYS = (
    "ltm data-group internal /Common/dg_minimal {\n"
    "    type string\n"
    "    records {\n"
    '        \\"/owa\\" {\n'
    '            data \\"x\\"\n'
    "        }\n"
    "    }\n"
    "}\n"
)


def test_parser_finishes_on_escaped_quote_keys():
    """Regression for the v1.9.0-14 zipapp hang on ``tmsh``-emitted
    data-group records whose keys use the ``\\"...\\"`` escape form.
    The bug confused the property-extractor (the ``\\\\"`` started a
    runaway string scan that swallowed braces), which leaked a stray
    ``}`` into the ``records`` list-block, and ``_parse_list_block``
    sat in an infinite loop because it didn't advance over the
    unmatched brace.  The fix: treat ``\\\\<x>`` as opaque outside
    strings, and add a no-progress guard.

    A reasonable upper-bound ensures the test surfaces a regression
    rather than wedging the suite.
    """
    import threading

    finished = threading.Event()
    error_box: list[BaseException] = []

    def _run():
        try:
            run_query(".ltm.data-group[]", {"m": _DG_ESCAPED_KEYS})
        except BaseException as exc:  # noqa: BLE001
            error_box.append(exc)
        finally:
            finished.set()

    t = threading.Thread(target=_run, daemon=True)
    t.start()
    t.join(timeout=5.0)
    assert finished.is_set(), "parser hung on escaped-quote data-group keys"
    assert not error_box, error_box[0]


# Issue 4 — ``+=`` materialises a missing compound block.
_NO_RULES_SCF = (
    "ltm virtual /Common/no_rules_yet {\n"
    "    destination /Common/198.51.100.5:443\n"
    "    ip-protocol tcp\n"
    "    mask 255.255.255.255\n"
    "}\n"
)

_MIXED_RULES_SCF = (
    "ltm virtual /Common/has_rules {\n"
    "    destination /Common/198.51.100.5:443\n"
    "    rules { /Common/audit_rule }\n"
    "}\n"
    "ltm virtual /Common/no_rules {\n"
    "    destination /Common/198.51.100.6:80\n"
    "}\n"
)


def test_plus_eq_on_missing_rules_block_materialises_it():
    """``.rules += "..."`` against a VS with no ``rules { … }`` block
    now creates one — matching cookbook example #7.  Regression for
    the v1.9.0-14 zipapp behaviour where this raised "compound values
    are not writable in v1".
    """
    result = run_query('.ltm.virtual[] | .rules += "/Common/log_rule"', {"m": _NO_RULES_SCF})
    new_src = result.edits_per_file["m"].new_source
    assert "rules { /Common/log_rule }" in new_src


def test_plus_eq_extends_existing_rules_block():
    """When the rules block already exists, ``+=`` extends it rather
    than appending a duplicate."""
    result = run_query('.ltm.virtual[] | .rules += "/Common/log_rule"', {"m": _MIXED_RULES_SCF})
    new_src = result.edits_per_file["m"].new_source
    # The has_rules VS sees ``audit_rule`` and ``log_rule`` in one block.
    assert "rules { /Common/audit_rule /Common/log_rule }" in new_src
    # And no_rules gets a freshly materialised block.
    assert "rules { /Common/log_rule }" in new_src
    # No duplicated blocks anywhere — every "    rules {" property
    # line (indented, not a stanza header) appears exactly once per VS.
    assert new_src.count("    rules {") == 2


def test_cookbook_idempotent_attach_idiom_works():
    """Cookbook example #7 — the canonical "attach this iRule to every
    VS that doesn't already have it" idiom now works end-to-end."""
    result = run_query(
        ".ltm.virtual[] "
        '| select(not contains(.rules, "/Common/log_rule")) '
        '| .rules += "/Common/log_rule"',
        {"m": _MIXED_RULES_SCF},
    )
    new_src = result.edits_per_file["m"].new_source
    assert new_src.count("/Common/log_rule") == 2
    # Running it again is a no-op (idempotent).
    again = run_query(
        ".ltm.virtual[] "
        '| select(not contains(.rules, "/Common/log_rule")) '
        '| .rules += "/Common/log_rule"',
        {"m": new_src},
    )
    applied = again.edits_per_file.get("m")
    assert applied is None or applied.new_source == applied.original


def test_pool_members_writes_are_rejected_not_corrupted():
    """``.ltm.pool[].members`` is sub-block-shaped — each element is an
    object with ``address`` / ``port`` / ``monitor`` fields, not a
    flat token.  The compound-block materialiser would stringify the
    member objects into a meaningless token sequence and produce
    invalid SCF; pool-member edits stay out of scope for v1.  The
    edit pipeline must reject the write with a clear error rather
    than corrupt the config silently.
    """
    from core.bigip.query.errors import EditError

    src = (
        "ltm pool /Common/web_pool {\n"
        "    members { /Common/n1:80 { address 192.0.2.10 } }\n"
        "    monitor /Common/http\n"
        "}\n"
    )
    with pytest.raises(EditError):
        run_query(".ltm.pool[].members = []", {"m": src})


# Issue 5 — ``--json`` of scalar / path-ref projections.
_JSON_SCALAR_SCF = (
    "ltm pool /Common/web_pool {\n"
    "    members { /Common/n1:80 { address 192.0.2.10 } }\n"
    "    monitor /Common/http\n"
    "}\n"
    "ltm pool /Common/api_pool {\n"
    "    members { /Common/n2:80 { address 192.0.2.11 } }\n"
    "    monitor /Common/http\n"
    "}\n"
)


def test_json_emits_scalar_projection_as_array():
    """``--json`` of a scalar projection must surface every value, not
    drop them to ``[]``.  Regression for the v1.9.0-14 zipapp bug
    where the JSON serialiser filtered on object/PathRef kinds and
    dropped plain strings.
    """
    result = run_query(".ltm.pool[].name", {"m": _JSON_SCALAR_SCF})
    rendered = render(result.values_per_file["m"], mode="json")
    assert json.loads(rendered) == ["web_pool", "api_pool"]


def test_json_emits_pathref_projection_as_array():
    """PathRefs serialise to their ``full_path`` strings in JSON output."""
    result = run_query(".ltm.pool[].monitor", {"m": _JSON_SCALAR_SCF})
    rendered = render(result.values_per_file["m"], mode="json")
    assert json.loads(rendered) == ["/Common/http", "/Common/http"]


def test_json_emits_integer_aggregate_as_array():
    """``[.X[]] | length`` produces one integer; JSON wraps it as ``[N]``."""
    result = run_query("[.ltm.pool[]] | length", {"m": _JSON_SCALAR_SCF})
    rendered = render(result.values_per_file["m"], mode="json")
    assert json.loads(rendered) == [2]


def test_generated_builtins_doc_is_up_to_date():
    """The generated reference must match the registry on disk.

    Run ``python scripts/dev/gen_query_builtins_doc.py`` after changing
    any builtin's spec.  The generator is deterministic (no timestamps),
    so a diff here means the on-disk doc has drifted from the registry.
    """
    import sys
    from pathlib import Path

    repo_root = Path(__file__).resolve().parent.parent
    sys.path.insert(0, str(repo_root / "scripts" / "dev"))
    import gen_query_builtins_doc  # type: ignore[import-not-found]

    expected = gen_query_builtins_doc.render(list_builtins())
    on_disk = (repo_root / "docs" / "design" / "f5-query-dsl-builtins.md").read_text(
        encoding="utf-8"
    )
    assert on_disk == expected, (
        "docs/design/f5-query-dsl-builtins.md is out of date; run "
        "`python scripts/dev/gen_query_builtins_doc.py` to regenerate."
    )


def test_cli_help_examples_includes_every_cookbook_entry(capsys):
    with pytest.raises(SystemExit):
        main(["query", "--help-examples"])
    out = capsys.readouterr().out
    for example in list_examples():
        assert example.title in out


# ---------------------------------------------------------------------------
# Field enrichment — surfacing parsed-but-previously-unexposed attributes
# ---------------------------------------------------------------------------

_ENRICH_LTM_CONF = """ltm pool /Common/web_pool {
    description "primary web pool"
    members {
        /Common/n1:80 {
            address 10.0.0.1
            description "node 1"
            disabled
            ratio 3
            priority-group 5
            monitor /Common/http
        }
    }
    monitor /Common/http
    min-active-members 2
    service-down-action reset
    slow-ramp-time 30
    allow-snat yes
    allow-nat no
    reselect-tries 4
}
ltm node /Common/n1 {
    address 10.0.0.1
    description "first node"
    monitor /Common/http
    disabled
    connection-limit 1000
    ratio 1
}
ltm monitor http /Common/http_base { }
ltm monitor http /Common/http {
    defaults-from /Common/http_base
    description "http GET probe"
    interval 5
    timeout 16
    destination "*:80"
    send "GET / HTTP/1.0\\r\\n\\r\\n"
    recv "200 OK"
}
ltm persistence cookie /Common/cookie_pers {
    defaults-from /Common/cookie
    description "session cookie"
    timeout 1800
}
ltm profile http /Common/parent_http { }
ltm profile http /Common/http_profile {
    defaults-from /Common/parent_http
    description "tuned http"
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/web_pool
    description "front-door VIP"
    mask 255.255.255.255
    source 0.0.0.0/0
    ip-protocol tcp
    connection-limit 10000
    rate-limit 500
    translate-address enabled
    translate-port enabled
    disabled
    vlans { /Common/external /Common/internal }
    vlans-disabled
    fallback-persistence /Common/cookie_pers
    last-hop-pool /Common/web_pool
    fw-enforced-policy /Common/fw_pol
    auth { /Common/radius_auth }
}
ltm policy /Common/pol1 {
    strategy /Common/first-match
    description "main policy"
    status published
    last-modified "2024-01-02T03:04:05Z"
    rules { }
}
ltm data-group internal /Common/dg1 {
    type string
    description "lookup table"
    records {
        red { data crimson }
    }
}
net vlan /Common/external {
    tag 10
}
net vlan /Common/internal {
    tag 20
}
ltm persistence cookie /Common/cookie { }
ltm pool /Common/fw_pol { }
"""


def test_pool_surfaces_description_and_governance():
    result = _run(".ltm.pool.web_pool.description", _ENRICH_LTM_CONF)
    [desc] = result.values_per_file["mem://1"]
    assert desc == "primary web pool"


def test_pool_surfaces_service_down_action_and_slow_ramp():
    result = _run(".ltm.pool.web_pool.service-down-action", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["reset"]
    result = _run(".ltm.pool.web_pool.slow-ramp-time", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["30"]


def test_pool_member_state_and_ratio_are_projected():
    result = _run(".ltm.pool.web_pool.members[0].state", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["disabled"]
    result = _run(".ltm.pool.web_pool.members[0].ratio", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["3"]


def test_pool_member_monitor_chains_through_pathref():
    result = _run(".ltm.pool.web_pool.members[0].monitor.full-path", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/http"]


def test_node_state_and_monitor_are_projected():
    result = _run(".ltm.node.n1.state", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["disabled"]
    result = _run(".ltm.node.n1.monitor.full-path", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/http"]


def test_monitor_defaults_from_chains_through_pathref():
    result = _run(".ltm.monitor.http.defaults-from.full-path", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/http_base"]


def test_monitor_send_and_recv_are_unquoted():
    result = _run(".ltm.monitor.http.send", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"][0].startswith("GET /")
    result = _run(".ltm.monitor.http.recv", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["200 OK"]


def test_persistence_defaults_from_chain():
    result = _run(".ltm.persistence.cookie_pers.defaults-from.full-path", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/cookie"]


def test_profile_defaults_from_is_a_path_ref():
    result = _run(".ltm.profile.http_profile.defaults-from.full-path", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/parent_http"]


def test_virtual_state_description_and_protocol():
    desc = _run(".ltm.virtual.web_vs.description", _ENRICH_LTM_CONF)
    assert desc.values_per_file["mem://1"] == ["front-door VIP"]
    state = _run(".ltm.virtual.web_vs.state", _ENRICH_LTM_CONF)
    assert state.values_per_file["mem://1"] == ["disabled"]
    proto = _run(".ltm.virtual.web_vs.ip-protocol", _ENRICH_LTM_CONF)
    assert proto.values_per_file["mem://1"] == ["tcp"]


def test_virtual_vlans_resolve_through_path_ref():
    result = _run(".ltm.virtual.web_vs.vlans[].tag", _ENRICH_LTM_CONF)
    assert sorted(result.values_per_file["mem://1"]) == [10, 20]


def test_virtual_vlans_disabled_flag_is_visible():
    result = _run(".ltm.virtual.web_vs.vlans-disabled", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == [True]


def test_virtual_fallback_persistence_walks_to_persistence_type():
    result = _run(
        ".ltm.virtual.web_vs.fallback-persistence.type",
        _ENRICH_LTM_CONF,
    )
    assert result.values_per_file["mem://1"] == ["cookie"]


def test_policy_description_status_and_last_modified():
    desc = _run(".ltm.policy.pol1.description", _ENRICH_LTM_CONF)
    assert desc.values_per_file["mem://1"] == ["main policy"]
    status = _run(".ltm.policy.pol1.status", _ENRICH_LTM_CONF)
    assert status.values_per_file["mem://1"] == ["published"]
    mod = _run(".ltm.policy.pol1.last-modified", _ENRICH_LTM_CONF)
    assert mod.values_per_file["mem://1"] == ["2024-01-02T03:04:05Z"]


def test_data_group_description_is_projected():
    result = _run(".ltm.data-group.dg1.description", _ENRICH_LTM_CONF)
    assert result.values_per_file["mem://1"] == ["lookup table"]


_ENRICH_GTM_CONF = """gtm datacenter /Common/dc-east {
    contact "Pat Ops"
    location "Boston"
    description "east coast"
    prober-pool /Common/east-probers
    enabled
}
gtm server /Common/gtm-a {
    datacenter /Common/dc-east
    monitor /Common/bigip
    product bigip
    description "primary BIG-IP"
    enabled
    prober-pool /Common/east-probers
    prober-preference inherit
    limit-max-bps 1000000
    devices { 0 { addresses { 10.1.1.1 { } } } }
    virtual-servers { 0 { destination 10.1.1.2:80 } }
}
gtm pool a /Common/pool-a {
    description "A-record pool"
    enabled
    load-balancing-mode global-availability
    ttl 60
    verify-member-availability enabled
    qos-rtt 50
    qos-hops 4
}
gtm wideip a /Common/www.example.com {
    description "production"
    enabled
    pools { /Common/pool-a }
    pool-lb-mode topology
    failure-rcode noerror
    minimal-response disabled
}
gtm prober-pool /Common/east-probers {
    description "east DC probers"
    load-balancing-mode round-robin
    members { /Common/gtm-a { } }
    enabled
}
"""


def test_gtm_datacenter_state_and_prober_pool():
    state = _run(".gtm.datacenter.dc-east.state", _ENRICH_GTM_CONF)
    assert state.values_per_file["mem://1"] == ["enabled"]
    prober_path = _run(".gtm.datacenter.dc-east.prober-pool.full-path", _ENRICH_GTM_CONF)
    assert prober_path.values_per_file["mem://1"] == ["/Common/east-probers"]


def test_gtm_server_state_and_limits():
    state = _run(".gtm.server.gtm-a.state", _ENRICH_GTM_CONF)
    assert state.values_per_file["mem://1"] == ["enabled"]
    bps = _run(".gtm.server.gtm-a.limit-max-bps", _ENRICH_GTM_CONF)
    assert bps.values_per_file["mem://1"] == ["1000000"]


def test_gtm_pool_state_and_qos():
    state = _run(".gtm.pool.pool-a.state", _ENRICH_GTM_CONF)
    assert state.values_per_file["mem://1"] == ["enabled"]
    rtt = _run(".gtm.pool.pool-a.qos-rtt", _ENRICH_GTM_CONF)
    assert rtt.values_per_file["mem://1"] == ["50"]


def test_gtm_wideip_failure_rcode():
    rcode = _run('.gtm.wideip["/Common/www.example.com"].failure-rcode', _ENRICH_GTM_CONF)
    assert rcode.values_per_file["mem://1"] == ["noerror"]


def test_gtm_prober_pool_state_and_description():
    state = _run(".gtm.prober-pool.east-probers.state", _ENRICH_GTM_CONF)
    assert state.values_per_file["mem://1"] == ["enabled"]
    desc = _run(".gtm.prober-pool.east-probers.description", _ENRICH_GTM_CONF)
    assert desc.values_per_file["mem://1"] == ["east DC probers"]


_ENRICH_NET_CONF = """net vlan /Common/v10 {
    tag 10
    description "client vlan"
    mtu 1500
    cmp-hash src-ip
    failsafe disabled
}
net self /Common/self-v10 {
    address 10.0.10.1/24
    vlan /Common/v10
    floating enabled
    unit 1
    description "self IP on v10"
}
net route /Common/default-route {
    network default
    gw 10.0.10.254
    description "default gateway"
    mtu 9000
}
net route-domain /Common/rd1 {
    id 1
    vlans { /Common/v10 }
    description "tenant rd"
    parent /Common/rd0
    strict enabled
    fw-enforced-policy /Common/fw_pol
}
net route-domain /Common/rd0 {
    id 0
}
net port-list /Common/web_ports {
    ports { 80 443 }
    description "web ports"
}
net dns-resolver /Common/dnsr1 {
    description "tenant resolver"
    cache-size 1048576
    use-tcp yes
    use-udp yes
}
net stp /Common/cist {
    interfaces { 1.1 1.2 }
    description "spanning tree"
    mode rstp
}
"""


def test_net_vlan_description_and_mtu():
    desc = _run(".net.vlan.v10.description", _ENRICH_NET_CONF)
    assert desc.values_per_file["mem://1"] == ["client vlan"]
    mtu = _run(".net.vlan.v10.mtu", _ENRICH_NET_CONF)
    assert mtu.values_per_file["mem://1"] == ["1500"]


def test_net_self_floating_and_unit():
    floating = _run(".net.self.self-v10.floating", _ENRICH_NET_CONF)
    assert floating.values_per_file["mem://1"] == ["enabled"]


def test_net_route_mtu_and_description():
    mtu = _run(".net.route.default-route.mtu", _ENRICH_NET_CONF)
    assert mtu.values_per_file["mem://1"] == ["9000"]


def test_net_route_domain_parent_walks_to_id():
    result = _run(".net.route-domain.rd1.parent.id", _ENRICH_NET_CONF)
    assert result.values_per_file["mem://1"] == [0]


def test_net_port_list_description():
    desc = _run(".net.port-list.web_ports.description", _ENRICH_NET_CONF)
    assert desc.values_per_file["mem://1"] == ["web ports"]


def test_net_dns_resolver_cache_size_and_protocol_flags():
    cache = _run(".net.dns-resolver.dnsr1.cache-size", _ENRICH_NET_CONF)
    assert cache.values_per_file["mem://1"] == ["1048576"]


_ENRICH_CM_CONF = """cm device /Common/bigip-1 {
    hostname bigip-1.example.com
    management-ip 10.0.0.10
    base-mac 00:11:22:33:44:55
    description "primary unit"
    contact "Pat Ops"
    location "Boston"
    mirror-ip 10.0.99.1
    multicast-ip 224.0.0.245
    unicast-address {
        { ip 10.0.99.1 }
        { ip 10.0.99.2 }
    }
}
cm device-group /Common/sync-failover {
    type sync-failover
    auto-sync enabled
    description "two-unit cluster"
    save-on-auto-sync true
    devices { /Common/bigip-1 }
}
cm traffic-group /Common/traffic-group-1 {
    unit-id 1
    description "default TG"
    default-device /Common/bigip-1
    ha-load-factor 1
    ha-order { /Common/bigip-1 }
    auto-failback-enabled false
}
"""


def test_cm_device_description_and_contact():
    desc = _run(".cm.device.bigip-1.description", _ENRICH_CM_CONF)
    assert desc.values_per_file["mem://1"] == ["primary unit"]
    contact = _run(".cm.device.bigip-1.contact", _ENRICH_CM_CONF)
    assert contact.values_per_file["mem://1"] == ["Pat Ops"]


def test_cm_device_unicast_addresses_are_flattened():
    addrs = _run(".cm.device.bigip-1.unicast-address", _ENRICH_CM_CONF)
    [pair] = addrs.values_per_file["mem://1"]
    assert sorted(pair) == ["10.0.99.1", "10.0.99.2"]


def test_cm_device_group_type_and_save_on_auto_sync():
    typ = _run(".cm.device-group.sync-failover.type", _ENRICH_CM_CONF)
    assert typ.values_per_file["mem://1"] == ["sync-failover"]
    save = _run(".cm.device-group.sync-failover.save-on-auto-sync", _ENRICH_CM_CONF)
    assert save.values_per_file["mem://1"] == ["true"]


def test_cm_traffic_group_default_device_walks_to_hostname():
    host = _run(
        ".cm.traffic-group.traffic-group-1.default-device.hostname",
        _ENRICH_CM_CONF,
    )
    assert host.values_per_file["mem://1"] == ["bigip-1.example.com"]


def test_cm_traffic_group_ha_order_resolves_to_devices():
    result = _run(".cm.traffic-group.traffic-group-1.ha-order[].full-path", _ENRICH_CM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/bigip-1"]


_ENRICH_SYS_CONF = """sys folder /Common {
    device-group /Common/sync-failover
    description "shared partition"
}
sys file ssl-cert /Common/example.crt {
    source-path "file:///config/ssl/ssl.crt/example.crt"
    revision 1
    description "example cert"
    issuer "CN=Example CA"
    subject "CN=www.example.com"
    key-size 2048
}
sys file ssl-key /Common/example.key {
    source-path "file:///config/ssl/ssl.key/example.key"
    revision 1
    description "example key"
    key-size 2048
    key-type rsa-private
}
sys provision /Common/ltm {
    level nominal
    cpu-ratio 5
    memory-ratio 50
}
cm device-group /Common/sync-failover { }
"""


def test_sys_folder_description():
    desc = _run(".sys.folder.Common.description", _ENRICH_SYS_CONF)
    assert desc.values_per_file["mem://1"] == ["shared partition"]


def test_sys_file_ssl_cert_issuer_subject_and_key_size():
    issuer = _run('.sys.file-ssl-cert["/Common/example.crt"].issuer', _ENRICH_SYS_CONF)
    assert issuer.values_per_file["mem://1"] == ["CN=Example CA"]
    key_size = _run('.sys.file-ssl-cert["/Common/example.crt"].key-size', _ENRICH_SYS_CONF)
    assert key_size.values_per_file["mem://1"] == ["2048"]


def test_sys_file_ssl_key_key_type_is_projected():
    typ = _run('.sys.file-ssl-key["/Common/example.key"].key-type', _ENRICH_SYS_CONF)
    assert typ.values_per_file["mem://1"] == ["rsa-private"]


def test_sys_provision_cpu_and_memory_ratio():
    cpu = _run(".sys.provision.ltm.cpu-ratio", _ENRICH_SYS_CONF)
    assert cpu.values_per_file["mem://1"] == ["5"]
    mem = _run(".sys.provision.ltm.memory-ratio", _ENRICH_SYS_CONF)
    assert mem.values_per_file["mem://1"] == ["50"]


_ENRICH_SEC_CONF = """security firewall port-list /Common/web-ports {
    ports { 80 443 }
    description "web traffic"
}
security firewall rule-list /Common/rl1 {
    rules { }
    description "edge rules"
}
security ip-intelligence policy /Common/ip-int {
    description "block scanners"
    default-action drop
}
"""


def test_security_firewall_port_list_description():
    desc = _run(".security.firewall-port-list.web-ports.description", _ENRICH_SEC_CONF)
    assert desc.values_per_file["mem://1"] == ["web traffic"]


def test_security_ip_intelligence_default_action():
    action = _run(".security.ip-intelligence-policy.ip-int.default-action", _ENRICH_SEC_CONF)
    assert action.values_per_file["mem://1"] == ["drop"]


_ENRICH_APM_CONF = """apm policy access-policy /Common/ap1 {
    start-item /Common/ap1-start
    default-ending /Common/ap1-deny
    items { /Common/ap1-start /Common/ap1-deny }
    description "main policy"
}
apm policy policy-item /Common/ap1-start {
    caption Start
    item-type action
    description "entry"
}
apm policy policy-item /Common/ap1-deny {
    caption Deny
    item-type ending
}
apm policy customization-source /Common/cust1 {
    description "branded"
}
"""


def test_apm_access_policy_description():
    desc = _run(".apm.access-policy.ap1.description", _ENRICH_APM_CONF)
    assert desc.values_per_file["mem://1"] == ["main policy"]


def test_apm_policy_item_description():
    desc = _run(".apm.policy-item.ap1-start.description", _ENRICH_APM_CONF)
    assert desc.values_per_file["mem://1"] == ["entry"]


def test_apm_customization_source_description():
    desc = _run(".apm.customization-source.cust1.description", _ENRICH_APM_CONF)
    assert desc.values_per_file["mem://1"] == ["branded"]


# ---------------------------------------------------------------------------
# pem.* — Policy Enforcement Manager projection
# ---------------------------------------------------------------------------

_PEM_CONF = """ltm virtual /Common/sub_vs {
    destination /Common/10.1.1.1:80
}
ltm pool /Common/fwd_pool { }
ltm pool /Common/icap_pool { }
ltm snatpool /Common/sp1 {
    members { 10.0.0.99 }
}
pem profile spm /Common/spm_parent { }
pem profile spm /Common/spm1 {
    defaults-from /Common/spm_parent
    description "subscriber policy mgmt"
}
pem profile subscriber-mgmt /Common/submgmt1 {
    description "subscriber mgmt"
}
pem profile radius-aaa /Common/radius1 {
    description "radius AAA"
}
pem profile diameter-endpoint /Common/diam1 {
    description "diameter endpoint"
}
pem policy /Common/pol_main {
    description "main PEM policy"
    rules {
        block_streaming { }
        log_session { }
    }
}
pem irule /Common/pem_log {
    when PEM_POLICY {
        log local0. "policy hit"
    }
}
pem listener /Common/listener1 {
    description "front listener"
    profile-spm /Common/spm1
    profile-subscriber-mgmt /Common/submgmt1
    virtual-servers { /Common/sub_vs }
}
pem forwarding-endpoint /Common/fwd1 {
    description "forward to CDN"
    pool /Common/fwd_pool
    snat-pool /Common/sp1
    source-ip 10.0.0.1
    type endpoint
    persistence src-ip
    translate-address enabled
    translate-service enabled
}
pem interception-endpoint /Common/icap1 {
    description "ICAP tap"
    pool /Common/icap_pool
    persistence src-ip
}
pem service-chain-endpoint /Common/chain1 {
    description "two-step chain"
    service-endpoints {
        step_a { order 1 }
        step_b { order 2 }
    }
    steering-policy /Common/pol_main
}
pem quota-mgmt rating-group /Common/rg1 {
    description "streaming quota"
    rating-group-id 100
    default-quota 1000
    default-quota-holding-time 3600
    default-validity-time 86400
    total-octets 5000000
    time 60
}
"""


def test_pem_policy_description_and_rules():
    desc = _run(".pem.policy.pol_main.description", _PEM_CONF)
    assert desc.values_per_file["mem://1"] == ["main PEM policy"]
    rules = _run(".pem.policy.pol_main.rules", _PEM_CONF)
    [rule_list] = rules.values_per_file["mem://1"]
    assert sorted(rule_list) == ["block_streaming", "log_session"]


def test_pem_irule_body_carries_pem_event():
    body = _run(".pem.irule.pem_log.body", _PEM_CONF)
    [src] = body.values_per_file["mem://1"]
    assert "PEM_POLICY" in src
    assert "policy hit" in src


def test_pem_listener_profile_spm_chains_to_defaults_from():
    result = _run(".pem.listener.listener1.profile-spm.defaults-from.full-path", _PEM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/spm_parent"]


def test_pem_listener_virtual_servers_resolve_to_destinations():
    result = _run(".pem.listener.listener1.virtual-servers[].destination", _PEM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/10.1.1.1:80"]


def test_pem_forwarding_endpoint_pool_pathref_resolves():
    result = _run(".pem.forwarding-endpoint.fwd1.pool.full-path", _PEM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/fwd_pool"]


def test_pem_forwarding_endpoint_snat_pool_walks_to_members():
    result = _run(".pem.forwarding-endpoint.fwd1.snat-pool.members", _PEM_CONF)
    [members] = result.values_per_file["mem://1"]
    assert members == ["10.0.0.99"]


def test_pem_forwarding_endpoint_scalar_fields():
    typ = _run(".pem.forwarding-endpoint.fwd1.type", _PEM_CONF)
    assert typ.values_per_file["mem://1"] == ["endpoint"]
    src = _run(".pem.forwarding-endpoint.fwd1.source-ip", _PEM_CONF)
    assert src.values_per_file["mem://1"] == ["10.0.0.1"]


def test_pem_interception_endpoint_pool_resolves():
    result = _run(".pem.interception-endpoint.icap1.pool.full-path", _PEM_CONF)
    assert result.values_per_file["mem://1"] == ["/Common/icap_pool"]


def test_pem_service_chain_endpoint_lists_steps():
    result = _run(".pem.service-chain-endpoint.chain1.service-endpoints", _PEM_CONF)
    [steps] = result.values_per_file["mem://1"]
    assert sorted(steps) == ["step_a", "step_b"]


def test_pem_service_chain_steering_policy_walks_to_rules():
    result = _run(".pem.service-chain-endpoint.chain1.steering-policy.rules", _PEM_CONF)
    [rule_list] = result.values_per_file["mem://1"]
    assert sorted(rule_list) == ["block_streaming", "log_session"]


def test_pem_profile_type_distinguishes_sub_kinds():
    result = _run(".pem.profile[] | .type", _PEM_CONF)
    assert sorted(result.values_per_file["mem://1"]) == [
        "diameter-endpoint",
        "radius-aaa",
        "spm",
        "spm",
        "subscriber-mgmt",
    ]


def test_pem_rating_group_quota_fields():
    qid = _run(".pem.rating-group.rg1.rating-group-id", _PEM_CONF)
    assert qid.values_per_file["mem://1"] == ["100"]
    quota = _run(".pem.rating-group.rg1.default-quota", _PEM_CONF)
    assert quota.values_per_file["mem://1"] == ["1000"]


# ---------------------------------------------------------------------------
# Bundle 1 — cert/key metadata
# ---------------------------------------------------------------------------

_CERT_KEY_CONF = """cm cert /Common/dtca.crt {
    cache-path "/config/big3d/client.crt"
    revision 1
    issuer "CN=Acme Root CA"
    subject "CN=bigip-1.example.com"
    subject-alternative-name "DNS:bigip-1.example.com,DNS:bigip-1"
    expiration-date 2099012345
    expiration-string "Jan 1 00:00:00 2099 GMT"
    fingerprint "SHA256/AB:CD:EF"
    serial-number 1001
    version 3
    key-type rsa-public
    certificate-key-size 2048
    is-bundle false
    email "ops@example.com"
    source-path "file:///config/big3d/client.crt"
    system-path "/config/big3d/client.crt"
    size 1234
    mode 33188
    create-time "2024-01-02T03:04:05Z"
    created-by admin
    last-update-time "2024-06-07T08:09:10Z"
    updated-by ops
}
cm key /Common/dtca.key {
    cache-path "/config/big3d/client.key"
    revision 1
    key-size 2048
    key-type rsa-private
    security-type normal
    source-path "file:///config/big3d/client.key"
    system-path "/config/big3d/client.key"
    size 1675
    mode 33184
    create-time "2024-01-02T03:04:05Z"
    created-by admin
    last-update-time "2024-06-07T08:09:10Z"
    updated-by ops
}
sys file ssl-cert /Common/example.crt {
    source-path "file:///config/ssl/ssl.crt/example.crt"
    revision 1
    issuer "CN=Example CA"
    subject "CN=www.example.com"
    subject-alternative-name "DNS:www.example.com,DNS:example.com"
    expiration-date 2099012345
    expiration-string "Jan 1 00:00:00 2099 GMT"
    fingerprint "SHA256/12:34:56"
    key-size 2048
    key-type rsa-public
    is-bundle false
    certificate-key-size 2048
    issuer-cert /Common/root.crt
    serial-number 1002
    version 3
    bundle-certificates { /Common/root.crt /Common/intermediate.crt }
    cert-validation-options { ocsp }
    cert-validators { /Common/ocsp1 }
    checksum "SHA1:abcdef"
    mode 33188
    size 3210
    create-time "2024-01-02T03:04:05Z"
    created-by admin
    last-update-time "2024-06-07T08:09:10Z"
    updated-by ops
}
sys file ssl-cert /Common/root.crt {
    source-path "file:///config/ssl/ssl.crt/root.crt"
    issuer "CN=Example CA"
}
sys file ssl-key /Common/example.key {
    source-path "file:///config/ssl/ssl.key/example.key"
    revision 1
    key-size 2048
    key-type rsa-private
    security-type normal
    checksum "SHA1:fedcba"
    mode 33184
    size 1675
    create-time "2024-01-02T03:04:05Z"
    created-by admin
    last-update-time "2024-06-07T08:09:10Z"
    updated-by ops
}
"""


def test_cm_cert_surfaces_issuer_subject_and_san():
    issuer = _run('.cm.cert["/Common/dtca.crt"].issuer', _CERT_KEY_CONF)
    assert issuer.values_per_file["mem://1"] == ["CN=Acme Root CA"]
    subj = _run('.cm.cert["/Common/dtca.crt"].subject', _CERT_KEY_CONF)
    assert subj.values_per_file["mem://1"] == ["CN=bigip-1.example.com"]
    san = _run('.cm.cert["/Common/dtca.crt"].subject-alternative-name', _CERT_KEY_CONF)
    assert san.values_per_file["mem://1"] == ["DNS:bigip-1.example.com,DNS:bigip-1"]


def test_cm_cert_serial_version_and_keytype():
    serial = _run('.cm.cert["/Common/dtca.crt"].serial-number', _CERT_KEY_CONF)
    assert serial.values_per_file["mem://1"] == ["1001"]
    version = _run('.cm.cert["/Common/dtca.crt"].version', _CERT_KEY_CONF)
    assert version.values_per_file["mem://1"] == ["3"]
    ktype = _run('.cm.cert["/Common/dtca.crt"].key-type', _CERT_KEY_CONF)
    assert ktype.values_per_file["mem://1"] == ["rsa-public"]


def test_cm_cert_audit_fields_round_trip():
    ct = _run('.cm.cert["/Common/dtca.crt"].create-time', _CERT_KEY_CONF)
    assert ct.values_per_file["mem://1"] == ["2024-01-02T03:04:05Z"]
    cb = _run('.cm.cert["/Common/dtca.crt"].created-by', _CERT_KEY_CONF)
    assert cb.values_per_file["mem://1"] == ["admin"]


def test_cm_key_size_type_and_security():
    sz = _run('.cm.key["/Common/dtca.key"].key-size', _CERT_KEY_CONF)
    assert sz.values_per_file["mem://1"] == ["2048"]
    sec = _run('.cm.key["/Common/dtca.key"].security-type', _CERT_KEY_CONF)
    assert sec.values_per_file["mem://1"] == ["normal"]


def test_sys_file_ssl_cert_issuer_cert_walks_via_path_ref():
    result = _run('.sys.file-ssl-cert["/Common/example.crt"].issuer-cert.issuer', _CERT_KEY_CONF)
    assert result.values_per_file["mem://1"] == ["CN=Example CA"]


def test_sys_file_ssl_cert_bundle_certificates_listed():
    bundle = _run('.sys.file-ssl-cert["/Common/example.crt"].bundle-certificates', _CERT_KEY_CONF)
    [items] = bundle.values_per_file["mem://1"]
    assert sorted(items) == ["/Common/intermediate.crt", "/Common/root.crt"]


def test_sys_file_ssl_cert_validators_and_options():
    opts = _run(
        '.sys.file-ssl-cert["/Common/example.crt"].cert-validation-options',
        _CERT_KEY_CONF,
    )
    [items] = opts.values_per_file["mem://1"]
    assert items == ["ocsp"]


def test_sys_file_ssl_key_checksum_and_audit_fields():
    cs = _run('.sys.file-ssl-key["/Common/example.key"].checksum', _CERT_KEY_CONF)
    assert cs.values_per_file["mem://1"] == ["SHA1:fedcba"]
    ub = _run('.sys.file-ssl-key["/Common/example.key"].updated-by', _CERT_KEY_CONF)
    assert ub.values_per_file["mem://1"] == ["ops"]


# ---------------------------------------------------------------------------
# Bundle 2 — ltm virtual flags + refs
# ---------------------------------------------------------------------------

_VS_FLAGS_CONF = """net vlan /Common/ext { tag 10 }
net vlan /Common/int { tag 20 }
apm policy access-policy /Common/per_flow_ap { }
ltm virtual /Common/fwd_vs {
    destination /Common/0.0.0.0:any
    mask any
    address-status no
    auto-discovery enabled
    cmp-enabled yes
    eviction-protected enabled
    ip-forward
    nat64 disabled
    gtm-score 42
    mirror enabled
    service-down-immediate-action reset
    source-port preserve-strict
    serverssl-use-sni enabled
    rate-limit-dst-mask 24
    rate-limit-src-mask 16
    rate-class /Common/rc1
    transparent-nexthop /Common/ext
    per-flow-request-access-policy /Common/per_flow_ap
}
ltm virtual /Common/internal_vs {
    destination /Common/10.1.1.1:80
    internal
}
ltm virtual /Common/dhcp_vs {
    destination /Common/0.0.0.0:67
    dhcp-relay
}
ltm virtual /Common/l2_vs {
    destination /Common/10.1.2.1:80
    l2-forward
}
ltm virtual /Common/reject_vs {
    destination /Common/10.1.3.1:80
    reject
}
"""


def test_virtual_address_status_and_auto_discovery():
    addr_status = _run(".ltm.virtual.fwd_vs.address-status", _VS_FLAGS_CONF)
    assert addr_status.values_per_file["mem://1"] == ["no"]
    ad = _run(".ltm.virtual.fwd_vs.auto-discovery", _VS_FLAGS_CONF)
    assert ad.values_per_file["mem://1"] == ["enabled"]


def test_virtual_cmp_enabled_and_eviction_protected():
    cmp = _run(".ltm.virtual.fwd_vs.cmp-enabled", _VS_FLAGS_CONF)
    assert cmp.values_per_file["mem://1"] == ["yes"]
    ev = _run(".ltm.virtual.fwd_vs.eviction-protected", _VS_FLAGS_CONF)
    assert ev.values_per_file["mem://1"] == ["enabled"]


def test_virtual_bare_flags_are_true_when_present():
    intl = _run(".ltm.virtual.internal_vs.internal", _VS_FLAGS_CONF)
    assert intl.values_per_file["mem://1"] == [True]
    dhcp = _run(".ltm.virtual.dhcp_vs.dhcp-relay", _VS_FLAGS_CONF)
    assert dhcp.values_per_file["mem://1"] == [True]
    ipf = _run(".ltm.virtual.fwd_vs.ip-forward", _VS_FLAGS_CONF)
    assert ipf.values_per_file["mem://1"] == [True]
    l2 = _run(".ltm.virtual.l2_vs.l2-forward", _VS_FLAGS_CONF)
    assert l2.values_per_file["mem://1"] == [True]
    rj = _run(".ltm.virtual.reject_vs.reject", _VS_FLAGS_CONF)
    assert rj.values_per_file["mem://1"] == [True]


def test_virtual_bare_flags_are_false_when_absent():
    intl = _run(".ltm.virtual.fwd_vs.internal", _VS_FLAGS_CONF)
    assert intl.values_per_file["mem://1"] == [False]


def test_virtual_gtm_score_mirror_and_service_down_action():
    score = _run(".ltm.virtual.fwd_vs.gtm-score", _VS_FLAGS_CONF)
    assert score.values_per_file["mem://1"] == ["42"]
    mirror = _run(".ltm.virtual.fwd_vs.mirror", _VS_FLAGS_CONF)
    assert mirror.values_per_file["mem://1"] == ["enabled"]
    sda = _run(".ltm.virtual.fwd_vs.service-down-immediate-action", _VS_FLAGS_CONF)
    assert sda.values_per_file["mem://1"] == ["reset"]


def test_virtual_source_port_and_rate_limit_masks():
    sp = _run(".ltm.virtual.fwd_vs.source-port", _VS_FLAGS_CONF)
    assert sp.values_per_file["mem://1"] == ["preserve-strict"]
    dst_mask = _run(".ltm.virtual.fwd_vs.rate-limit-dst-mask", _VS_FLAGS_CONF)
    assert dst_mask.values_per_file["mem://1"] == ["24"]
    src_mask = _run(".ltm.virtual.fwd_vs.rate-limit-src-mask", _VS_FLAGS_CONF)
    assert src_mask.values_per_file["mem://1"] == ["16"]


def test_virtual_transparent_nexthop_walks_to_vlan_tag():
    result = _run(".ltm.virtual.fwd_vs.transparent-nexthop.tag", _VS_FLAGS_CONF)
    assert result.values_per_file["mem://1"] == [10]


def test_virtual_per_flow_access_policy_pathref_resolves():
    result = _run(
        ".ltm.virtual.fwd_vs.per-flow-request-access-policy.full-path",
        _VS_FLAGS_CONF,
    )
    assert result.values_per_file["mem://1"] == ["/Common/per_flow_ap"]


def test_virtual_rate_class_is_projected_as_string():
    rc = _run(".ltm.virtual.fwd_vs.rate-class", _VS_FLAGS_CONF)
    assert rc.values_per_file["mem://1"] == ["/Common/rc1"]


# ---------------------------------------------------------------------------
# Bundle 3 — ltm pool scalars
# ---------------------------------------------------------------------------

_POOL_SCALARS_CONF = """ltm profile http /Common/http_profile1 { }
ltm profile tcp /Common/tcp_profile1 { }
ltm pool /Common/web_pool {
    members {
        /Common/n1:80 { address 10.0.0.1 }
    }
    monitor /Common/http
    load-balancing-mode least-connections-member
    connection-limit 5000
    rate-limit 250
    ratio 7
    down-interval 30
    interval 5
    min-up-members-action reboot
    min-up-members-checking enabled
    ip-tos-to-client mimic
    ip-tos-to-server pass-through
    link-qos-to-client 0
    link-qos-to-server 5
    gateway-failsafe-device /Common/bigip-1
    ignore-persisted-weight enabled
    inherit-profile enabled
    queue-on-connection-limit enabled
    address-family ipv4
    autopopulate disabled
    profiles { /Common/http_profile1 /Common/tcp_profile1 }
}
"""


def test_pool_connection_and_rate_limits():
    cl = _run(".ltm.pool.web_pool.connection-limit", _POOL_SCALARS_CONF)
    assert cl.values_per_file["mem://1"] == ["5000"]
    rl = _run(".ltm.pool.web_pool.rate-limit", _POOL_SCALARS_CONF)
    assert rl.values_per_file["mem://1"] == ["250"]


def test_pool_ratio_and_intervals():
    ratio = _run(".ltm.pool.web_pool.ratio", _POOL_SCALARS_CONF)
    assert ratio.values_per_file["mem://1"] == ["7"]
    down = _run(".ltm.pool.web_pool.down-interval", _POOL_SCALARS_CONF)
    assert down.values_per_file["mem://1"] == ["30"]
    interval = _run(".ltm.pool.web_pool.interval", _POOL_SCALARS_CONF)
    assert interval.values_per_file["mem://1"] == ["5"]


def test_pool_min_up_members_action_and_checking():
    action = _run(".ltm.pool.web_pool.min-up-members-action", _POOL_SCALARS_CONF)
    assert action.values_per_file["mem://1"] == ["reboot"]
    checking = _run(".ltm.pool.web_pool.min-up-members-checking", _POOL_SCALARS_CONF)
    assert checking.values_per_file["mem://1"] == ["enabled"]


def test_pool_ip_tos_and_link_qos():
    ipc = _run(".ltm.pool.web_pool.ip-tos-to-client", _POOL_SCALARS_CONF)
    assert ipc.values_per_file["mem://1"] == ["mimic"]
    ips = _run(".ltm.pool.web_pool.ip-tos-to-server", _POOL_SCALARS_CONF)
    assert ips.values_per_file["mem://1"] == ["pass-through"]
    lqc = _run(".ltm.pool.web_pool.link-qos-to-client", _POOL_SCALARS_CONF)
    assert lqc.values_per_file["mem://1"] == ["0"]
    lqs = _run(".ltm.pool.web_pool.link-qos-to-server", _POOL_SCALARS_CONF)
    assert lqs.values_per_file["mem://1"] == ["5"]


def test_pool_gateway_failsafe_device_and_inherit_profile():
    gfd = _run(".ltm.pool.web_pool.gateway-failsafe-device", _POOL_SCALARS_CONF)
    assert gfd.values_per_file["mem://1"] == ["/Common/bigip-1"]
    ip = _run(".ltm.pool.web_pool.inherit-profile", _POOL_SCALARS_CONF)
    assert ip.values_per_file["mem://1"] == ["enabled"]


def test_pool_address_family_and_autopopulate():
    af = _run(".ltm.pool.web_pool.address-family", _POOL_SCALARS_CONF)
    assert af.values_per_file["mem://1"] == ["ipv4"]
    ap = _run(".ltm.pool.web_pool.autopopulate", _POOL_SCALARS_CONF)
    assert ap.values_per_file["mem://1"] == ["disabled"]


def test_pool_profiles_resolve_through_path_ref():
    result = _run(".ltm.pool.web_pool.profiles[].full-path", _POOL_SCALARS_CONF)
    assert sorted(result.values_per_file["mem://1"]) == [
        "/Common/http_profile1",
        "/Common/tcp_profile1",
    ]


# ---------------------------------------------------------------------------
# Bundle 4 — ltm persistence behaviour flags
# ---------------------------------------------------------------------------

_PERSISTENCE_FLAGS_CONF = """ltm persistence cookie /Common/sess_cookie {
    description "session cookie"
    timeout 1800
    match-across-pools enabled
    match-across-services enabled
    match-across-virtuals disabled
    mirror enabled
    override-connection-limit disabled
    always-send enabled
    cookie-name "ASESSION"
    cookie-encryption preferred
    cookie-encryption-passphrase "$M$secret"
    httponly enabled
    secure enabled
    expiration 1d:0:0:0
    method insert
}
ltm persistence hash /Common/sess_hash {
    timeout 600
    hash-length 32
    hash-offset 0
    method carp
}
ltm persistence source-addr /Common/sess_src {
    timeout 600
    mask 255.255.255.0
}
"""


def test_persistence_match_across_flags_are_visible():
    pools = _run(".ltm.persistence.sess_cookie.match-across-pools", _PERSISTENCE_FLAGS_CONF)
    assert pools.values_per_file["mem://1"] == ["enabled"]
    svcs = _run(".ltm.persistence.sess_cookie.match-across-services", _PERSISTENCE_FLAGS_CONF)
    assert svcs.values_per_file["mem://1"] == ["enabled"]
    vss = _run(".ltm.persistence.sess_cookie.match-across-virtuals", _PERSISTENCE_FLAGS_CONF)
    assert vss.values_per_file["mem://1"] == ["disabled"]


def test_persistence_mirror_and_override_limit():
    mirror = _run(".ltm.persistence.sess_cookie.mirror", _PERSISTENCE_FLAGS_CONF)
    assert mirror.values_per_file["mem://1"] == ["enabled"]
    ovr = _run(
        ".ltm.persistence.sess_cookie.override-connection-limit",
        _PERSISTENCE_FLAGS_CONF,
    )
    assert ovr.values_per_file["mem://1"] == ["disabled"]


def test_persistence_cookie_name_is_unquoted():
    name = _run(".ltm.persistence.sess_cookie.cookie-name", _PERSISTENCE_FLAGS_CONF)
    assert name.values_per_file["mem://1"] == ["ASESSION"]


def test_persistence_cookie_encryption_and_security_flags():
    enc = _run(".ltm.persistence.sess_cookie.cookie-encryption", _PERSISTENCE_FLAGS_CONF)
    assert enc.values_per_file["mem://1"] == ["preferred"]
    secure = _run(".ltm.persistence.sess_cookie.secure", _PERSISTENCE_FLAGS_CONF)
    assert secure.values_per_file["mem://1"] == ["enabled"]
    ho = _run(".ltm.persistence.sess_cookie.httponly", _PERSISTENCE_FLAGS_CONF)
    assert ho.values_per_file["mem://1"] == ["enabled"]


def test_persistence_expiration_method_and_always_send():
    exp = _run(".ltm.persistence.sess_cookie.expiration", _PERSISTENCE_FLAGS_CONF)
    assert exp.values_per_file["mem://1"] == ["1d:0:0:0"]
    method = _run(".ltm.persistence.sess_cookie.method", _PERSISTENCE_FLAGS_CONF)
    assert method.values_per_file["mem://1"] == ["insert"]
    asend = _run(".ltm.persistence.sess_cookie.always-send", _PERSISTENCE_FLAGS_CONF)
    assert asend.values_per_file["mem://1"] == ["enabled"]


def test_persistence_hash_length_and_offset():
    hl = _run(".ltm.persistence.sess_hash.hash-length", _PERSISTENCE_FLAGS_CONF)
    assert hl.values_per_file["mem://1"] == ["32"]
    ho = _run(".ltm.persistence.sess_hash.hash-offset", _PERSISTENCE_FLAGS_CONF)
    assert ho.values_per_file["mem://1"] == ["0"]
    method = _run(".ltm.persistence.sess_hash.method", _PERSISTENCE_FLAGS_CONF)
    assert method.values_per_file["mem://1"] == ["carp"]


# ---------------------------------------------------------------------------
# Bundle 5 — net.* enrichments
# ---------------------------------------------------------------------------

_NET_ENRICH_CONF = """net vlan /Common/vbase {
    tag 100
    mtu 9000
    cmp-hash src-ip
    failsafe enabled
    failsafe-action reboot
    failsafe-timeout 30
    fwd-mode l3
    hardware-syncookie enabled
    learning enable-forward
    tag-mode customer
    virtual-wire disabled
    source-checking enabled
    syn-flood-rate-limit 1000
    syncache-threshold 4096
    service-policy /Common/svc_pol
}
net vlan /Common/v10 { tag 10 }
net vlan /Common/v20 { tag 20 }
net self /Common/self1 {
    address 10.0.0.1/24
    vlan /Common/vbase
    service-policy /Common/svc_pol
    fw-enforced-policy /Common/edge_fw
    fw-staged-policy /Common/edge_fw_staged
    inherited-traffic-group true
    address-source from-user
}
net route-domain /Common/rd_full {
    id 5
    vlans { /Common/vbase }
    bwc-policy /Common/bwc1
    connection-limit 100000
    flow-eviction-policy /Common/fep1
    routing-protocol { BGP OSPFv2 }
    security-nat-policy /Common/snat_pol
    service-policy /Common/svc_pol
}
net interface 1.1 {
    media-fixed 10000T-FD
    mtu 9000
    flow-control tx-rx
    mac-address 00:11:22:33:44:55
    media-active 10000T-FD
    media-max 10000T-FD
    port-fwd-mode l3
    qinq-ethertype 0x8100
    stp enabled
    stp-edge-port enabled
    stp-link-type auto
    stp-auto-edge-port enabled
    sflow {
        poll-interval 0
        poll-interval-global yes
    }
    vendor "F5 Networks"
    vendor-oui 009065
    vendor-partnum "OPT-0026"
    vendor-revision 03
    transmitter-technology fiber
    lacp-port-priority 32768
    virtual-wire disabled
}
net tunnels tunnel /Common/tun1 {
    profile /Common/gre
    local-address 10.0.99.1
    remote-address 10.0.99.2
    mtu 1500
    mode bidirectional
    idle-timeout 300
    auto-lasthop default
    secondary-address 10.0.99.3
    traffic-group /Common/traffic-group-1
    transparent disabled
    key 12345
    use-pmtu enabled
    tos preserve
}
cm traffic-group /Common/traffic-group-1 { unit-id 1 }
net dns-resolver /Common/dnsr_full {
    route-domain 0
    cache-size 1048576
    nameservers { 1.1.1.1 1.0.0.1 }
    answer-default-zones yes
    prefetch enabled
    nameserver-min-rtt 5
    nameserver-ttl 30
    outbound-msg-retry 3
}
net stp /Common/cist1 {
    interfaces { 1.1 1.2 }
    mode rstp
    priority 32768
    external-path-cost 20000
    internal-path-cost 20000
    vlans { /Common/v10 /Common/v20 }
}
"""


def test_net_vlan_failsafe_and_learning():
    fa = _run(".net.vlan.vbase.failsafe-action", _NET_ENRICH_CONF)
    assert fa.values_per_file["mem://1"] == ["reboot"]
    learning = _run(".net.vlan.vbase.learning", _NET_ENRICH_CONF)
    assert learning.values_per_file["mem://1"] == ["enable-forward"]


def test_net_vlan_fwd_mode_and_syncookies():
    fm = _run(".net.vlan.vbase.fwd-mode", _NET_ENRICH_CONF)
    assert fm.values_per_file["mem://1"] == ["l3"]
    hs = _run(".net.vlan.vbase.hardware-syncookie", _NET_ENRICH_CONF)
    assert hs.values_per_file["mem://1"] == ["enabled"]
    rl = _run(".net.vlan.vbase.syn-flood-rate-limit", _NET_ENRICH_CONF)
    assert rl.values_per_file["mem://1"] == ["1000"]


def test_net_self_service_and_fw_policies():
    sp = _run(".net.self.self1.service-policy", _NET_ENRICH_CONF)
    assert sp.values_per_file["mem://1"] == ["/Common/svc_pol"]
    fw = _run(".net.self.self1.fw-enforced-policy", _NET_ENRICH_CONF)
    assert fw.values_per_file["mem://1"] == ["/Common/edge_fw"]
    fws = _run(".net.self.self1.fw-staged-policy", _NET_ENRICH_CONF)
    assert fws.values_per_file["mem://1"] == ["/Common/edge_fw_staged"]


def test_net_self_inherited_traffic_group_and_address_source():
    itg = _run(".net.self.self1.inherited-traffic-group", _NET_ENRICH_CONF)
    assert itg.values_per_file["mem://1"] == ["true"]
    addr_src = _run(".net.self.self1.address-source", _NET_ENRICH_CONF)
    assert addr_src.values_per_file["mem://1"] == ["from-user"]


def test_net_route_domain_governance_fields():
    bwc = _run(".net.route-domain.rd_full.bwc-policy", _NET_ENRICH_CONF)
    assert bwc.values_per_file["mem://1"] == ["/Common/bwc1"]
    cl = _run(".net.route-domain.rd_full.connection-limit", _NET_ENRICH_CONF)
    assert cl.values_per_file["mem://1"] == ["100000"]
    flep = _run(".net.route-domain.rd_full.flow-eviction-policy", _NET_ENRICH_CONF)
    assert flep.values_per_file["mem://1"] == ["/Common/fep1"]


def test_net_route_domain_routing_protocol_is_a_list():
    rp = _run(".net.route-domain.rd_full.routing-protocol", _NET_ENRICH_CONF)
    [protos] = rp.values_per_file["mem://1"]
    assert sorted(protos) == ["BGP", "OSPFv2"]


def test_net_interface_mtu_mac_and_media():
    mtu = _run('.net.interface["1.1"].mtu', _NET_ENRICH_CONF)
    assert mtu.values_per_file["mem://1"] == ["9000"]
    mac = _run('.net.interface["1.1"].mac-address', _NET_ENRICH_CONF)
    assert mac.values_per_file["mem://1"] == ["00:11:22:33:44:55"]
    ma = _run('.net.interface["1.1"].media-active', _NET_ENRICH_CONF)
    assert ma.values_per_file["mem://1"] == ["10000T-FD"]


def test_net_interface_stp_and_sflow_subblock():
    stp = _run('.net.interface["1.1"].stp', _NET_ENRICH_CONF)
    assert stp.values_per_file["mem://1"] == ["enabled"]
    spi = _run('.net.interface["1.1"].sflow-poll-interval', _NET_ENRICH_CONF)
    assert spi.values_per_file["mem://1"] == ["0"]
    spig = _run('.net.interface["1.1"].sflow-poll-interval-global', _NET_ENRICH_CONF)
    assert spig.values_per_file["mem://1"] == ["yes"]


def test_net_interface_vendor_metadata_is_unquoted():
    vendor = _run('.net.interface["1.1"].vendor', _NET_ENRICH_CONF)
    assert vendor.values_per_file["mem://1"] == ["F5 Networks"]
    part = _run('.net.interface["1.1"].vendor-partnum', _NET_ENRICH_CONF)
    assert part.values_per_file["mem://1"] == ["OPT-0026"]


def test_net_tunnel_traffic_group_pathref_walks_to_unit_id():
    result = _run(".net.tunnels-tunnel.tun1.traffic-group.unit-id", _NET_ENRICH_CONF)
    assert result.values_per_file["mem://1"] == ["1"]


def test_net_tunnel_mtu_mode_and_pmtu():
    mtu = _run(".net.tunnels-tunnel.tun1.mtu", _NET_ENRICH_CONF)
    assert mtu.values_per_file["mem://1"] == ["1500"]
    mode = _run(".net.tunnels-tunnel.tun1.mode", _NET_ENRICH_CONF)
    assert mode.values_per_file["mem://1"] == ["bidirectional"]
    pmtu = _run(".net.tunnels-tunnel.tun1.use-pmtu", _NET_ENRICH_CONF)
    assert pmtu.values_per_file["mem://1"] == ["enabled"]


def test_net_dns_resolver_nameservers_and_prefetch():
    ns = _run(".net.dns-resolver.dnsr_full.nameservers", _NET_ENRICH_CONF)
    [servers] = ns.values_per_file["mem://1"]
    assert sorted(servers) == ["1.0.0.1", "1.1.1.1"]
    pf = _run(".net.dns-resolver.dnsr_full.prefetch", _NET_ENRICH_CONF)
    assert pf.values_per_file["mem://1"] == ["enabled"]
    adz = _run(".net.dns-resolver.dnsr_full.answer-default-zones", _NET_ENRICH_CONF)
    assert adz.values_per_file["mem://1"] == ["yes"]


def test_net_stp_priority_path_cost_and_vlans():
    pri = _run(".net.stp.cist1.priority", _NET_ENRICH_CONF)
    assert pri.values_per_file["mem://1"] == ["32768"]
    ec = _run(".net.stp.cist1.external-path-cost", _NET_ENRICH_CONF)
    assert ec.values_per_file["mem://1"] == ["20000"]
    vlans = _run(".net.stp.cist1.vlans[].tag", _NET_ENRICH_CONF)
    assert sorted(vlans.values_per_file["mem://1"]) == [10, 20]


# ---------------------------------------------------------------------------
# Bundle 6 — apm enrichments (oauth db-instance + policy agent)
# ---------------------------------------------------------------------------

_APM_BUNDLE_CONF = """apm oauth db-instance /Common/oauth_db {
    description "primary oauth db"
    db-name oauth
    purge-frequency 86400
    purge-time "03:00"
}
apm policy agent aaa-active-directory /Common/ap1_act_ad_ag {
    customization-group /Common/ap1_act_ad_cg
    server /Common/ad_aaa
    fetch-nested-groups true
    fetch-primary-groups true
    show-extended-error true
    upn enabled
    max-logon-attempt 3
}
apm policy agent aaa-radius /Common/ap1_rad_ag {
    customization-group /Common/ap1_rad_cg
    server /Common/radius_aaa
    password-source session.radius.password
    username-source session.logon.last.username
    auth-max-logon-attempt 5
}
apm policy agent aaa-saml /Common/ap1_saml_ag {
    attribute-consuming-service /Common/acs1
    attr-consuming-service-session-var session.saml.acs
}
apm policy agent ending-allow /Common/ap1_allow { }
apm policy agent endpoint-machine-info /Common/ap1_minfo {
    hints "session.client.os"
}
apm policy agent ip-geolocation-lookup /Common/ap1_geo { }
"""


def test_apm_oauth_db_instance_db_name_and_purge():
    db = _run(".apm.oauth-db-instance.oauth_db.db-name", _APM_BUNDLE_CONF)
    assert db.values_per_file["mem://1"] == ["oauth"]
    pf = _run(".apm.oauth-db-instance.oauth_db.purge-frequency", _APM_BUNDLE_CONF)
    assert pf.values_per_file["mem://1"] == ["86400"]
    pt = _run(".apm.oauth-db-instance.oauth_db.purge-time", _APM_BUNDLE_CONF)
    assert pt.values_per_file["mem://1"] == ["03:00"]


def test_apm_policy_agent_ad_fetch_and_upn():
    fn = _run(".apm.policy-agent.ap1_act_ad_ag.fetch-nested-groups", _APM_BUNDLE_CONF)
    assert fn.values_per_file["mem://1"] == ["true"]
    fp = _run(".apm.policy-agent.ap1_act_ad_ag.fetch-primary-groups", _APM_BUNDLE_CONF)
    assert fp.values_per_file["mem://1"] == ["true"]
    upn = _run(".apm.policy-agent.ap1_act_ad_ag.upn", _APM_BUNDLE_CONF)
    assert upn.values_per_file["mem://1"] == ["enabled"]


def test_apm_policy_agent_max_logon_attempt_variants():
    a = _run(".apm.policy-agent.ap1_act_ad_ag.max-logon-attempt", _APM_BUNDLE_CONF)
    assert a.values_per_file["mem://1"] == ["3"]
    b = _run(".apm.policy-agent.ap1_rad_ag.auth-max-logon-attempt", _APM_BUNDLE_CONF)
    assert b.values_per_file["mem://1"] == ["5"]


def test_apm_policy_agent_password_and_username_source():
    ps = _run(".apm.policy-agent.ap1_rad_ag.password-source", _APM_BUNDLE_CONF)
    assert ps.values_per_file["mem://1"] == ["session.radius.password"]
    us = _run(".apm.policy-agent.ap1_rad_ag.username-source", _APM_BUNDLE_CONF)
    assert us.values_per_file["mem://1"] == ["session.logon.last.username"]


def test_apm_policy_agent_saml_attribute_consuming_service():
    acs = _run(
        ".apm.policy-agent.ap1_saml_ag.attribute-consuming-service",
        _APM_BUNDLE_CONF,
    )
    assert acs.values_per_file["mem://1"] == ["/Common/acs1"]
    var = _run(
        ".apm.policy-agent.ap1_saml_ag.attr-consuming-service-session-var",
        _APM_BUNDLE_CONF,
    )
    assert var.values_per_file["mem://1"] == ["session.saml.acs"]


def test_apm_policy_agent_extra_sub_types_are_recognised():
    result = _run(".apm.policy-agent[] | .agent-type", _APM_BUNDLE_CONF)
    assert sorted(result.values_per_file["mem://1"]) == [
        "aaa-active-directory",
        "aaa-radius",
        "aaa-saml",
        "ending-allow",
        "endpoint-machine-info",
        "ip-geolocation-lookup",
    ]


def test_apm_policy_agent_hints_carry_through():
    hints = _run(".apm.policy-agent.ap1_minfo.hints", _APM_BUNDLE_CONF)
    assert hints.values_per_file["mem://1"] == ["session.client.os"]


# ---------------------------------------------------------------------------
# Bundle 7 — ltm virtual-address
# ---------------------------------------------------------------------------

_VIRTUAL_ADDRESS_CONF = """ltm virtual-address /Common/192.0.2.10 {
    address 192.0.2.10
    arp enabled
    auto-delete true
    connection-limit 1000
    description "front-end VIP"
    floating enabled
    icmp-echo enabled
    inherited-traffic-group true
    mask 255.255.255.255
    route-advertisement disabled
    server-scope any
    spanning disabled
    traffic-group /Common/traffic-group-1
    unit 1
}
ltm virtual-address /Common/198.51.100.20 {
    address 198.51.100.20
    arp disabled
    disabled
    icmp-echo disabled
    mask 255.255.255.255
    traffic-group /Common/traffic-group-1
}
cm traffic-group /Common/traffic-group-1 {
    unit-id 1
}
"""


def test_ltm_virtual_address_projects_core_scalars():
    base = '.ltm.virtual-address["/Common/192.0.2.10"]'
    assert _run(f"{base}.address", _VIRTUAL_ADDRESS_CONF).values_per_file["mem://1"] == [
        "192.0.2.10"
    ]
    assert _run(f"{base}.mask", _VIRTUAL_ADDRESS_CONF).values_per_file["mem://1"] == [
        "255.255.255.255"
    ]
    assert _run(f"{base}.arp", _VIRTUAL_ADDRESS_CONF).values_per_file["mem://1"] == ["enabled"]
    assert _run(f'{base}."icmp-echo"', _VIRTUAL_ADDRESS_CONF).values_per_file["mem://1"] == [
        "enabled"
    ]
    assert _run(f"{base}.floating", _VIRTUAL_ADDRESS_CONF).values_per_file["mem://1"] == ["enabled"]


def test_ltm_virtual_address_state_flag_from_bare_keyword():
    """``ltm virtual-address`` accepts bare ``enabled`` / ``disabled``
    tokens just like ``ltm virtual`` / ``ltm node`` — surface as the
    ``state`` field.
    """
    res = _run(".ltm.virtual-address[] | .state", _VIRTUAL_ADDRESS_CONF)
    assert sorted(res.values_per_file["mem://1"]) == ["", "disabled"]


def test_ltm_virtual_address_description_unquoted():
    res = _run(
        '.ltm.virtual-address["/Common/192.0.2.10"].description',
        _VIRTUAL_ADDRESS_CONF,
    )
    assert res.values_per_file["mem://1"] == ["front-end VIP"]


def test_ltm_virtual_address_traffic_group_is_pathref_into_cm():
    """``traffic-group`` chains into ``cm traffic-group`` — quick
    sanity-check that the cross-module PathRef resolves.
    """
    res = _run(
        '.ltm.virtual-address["/Common/192.0.2.10"].traffic-group.unit-id',
        _VIRTUAL_ADDRESS_CONF,
    )
    assert res.values_per_file["mem://1"] == ["1"]


def test_ltm_virtual_address_route_advertisement_and_scope():
    base = '.ltm.virtual-address["/Common/192.0.2.10"]'
    conf = _VIRTUAL_ADDRESS_CONF
    assert _run(f'{base}."route-advertisement"', conf).values_per_file["mem://1"] == ["disabled"]
    assert _run(f'{base}."server-scope"', conf).values_per_file["mem://1"] == ["any"]
    assert _run(f"{base}.spanning", conf).values_per_file["mem://1"] == ["disabled"]
    assert _run(f"{base}.unit", conf).values_per_file["mem://1"] == ["1"]
    assert _run(f'{base}."inherited-traffic-group"', conf).values_per_file["mem://1"] == ["true"]
    assert _run(f'{base}."auto-delete"', conf).values_per_file["mem://1"] == ["true"]
    assert _run(f'{base}."connection-limit"', conf).values_per_file["mem://1"] == ["1000"]


# ---------------------------------------------------------------------------
# Bundle 8 — auth.* (partitions, users, password / source / remote-role
# singletons, LDAP / RADIUS / TACACS / cert-LDAP / APM auth)
# ---------------------------------------------------------------------------

_AUTH_BUNDLE_CONF = """auth partition Common {
    description "default partition"
    default-route-domain 0
}
auth partition Tenant_A {
    description "App tenant"
    default-route-domain 0
}
auth user admin {
    description "Default admin"
    shell bash
    partition Common
    encrypted-password $6$abcdef
    partition-access {
        all-partitions { role admin }
        Tenant_A { role operator }
    }
}
auth password {
    minimum-length 6
}
auth password-policy {
    minimum-length 8
    required-uppercase 1
    required-numeric 1
    max-login-failures 5
}
auth source {
    fallback true
    type radius
}
auth remote-role {
    role-info {
        TEAM1 { role admin }
        TEAM2 { role operator }
    }
}
auth remote-user {
    default-partition all
    default-role resource-admin
    remote-console-access tmsh
}
auth login-failures { }
auth ldap /Common/system-auth {
    bind-dn cn=admin,dc=example,dc=test
    bind-timeout 40
    filter "(uid=%u)"
    group-dn ou=groups,dc=example,dc=test
    search-base-dn dc=example,dc=test
    servers { ldap1.example.test ldap2.example.test }
    ssl enabled
    version 3
}
auth radius /Common/system-auth-radius {
    servers { /Common/system_auth_name1 /Common/system_auth_name2 }
    service-type login
}
auth radius-server /Common/system_auth_name1 {
    port 1811
    secret obscured1
    server 192.0.2.4
}
auth radius-server /Common/system_auth_name2 {
    port 1888
    secret obscured2
    server 198.51.100.4
}
auth tacacs /Common/system-auth-tac {
    protocol ip
    secret obscured
    servers { 192.0.2.5 198.51.100.5 }
    service ppp
}
auth cert-ldap /Common/cert-ldap-1 {
    bind-dn cn=cert-reader
    search-base-dn dc=example,dc=test
    servers { ldap1.example.test }
}
auth apm-auth /Common/apm-auth-1 {
    profile /Common/access_policy_1
}
net route-domain /Common/0 {
    id 0
}
"""


def test_auth_partition_projects_description_and_default_route_domain():
    res = _run(".auth.partition[] | .name", _AUTH_BUNDLE_CONF)
    assert sorted(res.values_per_file["mem://1"]) == ["Common", "Tenant_A"]
    res = _run('.auth.partition["Tenant_A"].description', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["App tenant"]


def test_auth_partition_default_route_domain_is_pathref_into_net():
    """``default-route-domain`` carries the numeric id; we project it
    as a PathRef into ``net route-domain`` so the chain
    ``.auth.partition[].default-route-domain.id`` resolves when a
    route-domain object is present with that id as the basename."""
    res = _run('.auth.partition["Tenant_A"]."default-route-domain"', _AUTH_BUNDLE_CONF)
    [ref] = res.values_per_file["mem://1"]
    assert ref.full_path == "0"


def test_auth_user_projects_shell_partition_and_partition_access():
    base = '.auth.user["admin"]'
    assert _run(f"{base}.shell", _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == ["bash"]
    res = _run(f"{base}.partition", _AUTH_BUNDLE_CONF)
    [ref] = res.values_per_file["mem://1"]
    assert ref.full_path == "Common"
    res = _run(f'{base}."partition-access"', _AUTH_BUNDLE_CONF)
    [pa] = res.values_per_file["mem://1"]
    assert sorted(pa) == ["Tenant_A", "all-partitions"]


def test_auth_password_singleton_projects_minimum_length():
    res = _run(".auth.password[]", _AUTH_BUNDLE_CONF)
    [obj] = res.values_per_file["mem://1"]
    assert obj.fields["minimum-length"] == "6"


def test_auth_password_policy_singleton_projects_policy_fields():
    base = ".auth.password-policy[]"
    res = _run(f'{base} | ."minimum-length"', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["8"]
    res = _run(f'{base} | ."max-login-failures"', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["5"]


def test_auth_source_singleton_projects_type_and_fallback():
    res = _run(".auth.source[].type", _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["radius"]
    res = _run(".auth.source[].fallback", _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["true"]


def test_auth_remote_role_singleton_surfaces_role_info_keys():
    res = _run('.auth.remote-role[]."role-info"', _AUTH_BUNDLE_CONF)
    [keys] = res.values_per_file["mem://1"]
    assert sorted(keys) == ["TEAM1", "TEAM2"]


def test_auth_remote_user_singleton_projects_default_partition_and_role():
    res = _run('.auth.remote-user[]."default-partition"', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["all"]
    res = _run('.auth.remote-user[]."default-role"', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == ["resource-admin"]


def test_auth_login_failures_singleton_is_present():
    res = _run('.auth.login-failures[]."full-path"', _AUTH_BUNDLE_CONF)
    assert res.values_per_file["mem://1"] == [""]


def test_auth_ldap_projects_bind_dn_servers_and_ssl():
    base = '.auth.ldap["/Common/system-auth"]'
    assert _run(f'{base}."bind-dn"', _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == [
        "cn=admin,dc=example,dc=test"
    ]
    [servers] = _run(f"{base}.servers", _AUTH_BUNDLE_CONF).values_per_file["mem://1"]
    assert list(servers) == ["ldap1.example.test", "ldap2.example.test"]
    assert _run(f"{base}.ssl", _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == ["enabled"]


def test_auth_radius_servers_walk_through_pathref_chain():
    """``auth radius.servers[]`` are PathRefs into ``auth radius-server``;
    chaining ``.port`` walks server -> port across both entries.
    """
    res = _run(
        '.auth.radius["/Common/system-auth-radius"].servers[].port',
        _AUTH_BUNDLE_CONF,
    )
    assert sorted(res.values_per_file["mem://1"]) == ["1811", "1888"]


def test_auth_radius_server_projects_server_and_secret():
    base = '.auth.radius-server["/Common/system_auth_name1"]'
    assert _run(f"{base}.server", _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == ["192.0.2.4"]
    assert _run(f"{base}.secret", _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == ["obscured1"]


def test_auth_tacacs_projects_servers_and_protocol():
    base = '.auth.tacacs["/Common/system-auth-tac"]'
    [servers] = _run(f"{base}.servers", _AUTH_BUNDLE_CONF).values_per_file["mem://1"]
    assert list(servers) == ["192.0.2.5", "198.51.100.5"]
    assert _run(f"{base}.protocol", _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == ["ip"]


def test_auth_cert_ldap_projects_bind_dn_and_servers():
    base = '.auth.cert-ldap["/Common/cert-ldap-1"]'
    assert _run(f'{base}."bind-dn"', _AUTH_BUNDLE_CONF).values_per_file["mem://1"] == [
        "cn=cert-reader"
    ]
    [servers] = _run(f"{base}.servers", _AUTH_BUNDLE_CONF).values_per_file["mem://1"]
    assert list(servers) == ["ldap1.example.test"]


def test_auth_apm_auth_profile_is_pathref_into_access_policy():
    res = _run('.auth.apm-auth["/Common/apm-auth-1"].profile', _AUTH_BUNDLE_CONF)
    [ref] = res.values_per_file["mem://1"]
    assert ref.full_path == "/Common/access_policy_1"


# ---------------------------------------------------------------------------
# Bundle 9 — AFM ``security firewall.*`` core (13 kinds: policy, address-list,
# global-rules / management-ip-rules / global-fqdn-policy / on-demand-* /
# uuid-default-autogenerate / config-change-log singletons, schedule,
# user-list, user-domain, port-misuse-policy).
# ---------------------------------------------------------------------------

_FW9_CONF = """security firewall policy /Common/policy_main {
    description "main firewall policy"
    rules {
        binding_a { rule-list /Common/rl_a }
        binding_b { rule-list /Common/rl_b }
    }
}
security firewall rule-list /Common/rl_a { }
security firewall rule-list /Common/rl_b { }
security firewall address-list /Common/al_internal {
    description "internal nets"
    addresses {
        192.0.2.0/24 { }
        198.51.100.5 { }
    }
}
security firewall address-list /Common/al_parent {
    description "parent"
    address-lists {
        /Common/al_internal { }
    }
}
security firewall global-rules {
    description "global wrapper"
    rules { gfoo { } gbar { } }
    enforced-policy /Common/policy_main
}
security firewall management-ip-rules {
    description "mgmt"
    rules { mgmt_allow { } }
}
security firewall schedule /Common/sched_biz {
    description "business hours"
    daily-hour-start 09:00
    daily-hour-end 17:00
    days-of-week { mon tue wed thu fri }
}
security firewall user-list /Common/ul_users {
    users { alice { } bob { } }
}
security firewall user-domain /Common/ud_corp {
    domain corp.example.test
}
security firewall global-fqdn-policy {
    context global
}
security firewall port-misuse-policy /Common/pmp1 {
    default-log no
}
security firewall on-demand-compilation { }
security firewall on-demand-rule-deploy { }
security firewall uuid-default-autogenerate {
    auto-generate-uuid enabled
}
security firewall config-change-log {
    log-publisher /Common/local-db-publisher
}
"""


def test_security_firewall_policy_projects_rule_bindings_and_rule_lists():
    base = '.security.firewall-policy["/Common/policy_main"]'
    [names] = _run(f"{base}.rules", _FW9_CONF).values_per_file["mem://1"]
    assert sorted(names) == ["binding_a", "binding_b"]
    [refs] = _run(f'{base}."rule-lists"', _FW9_CONF).values_per_file["mem://1"]
    assert sorted(r.full_path for r in refs) == ["/Common/rl_a", "/Common/rl_b"]


def test_security_firewall_policy_rule_lists_resolve_through_pathref():
    """The ``rule-lists`` PathRefs deref into the existing
    ``security firewall rule-list`` projection.  Chaining ``.name`` from
    the deref'd ObjectRef yields the rule-list basenames.
    """
    res = _run(
        '.security.firewall-policy["/Common/policy_main"]."rule-lists"[].name',
        _FW9_CONF,
    )
    assert sorted(res.values_per_file["mem://1"]) == ["rl_a", "rl_b"]


def test_security_firewall_address_list_projects_addresses_and_children():
    [addrs] = _run(
        '.security.firewall-address-list["/Common/al_internal"].addresses',
        _FW9_CONF,
    ).values_per_file["mem://1"]
    assert sorted(addrs) == ["192.0.2.0/24", "198.51.100.5"]
    [children] = _run(
        '.security.firewall-address-list["/Common/al_parent"]."address-lists"',
        _FW9_CONF,
    ).values_per_file["mem://1"]
    assert [c.full_path for c in children] == ["/Common/al_internal"]


def test_security_firewall_global_rules_singleton_projects_rules_and_enforced_policy():
    [names] = _run(".security.firewall-global-rules[].rules", _FW9_CONF).values_per_file["mem://1"]
    assert sorted(names) == ["gbar", "gfoo"]
    [policy] = _run(
        '.security.firewall-global-rules[]."enforced-policy"', _FW9_CONF
    ).values_per_file["mem://1"]
    assert policy.full_path == "/Common/policy_main"


def test_security_firewall_management_ip_rules_singleton_surfaces_rule_names():
    [names] = _run(".security.firewall-management-ip-rules[].rules", _FW9_CONF).values_per_file[
        "mem://1"
    ]
    assert list(names) == ["mgmt_allow"]


def test_security_firewall_schedule_projects_days_and_hours():
    base = '.security.firewall-schedule["/Common/sched_biz"]'
    assert _run(f'{base}."daily-hour-start"', _FW9_CONF).values_per_file["mem://1"] == ["09:00"]
    assert _run(f'{base}."daily-hour-end"', _FW9_CONF).values_per_file["mem://1"] == ["17:00"]
    [days] = _run(f'{base}."days-of-week"', _FW9_CONF).values_per_file["mem://1"]
    assert sorted(days) == ["fri", "mon", "thu", "tue", "wed"]


def test_security_firewall_user_list_projects_user_keys():
    [users] = _run(
        '.security.firewall-user-list["/Common/ul_users"].users', _FW9_CONF
    ).values_per_file["mem://1"]
    assert sorted(users) == ["alice", "bob"]


def test_security_firewall_user_domain_projects_domain_scalar():
    assert _run(
        '.security.firewall-user-domain["/Common/ud_corp"].domain', _FW9_CONF
    ).values_per_file["mem://1"] == ["corp.example.test"]


def test_security_firewall_global_fqdn_policy_singleton_projects_context():
    assert _run(".security.firewall-global-fqdn-policy[].context", _FW9_CONF).values_per_file[
        "mem://1"
    ] == ["global"]


def test_security_firewall_port_misuse_policy_projects_default_log():
    assert _run(
        '.security.firewall-port-misuse-policy["/Common/pmp1"]."default-log"', _FW9_CONF
    ).values_per_file["mem://1"] == ["no"]


def test_security_firewall_on_demand_compilation_singleton_is_present():
    res = _run('.security.firewall-on-demand-compilation[]."full-path"', _FW9_CONF)
    assert res.values_per_file["mem://1"] == [""]


def test_security_firewall_on_demand_rule_deploy_singleton_is_present():
    res = _run('.security.firewall-on-demand-rule-deploy[]."full-path"', _FW9_CONF)
    assert res.values_per_file["mem://1"] == [""]


def test_security_firewall_uuid_default_autogenerate_singleton_projects_setting():
    res = _run('.security.firewall-uuid-default-autogenerate[]."auto-generate-uuid"', _FW9_CONF)
    assert res.values_per_file["mem://1"] == ["enabled"]


def test_security_firewall_config_change_log_singleton_projects_publisher():
    res = _run('.security.firewall-config-change-log[]."log-publisher"', _FW9_CONF)
    assert res.values_per_file["mem://1"] == ["/Common/local-db-publisher"]


# ---------------------------------------------------------------------------
# Bundle 10a — high-value security.* outside firewall.*:
# NAT policy + translations, log profile, DoS profile, IP intelligence
# feed-list / global-policy, zone / protected zone, packet-filter,
# SSH profile, HTTP profile, bot-defense profile (14 kinds).
# ---------------------------------------------------------------------------

_SEC10A_CONF = """security firewall rule-list /Common/rl_a { }
security nat policy /Common/nat_main {
    description "main nat policy"
    rules {
        binding_a { rule-list /Common/rl_a }
    }
}
security nat source-translation /Common/sn_pool {
    type dynamic-pat
    addresses { 192.0.2.10 }
    ports { 1024-65535 }
    traffic-group /Common/traffic-group-1
    egress-interfaces-disabled
}
security nat destination-translation /Common/dn_pool {
    type static-pat
    addresses { 192.0.2.20 }
    ports { 8443 }
}
security log profile /Common/lp_default {
    description "default log profile"
}
security dos profile /Common/dp_default {
    description "default dos"
    app-service none
    threshold-sensitivity low
}
security ip-intelligence feed-list /Common/feeds_main {
    description "main feeds"
    feeds {
        spamhaus_drop { }
        torexit { }
    }
}
security ip-intelligence global-policy {
    description "global"
    log-publisher /Common/local-db-publisher
}
net vlan /Common/vlan_inside { tag 100 }
security zone /Common/zone_inside {
    description "inside zone"
    vlans { /Common/vlan_inside }
}
security protected zone /Common/pz_critical {
    description "critical"
}
security packet-filter policy /Common/pf_main {
    rules { allow_all { } deny_all { } }
}
security packet-filter default-rules {
    action accept
}
security ssh profile /Common/ssh_p1 {
    timeout 60
}
security http profile /Common/http_p1 {
    description "http sec"
    defaults_from /Common/http_default
}
security bot-defense profile /Common/bot_p1 {
    description "bot"
    template relaxed
}
cm traffic-group /Common/traffic-group-1 { unit-id 1 }
"""


def test_security_nat_policy_projects_rule_bindings_and_pathrefs():
    base = '.security.nat-policy["/Common/nat_main"]'
    [names] = _run(f"{base}.rules", _SEC10A_CONF).values_per_file["mem://1"]
    assert list(names) == ["binding_a"]
    [refs] = _run(f'{base}."rule-lists"', _SEC10A_CONF).values_per_file["mem://1"]
    assert [r.full_path for r in refs] == ["/Common/rl_a"]


def test_security_nat_source_translation_projects_addresses_and_traffic_group():
    base = '.security.nat-source-translation["/Common/sn_pool"]'
    assert _run(f"{base}.type", _SEC10A_CONF).values_per_file["mem://1"] == ["dynamic-pat"]
    [addrs] = _run(f"{base}.addresses", _SEC10A_CONF).values_per_file["mem://1"]
    assert list(addrs) == ["192.0.2.10"]
    [tg] = _run(f'{base}."traffic-group"', _SEC10A_CONF).values_per_file["mem://1"]
    assert tg.full_path == "/Common/traffic-group-1"
    assert _run(f'{base}."egress-interfaces-disabled"', _SEC10A_CONF).values_per_file[
        "mem://1"
    ] == [True]


def test_security_nat_destination_translation_projects_addresses_and_ports():
    base = '.security.nat-destination-translation["/Common/dn_pool"]'
    [ports] = _run(f"{base}.ports", _SEC10A_CONF).values_per_file["mem://1"]
    assert list(ports) == ["8443"]
    assert _run(f"{base}.type", _SEC10A_CONF).values_per_file["mem://1"] == ["static-pat"]


def test_security_log_profile_projects_description():
    res = _run(".security.log-profile[].description", _SEC10A_CONF)
    assert res.values_per_file["mem://1"] == ["default log profile"]


def test_security_dos_profile_projects_threshold_sensitivity():
    res = _run('.security.dos-profile[]."threshold-sensitivity"', _SEC10A_CONF)
    assert res.values_per_file["mem://1"] == ["low"]


def test_security_ip_intelligence_feed_list_projects_feed_names():
    [feeds] = _run(".security.ip-intelligence-feed-list[].feeds", _SEC10A_CONF).values_per_file[
        "mem://1"
    ]
    assert sorted(feeds) == ["spamhaus_drop", "torexit"]


def test_security_ip_intelligence_global_policy_singleton_projects_log_publisher():
    res = _run('.security.ip-intelligence-global-policy[]."log-publisher"', _SEC10A_CONF)
    assert res.values_per_file["mem://1"] == ["/Common/local-db-publisher"]


def test_security_zone_vlans_walks_into_net_vlan():
    """``security zone.vlans[]`` is a PathRef list into ``net vlan``;
    chaining ``.tag`` pulls the VLAN tag.
    """
    res = _run(".security.zone[].vlans[].tag", _SEC10A_CONF)
    assert res.values_per_file["mem://1"] == [100]


def test_security_protected_zone_projects_description():
    assert _run(".security.protected-zone[].description", _SEC10A_CONF).values_per_file[
        "mem://1"
    ] == ["critical"]


def test_security_packet_filter_policy_surfaces_rule_keys():
    [rules] = _run(
        '.security.packet-filter-policy["/Common/pf_main"].rules', _SEC10A_CONF
    ).values_per_file["mem://1"]
    assert sorted(rules) == ["allow_all", "deny_all"]


def test_security_packet_filter_default_rules_singleton_projects_action():
    assert _run(".security.packet-filter-default-rules[].action", _SEC10A_CONF).values_per_file[
        "mem://1"
    ] == ["accept"]


def test_security_ssh_profile_projects_timeout():
    assert _run('.security.ssh-profile["/Common/ssh_p1"].timeout', _SEC10A_CONF).values_per_file[
        "mem://1"
    ] == ["60"]


def test_security_http_profile_projects_description():
    assert _run(
        '.security.http-profile["/Common/http_p1"].description', _SEC10A_CONF
    ).values_per_file["mem://1"] == ["http sec"]


def test_security_bot_defense_profile_projects_template():
    assert _run(
        '.security.bot-defense-profile["/Common/bot_p1"].template', _SEC10A_CONF
    ).values_per_file["mem://1"] == ["relaxed"]


# ---------------------------------------------------------------------------
# Bundle 10b — 37 minimal-shape security.* kinds (analytics, anti-fraud,
# blacklist-publisher, bot-defense sig/cat, cloud-services, datasync,
# debug, device-context, dos sub-kinds, flowspec, ip-intelligence
# blacklist-category, protocol-inspection sub-kinds, scrubber, ssh ciphers).
# All share BigipSecurityMinimalObject + _SECURITY_MINIMAL_FIELDS.
# ---------------------------------------------------------------------------

_SEC10B_CONF = """security analytics settings { description analytics }
security anti-fraud profile /Common/af1 { description "anti-fraud" }
security anti-fraud signatures-update /Common/afsu1 { description "afsu" }
security blacklist-publisher category /Common/blc1 { description "bl cat" }
security blacklist-publisher profile /Common/blp1 { description "bl prof" }
security bot-defense signature /Common/botsig1 { description "bot sig" }
security bot-defense signature-category /Common/botsigcat1 { description "bot sig cat" }
security cloud-services connector /Common/cs1 { description "cs conn" }
security datasync background-tasks { description "bg tasks" }
security datasync global-profile /Common/dsgp1 { description "global ds" }
security datasync local-profile /Common/dslp1 { description "local ds" }
security debug drop-redirect-stats { description "drop stats" }
security debug matcher { description matcher }
security debug register { description register }
security device device-context /Common/devctx1 { description "dev ctx" }
security dos autodos-file-object /Common/adfo1 { description "adfo" }
security dos behavioral-signature /Common/bs1 { description "bs" }
security dos bot-signature /Common/dosbotsig1 { description "dos bot sig" }
security dos bot-signature-category /Common/dosbotcat1 { description "dos bot cat" }
security dos device-config { description "dev cfg" }
security dos dns-nxdomain-stat { description "nxdomain" }
security dos dos-signature /Common/dossig1 { description "dos sig" }
security dos dynamic-signatures { description "dyn sigs" }
security dos ip-uncommon-protolist /Common/iup1 { description "iup" }
security dos l4bdos-file-object /Common/l4bdos1 { description "l4bdos" }
security dos network-whitelist /Common/nwl1 { description "nwl" }
security dos stress-stats { description "stress" }
security dos udp-portlist /Common/dosupl1 { description "udp" }
security dos virtual /Common/dosvs1 { description "dos virtual" }
security flowspec-route-injector profile /Common/fri1 { description "fri" }
security ip-intelligence blacklist-category /Common/ipbc1 { description "ip bc" }
security protocol-inspection common-config { description "pi common" }
security protocol-inspection learning-stats { description "pi learn" }
security protocol-inspection profile /Common/pip1 { description "pi profile" }
security protocol-inspection signature /Common/pisig1 { description "pi sig" }
security scrubber profile /Common/scr1 { description "scrubber" }
security ssh ciphers /Common/ciphers1 { description "ssh ciphers" }
"""


def test_security_minimal_each_kind_has_an_entry():
    """37 minimal kinds — confirm every dispatch label routes to its
    container and that the kind label round-trips through the
    projection.
    """
    cases: list[tuple[str, str]] = [
        ("analytics-settings", "security analytics settings"),
        ("anti-fraud-profile", "security anti-fraud profile"),
        ("anti-fraud-signatures-update", "security anti-fraud signatures-update"),
        ("blacklist-publisher-category", "security blacklist-publisher category"),
        ("blacklist-publisher-profile", "security blacklist-publisher profile"),
        ("bot-defense-signature", "security bot-defense signature"),
        ("bot-defense-signature-category", "security bot-defense signature-category"),
        ("cloud-services-connector", "security cloud-services connector"),
        ("datasync-background-tasks", "security datasync background-tasks"),
        ("datasync-global-profile", "security datasync global-profile"),
        ("datasync-local-profile", "security datasync local-profile"),
        ("debug-drop-redirect-stats", "security debug drop-redirect-stats"),
        ("debug-matcher", "security debug matcher"),
        ("debug-register", "security debug register"),
        ("device-device-context", "security device device-context"),
        ("dos-autodos-file-object", "security dos autodos-file-object"),
        ("dos-behavioral-signature", "security dos behavioral-signature"),
        ("dos-bot-signature", "security dos bot-signature"),
        ("dos-bot-signature-category", "security dos bot-signature-category"),
        ("dos-device-config", "security dos device-config"),
        ("dos-dns-nxdomain-stat", "security dos dns-nxdomain-stat"),
        ("dos-dos-signature", "security dos dos-signature"),
        ("dos-dynamic-signatures", "security dos dynamic-signatures"),
        ("dos-ip-uncommon-protolist", "security dos ip-uncommon-protolist"),
        ("dos-l4bdos-file-object", "security dos l4bdos-file-object"),
        ("dos-network-whitelist", "security dos network-whitelist"),
        ("dos-stress-stats", "security dos stress-stats"),
        ("dos-udp-portlist", "security dos udp-portlist"),
        ("dos-virtual", "security dos virtual"),
        ("flowspec-route-injector-profile", "security flowspec-route-injector profile"),
        ("ip-intelligence-blacklist-category", "security ip-intelligence blacklist-category"),
        ("protocol-inspection-common-config", "security protocol-inspection common-config"),
        ("protocol-inspection-learning-stats", "security protocol-inspection learning-stats"),
        ("protocol-inspection-profile", "security protocol-inspection profile"),
        ("protocol-inspection-signature", "security protocol-inspection signature"),
        ("scrubber-profile", "security scrubber profile"),
        ("ssh-ciphers", "security ssh ciphers"),
    ]
    for label, kind_str in cases:
        res = _run(f".security.{label}[].kind", _SEC10B_CONF)
        kinds = res.values_per_file["mem://1"]
        assert kind_str in kinds, f"{label}: expected kind={kind_str!r}, got {kinds!r}"


def test_security_minimal_singletons_resolve_via_empty_key():
    """``debug *``, ``dos device-config``, ``dos dns-nxdomain-stat``,
    ``dos dynamic-signatures``, ``dos stress-stats``, ``datasync
    background-tasks``, ``protocol-inspection common-config``,
    ``protocol-inspection learning-stats``, and ``analytics settings``
    are 3-token singletons — they land under the empty-string key.
    """
    for label, expected_desc in [
        ("debug-matcher", "matcher"),
        ("debug-register", "register"),
        ("debug-drop-redirect-stats", "drop stats"),
        ("dos-device-config", "dev cfg"),
        ("dos-dns-nxdomain-stat", "nxdomain"),
        ("dos-dynamic-signatures", "dyn sigs"),
        ("dos-stress-stats", "stress"),
        ("datasync-background-tasks", "bg tasks"),
        ("protocol-inspection-common-config", "pi common"),
        ("protocol-inspection-learning-stats", "pi learn"),
        ("analytics-settings", "analytics"),
    ]:
        res = _run(f'.security.{label}[""]."description"', _SEC10B_CONF)
        assert res.values_per_file["mem://1"] == [expected_desc], (
            f"{label}: expected {expected_desc!r}, got {res.values_per_file['mem://1']!r}"
        )


def test_security_minimal_path_keyed_kinds_resolve_by_full_path():
    """Path-keyed minimal kinds (``dos virtual /Common/X`` etc.)
    project the basename and full-path correctly.
    """
    for label, full_path, name in [
        ("dos-virtual", "/Common/dosvs1", "dosvs1"),
        ("dos-dos-signature", "/Common/dossig1", "dossig1"),
        ("scrubber-profile", "/Common/scr1", "scr1"),
        ("protocol-inspection-profile", "/Common/pip1", "pip1"),
        ("flowspec-route-injector-profile", "/Common/fri1", "fri1"),
        ("anti-fraud-profile", "/Common/af1", "af1"),
        ("blacklist-publisher-profile", "/Common/blp1", "blp1"),
        ("cloud-services-connector", "/Common/cs1", "cs1"),
        ("ssh-ciphers", "/Common/ciphers1", "ciphers1"),
    ]:
        res = _run(f'.security.{label}["{full_path}"].name', _SEC10B_CONF)
        assert res.values_per_file["mem://1"] == [name], (
            f"{label}: got {res.values_per_file['mem://1']!r}"
        )


# ---------------------------------------------------------------------------
# Bundle 11 — gtm monitor.*  (31 per-protocol variants merged into one
# container keyed by full-path; ``type`` distinguishes them).
# ---------------------------------------------------------------------------

_GTM_MON_CONF = """gtm monitor bigip /Common/gtm_bigip {
    description "gtm bigip monitor"
    interval 30
    timeout 90
}
gtm monitor bigip-link /Common/gtm_bigip_link { interval 30 }
gtm monitor http /Common/gtm_http {
    interval 30
    send "GET / HTTP/1.0\\r\\n\\r\\n"
    recv "200 OK"
}
gtm monitor https /Common/gtm_https {
    defaults-from /Common/gtm_https_parent
    interval 30
}
gtm monitor https /Common/gtm_https_parent { interval 30 }
gtm monitor tcp /Common/gtm_tcp { interval 30 }
gtm monitor tcp-half-open /Common/gtm_tcp_ho { interval 30 }
gtm monitor udp /Common/gtm_udp { interval 30 }
gtm monitor ldap /Common/gtm_ldap { interval 30 }
gtm monitor sip /Common/gtm_sip { interval 30 }
gtm monitor snmp /Common/gtm_snmp { interval 30 }
gtm monitor snmp-link /Common/gtm_snmp_link { interval 30 }
gtm monitor radius /Common/gtm_radius { interval 30 }
gtm monitor radius-accounting /Common/gtm_radius_acc { interval 30 }
gtm monitor real-server /Common/gtm_rs { interval 30 }
gtm monitor wmi /Common/gtm_wmi { interval 30 }
gtm monitor wap /Common/gtm_wap { interval 30 }
gtm monitor scripted /Common/gtm_scripted { interval 30 }
gtm monitor mssql /Common/gtm_mssql { interval 30 }
gtm monitor mysql /Common/gtm_mysql { interval 30 }
gtm monitor oracle /Common/gtm_oracle { interval 30 }
gtm monitor postgresql /Common/gtm_psql { interval 30 }
gtm monitor smtp /Common/gtm_smtp { interval 30 }
gtm monitor imap /Common/gtm_imap { interval 30 }
gtm monitor pop3 /Common/gtm_pop3 { interval 30 }
gtm monitor nntp /Common/gtm_nntp { interval 30 }
gtm monitor ftp /Common/gtm_ftp { interval 30 }
gtm monitor soap /Common/gtm_soap { interval 30 }
gtm monitor firepass /Common/gtm_firepass { interval 30 }
gtm monitor gateway-icmp /Common/gtm_gw_icmp { interval 30 }
gtm monitor external /Common/gtm_ext { interval 30 }
gtm monitor gtp /Common/gtm_gtp { interval 30 }
ltm monitor http /Common/ltm_http { interval 5 }
"""


def test_gtm_monitor_merges_all_protocol_variants():
    """All 31 ``gtm monitor <type>`` variants land in one container
    keyed by full-path; the ``type`` field distinguishes them.
    """
    res = _run(".gtm.monitor[].type", _GTM_MON_CONF)
    types = res.values_per_file["mem://1"]
    expected = {
        "bigip",
        "bigip-link",
        "http",
        "https",
        "tcp",
        "tcp-half-open",
        "udp",
        "ldap",
        "sip",
        "snmp",
        "snmp-link",
        "radius",
        "radius-accounting",
        "real-server",
        "wmi",
        "wap",
        "scripted",
        "mssql",
        "mysql",
        "oracle",
        "postgresql",
        "smtp",
        "imap",
        "pop3",
        "nntp",
        "ftp",
        "soap",
        "firepass",
        "gateway-icmp",
        "external",
        "gtp",
    }
    assert set(types) == expected


def test_gtm_monitor_kept_separate_from_ltm_monitor():
    """``ltm monitor http /Common/X`` and ``gtm monitor http /Common/X``
    used to collide in ``config.monitors``.  GTM monitors now live in
    ``config.gtm_monitors`` so the LTM container is uncontaminated.
    """
    res = _run('.ltm.monitor[]."full-path"', _GTM_MON_CONF)
    assert res.values_per_file["mem://1"] == ["/Common/ltm_http"]
    res = _run('.gtm.monitor[]."full-path"', _GTM_MON_CONF)
    # 31 ``gtm monitor`` variants + an extra ``https`` parent for the
    # defaults-from chain test = 32.
    assert len(res.values_per_file["mem://1"]) == 32
    assert all(p.startswith("/Common/gtm_") for p in res.values_per_file["mem://1"])


def test_gtm_monitor_send_and_recv_round_trip():
    """The send/recv strings carry escape sequences verbatim — the
    parser must not corrupt them.
    """
    base = '.gtm.monitor["/Common/gtm_http"]'
    assert _run(f"{base}.send", _GTM_MON_CONF).values_per_file["mem://1"] == [
        "GET / HTTP/1.0\\r\\n\\r\\n"
    ]
    assert _run(f"{base}.recv", _GTM_MON_CONF).values_per_file["mem://1"] == ["200 OK"]


def test_gtm_monitor_defaults_from_resolves_within_gtm_namespace():
    """``defaults-from`` is a PathRef into ``gtm monitor`` (not
    ``ltm monitor``) so chained access lands in the GTM parent
    monitor.  Same-path LTM monitor must not bleed through.
    """
    res = _run(
        '.gtm.monitor["/Common/gtm_https"]."defaults-from"."full-path"',
        _GTM_MON_CONF,
    )
    assert res.values_per_file["mem://1"] == ["/Common/gtm_https_parent"]


# ---------------------------------------------------------------------------
# Bundle 12 — gtm listeners / link / topology / distributed-app /
# global-settings singletons (10 kinds total).
# ---------------------------------------------------------------------------

_GTM12_CONF = """gtm datacenter /Common/dc_east { location east }
gtm prober-pool /Common/pp_main {
    description "main"
}
gtm pool a /Common/dns_pool {
    ttl 60
}
gtm wideip a /Common/widget.example.test { }
gtm wideip a /Common/widget2.example.test { }
gtm listener /Common/dns_listener_1 {
    address 192.0.2.53
    port 53
    ip-protocol udp
    pool /Common/dns_pool
    profiles { /Common/dns }
    rules { /Common/dns_log }
    vlans { /Common/vlan_dns }
}
gtm listener-doh-proxy /Common/doh_proxy_1 {
    address 192.0.2.443
    port 443
    pool /Common/dns_pool
}
gtm listener-doh-server /Common/doh_server_1 {
    address 192.0.2.444
    port 443
    pool /Common/dns_pool
}
gtm link /Common/uplink_a {
    description "primary uplink"
    datacenter /Common/dc_east
    monitor /Common/gtm_bigip
    prober-pool /Common/pp_main
    weight 10
}
gtm distributed-app /Common/app_widget {
    description "widget app"
    persist-cidr 32
    wide-ips { /Common/widget.example.test /Common/widget2.example.test }
}
gtm topology ldns: subnet 10.15.1.1/32 server: subnet 10.16.1.1/32 {
    order 1
}
gtm topology ldns: country DK server: not geoip-isp some-isp {
    description "country/isp rule"
    order 2
    score 5
}
gtm global-settings general {
    auto-discovery enabled
    synchronization yes
    synchronization-group-name sync_a
}
gtm global-settings load-balancing {
    topology-longest-match yes
}
gtm global-settings metrics {
    metrics-collection-protocols { icmp tcp }
}
gtm global-settings metrics-exclusions {
    addresses none
}
"""


def test_gtm_listener_projects_address_port_and_profiles():
    base = '.gtm.listener["/Common/dns_listener_1"]'
    assert _run(f"{base}.address", _GTM12_CONF).values_per_file["mem://1"] == ["192.0.2.53"]
    assert _run(f"{base}.port", _GTM12_CONF).values_per_file["mem://1"] == ["53"]
    [profiles] = _run(f"{base}.profiles", _GTM12_CONF).values_per_file["mem://1"]
    assert [p.full_path for p in profiles] == ["/Common/dns"]


def test_gtm_listener_pool_pathref_walks_into_gtm_pool():
    """``listener.pool`` is a PathRef into ``gtm pool``; chaining
    ``.ttl`` walks into the target pool.
    """
    res = _run('.gtm.listener["/Common/dns_listener_1"].pool.ttl', _GTM12_CONF)
    assert res.values_per_file["mem://1"] == ["60"]


def test_gtm_listener_doh_proxy_and_server_share_shape():
    proxy = _run(
        '.gtm.listener-doh-proxy["/Common/doh_proxy_1"].port', _GTM12_CONF
    ).values_per_file["mem://1"]
    server = _run(
        '.gtm.listener-doh-server["/Common/doh_server_1"].port', _GTM12_CONF
    ).values_per_file["mem://1"]
    assert proxy == ["443"]
    assert server == ["443"]


def test_gtm_link_datacenter_walks_through_pathref():
    """``link.datacenter`` is a PathRef into ``gtm datacenter``."""
    res = _run('.gtm.link["/Common/uplink_a"].datacenter.location', _GTM12_CONF)
    assert res.values_per_file["mem://1"] == ["east"]


def test_gtm_distributed_app_wide_ips_resolve_through_pathref():
    """``distributed-app.wide-ips[]`` is a PathRef list into ``gtm wideip``."""
    res = _run(
        '.gtm.distributed-app["/Common/app_widget"]."wide-ips"[]."full-path"',
        _GTM12_CONF,
    )
    assert sorted(res.values_per_file["mem://1"]) == [
        "/Common/widget.example.test",
        "/Common/widget2.example.test",
    ]


def test_gtm_topology_uses_multi_token_condition_as_identifier():
    """``gtm topology`` carries a multi-token condition instead of a
    full-path.  The parser captures the entire post-``topology`` header
    as the identifier and stores under that key.
    """
    res = _run('.gtm.topology[]."full-path"', _GTM12_CONF)
    keys = res.values_per_file["mem://1"]
    assert "ldns: subnet 10.15.1.1/32 server: subnet 10.16.1.1/32" in keys
    assert "ldns: country DK server: not geoip-isp some-isp" in keys


def test_gtm_topology_projects_order_and_score():
    res = _run(".gtm.topology[].order", _GTM12_CONF)
    assert sorted(res.values_per_file["mem://1"]) == ["1", "2"]


def test_gtm_global_settings_general_singleton_projects_sync():
    res = _run(".gtm.global-settings-general[].synchronization", _GTM12_CONF)
    assert res.values_per_file["mem://1"] == ["yes"]


def test_gtm_global_settings_load_balancing_singleton_projects_topology_longest_match():
    res = _run('.gtm.global-settings-load-balancing[]."topology-longest-match"', _GTM12_CONF)
    assert res.values_per_file["mem://1"] == ["yes"]


def test_gtm_global_settings_metrics_singleton_surfaces_protocol_list():
    [protos] = _run(
        '.gtm.global-settings-metrics[]."metrics-collection-protocols"', _GTM12_CONF
    ).values_per_file["mem://1"]
    assert sorted(protos) == ["icmp", "tcp"]


def test_gtm_global_settings_metrics_exclusions_singleton_projects_addresses():
    assert _run(".gtm.global-settings-metrics-exclusions[].addresses", _GTM12_CONF).values_per_file[
        "mem://1"
    ] == ["none"]


# ---------------------------------------------------------------------------
# Bundle 13 — ltm.* cross-cutting infra (cipher group/rule, nat, snat,
# snat-translation, policy-strategy, traffic-class,
# traffic-matching-criteria, ifile, eviction-policy — 10 kinds).
# ---------------------------------------------------------------------------

_LTM13_CONF = """ltm cipher rule /Common/cr_strong { cipher ECDHE:!RC4 }
ltm cipher rule /Common/cr_legacy { cipher AES128-SHA }
ltm cipher group /Common/cg_main {
    description "main cipher group"
    allow { /Common/cr_strong /Common/cr_legacy }
    exclude { /Common/cr_legacy }
    ordering strength
}
ltm nat /Common/nat_external {
    translation-address 192.0.2.50
    originating-address 10.0.0.5
    arp enabled
    vlans { /Common/vlan_inside }
    vlans-disabled
}
ltm snatpool /Common/sp_main { members { /Common/snat_xlate_1 } }
ltm snat /Common/snat_app {
    description "app snat"
    origins { 10.0.0.0/24 192.0.2.0/24 }
    snatpool /Common/sp_main
    vlans { /Common/vlan_inside }
    mirror enabled
}
ltm snat-translation /Common/snat_xlate_1 {
    address 192.0.2.55
    inherited-traffic-group true
    traffic-group /Common/traffic-group-1
    connection-limit 0
}
ltm policy-strategy /Common/ps_best_match {
    strategy best-match
    operands {
        0 { http-uri path }
        1 { http-host host }
    }
}
ltm traffic-class /Common/tc_1 {
    classification web
    match-method exact
}
security firewall address-list /Common/addrl_1 { addresses { 10.0.0.0/24 { } } }
security firewall port-list /Common/portl_1 { ports { 443 { } } }
ltm traffic-matching-criteria /Common/tmc_1 {
    destination-address-list /Common/addrl_1
    destination-port-list /Common/portl_1
    source-address-inline 0.0.0.0
    protocol tcp
}
ltm ifile /Common/ifile_1 {
    file-name /Common/ifile_data
}
ltm eviction-policy /Common/ep_1 {
    high-water-mark 95
    low-water-mark 85
    slow-flow-throttle enabled
}
"""


def test_ltm_cipher_group_allow_walks_into_cipher_rule():
    """``cipher group.allow[]`` is a PathRef list into
    ``ltm cipher rule``; chaining ``.cipher`` walks to the cipher
    string on each referenced rule.
    """
    res = _run('.ltm.cipher-group["/Common/cg_main"].allow[].cipher', _LTM13_CONF)
    assert sorted(res.values_per_file["mem://1"]) == ["AES128-SHA", "ECDHE:!RC4"]


def test_ltm_cipher_group_ordering_and_exclude():
    base = '.ltm.cipher-group["/Common/cg_main"]'
    assert _run(f"{base}.ordering", _LTM13_CONF).values_per_file["mem://1"] == ["strength"]
    [excl] = _run(f"{base}.exclude", _LTM13_CONF).values_per_file["mem://1"]
    assert [r.full_path for r in excl] == ["/Common/cr_legacy"]


def test_ltm_cipher_rule_projects_cipher():
    res = _run('.ltm.cipher-rule["/Common/cr_strong"].cipher', _LTM13_CONF)
    assert res.values_per_file["mem://1"] == ["ECDHE:!RC4"]


def test_ltm_nat_projects_addresses_arp_and_vlans():
    base = '.ltm.nat["/Common/nat_external"]'
    assert _run(f'{base}."translation-address"', _LTM13_CONF).values_per_file["mem://1"] == [
        "192.0.2.50"
    ]
    assert _run(f'{base}."originating-address"', _LTM13_CONF).values_per_file["mem://1"] == [
        "10.0.0.5"
    ]
    assert _run(f"{base}.arp", _LTM13_CONF).values_per_file["mem://1"] == ["enabled"]
    assert _run(f'{base}."vlans-disabled"', _LTM13_CONF).values_per_file["mem://1"] == [True]


def test_ltm_snat_origins_and_snatpool_pathref():
    base = '.ltm.snat["/Common/snat_app"]'
    [origins] = _run(f"{base}.origins", _LTM13_CONF).values_per_file["mem://1"]
    assert sorted(origins) == ["10.0.0.0/24", "192.0.2.0/24"]
    [pool] = _run(f"{base}.snatpool", _LTM13_CONF).values_per_file["mem://1"]
    assert pool.full_path == "/Common/sp_main"


def test_ltm_snat_translation_address_and_traffic_group():
    base = '.ltm.snat-translation["/Common/snat_xlate_1"]'
    assert _run(f"{base}.address", _LTM13_CONF).values_per_file["mem://1"] == ["192.0.2.55"]
    [tg] = _run(f'{base}."traffic-group"', _LTM13_CONF).values_per_file["mem://1"]
    assert tg.full_path == "/Common/traffic-group-1"


def test_ltm_policy_strategy_surfaces_operand_indices():
    base = '.ltm.policy-strategy["/Common/ps_best_match"]'
    assert _run(f"{base}.strategy", _LTM13_CONF).values_per_file["mem://1"] == ["best-match"]
    [operands] = _run(f"{base}.operands", _LTM13_CONF).values_per_file["mem://1"]
    assert sorted(operands) == ["0", "1"]


def test_ltm_traffic_class_classification_and_match_method():
    base = '.ltm.traffic-class["/Common/tc_1"]'
    assert _run(f"{base}.classification", _LTM13_CONF).values_per_file["mem://1"] == ["web"]
    assert _run(f'{base}."match-method"', _LTM13_CONF).values_per_file["mem://1"] == ["exact"]


def test_ltm_traffic_matching_criteria_destination_address_list_pathref_into_firewall():
    """``destination-address-list`` is a PathRef into
    ``security firewall address-list``; chaining ``.addresses``
    walks the addresses list of the referenced firewall address list.
    """
    res = _run(
        '.ltm.traffic-matching-criteria["/Common/tmc_1"]."destination-address-list".addresses',
        _LTM13_CONF,
    )
    [addrs] = res.values_per_file["mem://1"]
    assert list(addrs) == ["10.0.0.0/24"]


def test_ltm_traffic_matching_criteria_inline_vs_list_fields():
    base = '.ltm.traffic-matching-criteria["/Common/tmc_1"]'
    assert _run(f'{base}."source-address-inline"', _LTM13_CONF).values_per_file["mem://1"] == [
        "0.0.0.0"
    ]
    assert _run(f"{base}.protocol", _LTM13_CONF).values_per_file["mem://1"] == ["tcp"]


def test_ltm_ifile_projects_file_name():
    assert _run('.ltm.ifile["/Common/ifile_1"]."file-name"', _LTM13_CONF).values_per_file[
        "mem://1"
    ] == ["/Common/ifile_data"]


def test_ltm_eviction_policy_projects_water_marks():
    base = '.ltm.eviction-policy["/Common/ep_1"]'
    assert _run(f'{base}."high-water-mark"', _LTM13_CONF).values_per_file["mem://1"] == ["95"]
    assert _run(f'{base}."low-water-mark"', _LTM13_CONF).values_per_file["mem://1"] == ["85"]
    assert _run(f'{base}."slow-flow-throttle"', _LTM13_CONF).values_per_file["mem://1"] == [
        "enabled"
    ]


# ---------------------------------------------------------------------------
# Bundle 14 — ltm dns.* (DNS Express).  17 kinds total, including
# 3-word kinds (dns dnssec key/zone, dns cache resolver / transparent /
# validating-resolver, dns hpke key/profile), 4-word kinds (dns cache
# records all/key/msg/nameserver/rrset), and 3-word singletons (dns
# cache global-settings, dns analytics global-settings).
# ---------------------------------------------------------------------------

_LTM14_CONF = """ltm dns nameserver /Common/ns1 {
    address 192.0.2.53
    port 53
    tsig-key /Common/tsig1
}
ltm dns tsig-key /Common/tsig1 {
    algorithm hmac-sha256
    secret obscured
}
ltm dns zone /Common/example.test {
    dns-express-server /Common/ns1
    dns-express-allow-notify { /Common/ns1 }
    dns-express-enabled yes
}
ltm dns dnssec key /Common/zsk1 {
    type zone-signing-key
    algorithm rsasha256
    bit-width 1024
}
ltm dns dnssec zone /Common/example.test-dnssec {
    keys { /Common/zsk1 }
    enable yes
}
ltm dns cache resolver /Common/cr1 {
    message-cache-size 1m
    resolver-cache-size 10m
    answer-default-zones yes
    forward-zones {
        example.test { }
        internal.test { }
    }
}
ltm dns cache transparent /Common/ct1 {
    message-cache-size 1m
}
ltm dns cache validating-resolver /Common/cvr1 {
    message-cache-size 1m
    resolver-cache-size 10m
}
ltm dns cache global-settings {
    expiry-time 0
    nameserver-ttl 0
}
ltm dns cache records all /Common/cr_all { }
ltm dns cache records key /Common/cr_key { }
ltm dns cache records msg /Common/cr_msg { }
ltm dns cache records nameserver /Common/cr_ns { }
ltm dns cache records rrset /Common/cr_rrset { }
ltm dns hpke key /Common/hpke_k1 {
    algorithm hkdf-sha256
}
ltm dns hpke profile /Common/hpke_p1 {
    keys { /Common/hpke_k1 }
}
ltm dns analytics global-settings { }
"""


def test_ltm_dns_nameserver_tsig_key_resolves_through_pathref():
    """Two-word kind ``ltm dns nameserver``; ``tsig-key`` is a PathRef
    into ``ltm dns tsig-key``."""
    res = _run('.ltm.dns-nameserver["/Common/ns1"]."tsig-key".algorithm', _LTM14_CONF)
    assert res.values_per_file["mem://1"] == ["hmac-sha256"]


def test_ltm_dns_zone_dns_express_server_walks_into_nameserver():
    res = _run('.ltm.dns-zone["/Common/example.test"]."dns-express-server".address', _LTM14_CONF)
    assert res.values_per_file["mem://1"] == ["192.0.2.53"]


def test_ltm_dns_dnssec_zone_keys_pathref_list_resolves():
    """Three-word kind ``ltm dns dnssec key`` / ``ltm dns dnssec zone``;
    ``keys[]`` is a PathRef list into ``ltm dns dnssec key``.
    """
    res = _run(".ltm.dns-dnssec-zone[].keys[].algorithm", _LTM14_CONF)
    assert res.values_per_file["mem://1"] == ["rsasha256"]


def test_ltm_dns_cache_resolver_surfaces_forward_zone_keys():
    res = _run(".ltm.dns-cache-resolver[].forward-zones", _LTM14_CONF)
    [zones] = res.values_per_file["mem://1"]
    assert sorted(zones) == ["example.test", "internal.test"]


def test_ltm_dns_cache_transparent_and_validating_resolver_share_shape():
    base_t = '.ltm.dns-cache-transparent["/Common/ct1"]'
    base_v = '.ltm.dns-cache-validating-resolver["/Common/cvr1"]'
    assert _run(f'{base_t}."message-cache-size"', _LTM14_CONF).values_per_file["mem://1"] == ["1m"]
    assert _run(f'{base_v}."message-cache-size"', _LTM14_CONF).values_per_file["mem://1"] == ["1m"]


def test_ltm_dns_cache_global_settings_singleton_projects_expiry_time():
    """Three-word singleton — 4-token header parsed via the new
    `len(parts) == 4 and three_word in _THREE_WORD_TYPES` branch.
    """
    res = _run('.ltm.dns-cache-global-settings[]."expiry-time"', _LTM14_CONF)
    assert res.values_per_file["mem://1"] == ["0"]


def test_ltm_dns_cache_records_merges_all_five_subkinds():
    """Four-word kind: ``ltm dns cache records {all,key,msg,
    nameserver,rrset}``.  The parser collapses them into one
    container keyed by full-path; ``record-kind`` disambiguates.
    """
    res = _run('.ltm.dns-cache-records[]."record-kind"', _LTM14_CONF)
    assert sorted(res.values_per_file["mem://1"]) == [
        "all",
        "key",
        "msg",
        "nameserver",
        "rrset",
    ]


def test_ltm_dns_hpke_profile_keys_pathref_into_hpke_key():
    res = _run('.ltm.dns-hpke-profile["/Common/hpke_p1"].keys[].algorithm', _LTM14_CONF)
    assert res.values_per_file["mem://1"] == ["hkdf-sha256"]


def test_ltm_dns_analytics_global_settings_singleton_present():
    """The empty ``ltm dns analytics global-settings { }`` singleton
    still lands in the container with the empty-string key.
    """
    res = _run('.ltm.dns-analytics-global-settings[]."full-path"', _LTM14_CONF)
    assert res.values_per_file["mem://1"] == [""]


# ---------------------------------------------------------------------------
# Bundle 15 — ltm message-routing.*  (20 kinds across 4 protocols).
# All share BigipLtmMessageRoutingObject + _LTM_MESSAGE_ROUTING_FIELDS.
# ---------------------------------------------------------------------------

_LTM15_CONF = """ltm message-routing diameter peer /Common/dia_peer1 {
    description "diameter peer"
    transport-config /Common/dia_tc1
}
ltm message-routing diameter route /Common/dia_r1 { description "dia route" }
ltm message-routing diameter profile router /Common/dia_pr1 { description "dia router" }
ltm message-routing diameter profile session /Common/dia_ps1 { description "dia session" }
ltm message-routing diameter transport-config /Common/dia_tc1 { description "dia tc" }
ltm message-routing sip peer /Common/sip_peer1 { description "sip peer" }
ltm message-routing sip route /Common/sip_r1 { description "sip route" }
ltm message-routing sip profile router /Common/sip_pr1 { }
ltm message-routing sip profile session /Common/sip_ps1 { }
ltm message-routing sip transport-config /Common/sip_tc1 { }
ltm message-routing mqtt peer /Common/mqtt_peer1 { }
ltm message-routing mqtt route /Common/mqtt_r1 { }
ltm message-routing mqtt profile router /Common/mqtt_pr1 { }
ltm message-routing mqtt profile session /Common/mqtt_ps1 { }
ltm message-routing mqtt transport-config /Common/mqtt_tc1 { }
ltm message-routing generic peer /Common/gen_peer1 { }
ltm message-routing generic protocol /Common/gen_p1 { description "generic protocol" }
ltm message-routing generic route /Common/gen_r1 { }
ltm message-routing generic router /Common/gen_router1 { }
ltm message-routing generic transport-config /Common/gen_tc1 { }
"""


def test_ltm_message_routing_all_20_kinds_dispatch_correctly():
    """Walk every dispatch label and check the kind label
    round-trips through ``.kind``.  Cross-checks the dispatch
    table is in sync with ``_MODULE_KINDS["ltm"]``.
    """
    cases: list[tuple[str, str]] = [
        ("message-routing-diameter-peer", "ltm message-routing diameter peer"),
        ("message-routing-diameter-route", "ltm message-routing diameter route"),
        (
            "message-routing-diameter-profile-router",
            "ltm message-routing diameter profile router",
        ),
        (
            "message-routing-diameter-profile-session",
            "ltm message-routing diameter profile session",
        ),
        (
            "message-routing-diameter-transport-config",
            "ltm message-routing diameter transport-config",
        ),
        ("message-routing-sip-peer", "ltm message-routing sip peer"),
        ("message-routing-sip-route", "ltm message-routing sip route"),
        ("message-routing-sip-profile-router", "ltm message-routing sip profile router"),
        ("message-routing-sip-profile-session", "ltm message-routing sip profile session"),
        ("message-routing-sip-transport-config", "ltm message-routing sip transport-config"),
        ("message-routing-mqtt-peer", "ltm message-routing mqtt peer"),
        ("message-routing-mqtt-route", "ltm message-routing mqtt route"),
        ("message-routing-mqtt-profile-router", "ltm message-routing mqtt profile router"),
        ("message-routing-mqtt-profile-session", "ltm message-routing mqtt profile session"),
        ("message-routing-mqtt-transport-config", "ltm message-routing mqtt transport-config"),
        ("message-routing-generic-peer", "ltm message-routing generic peer"),
        ("message-routing-generic-protocol", "ltm message-routing generic protocol"),
        ("message-routing-generic-route", "ltm message-routing generic route"),
        ("message-routing-generic-router", "ltm message-routing generic router"),
        (
            "message-routing-generic-transport-config",
            "ltm message-routing generic transport-config",
        ),
    ]
    for label, kind_str in cases:
        res = _run(f".ltm.{label}[].kind", _LTM15_CONF)
        kinds = res.values_per_file["mem://1"]
        assert kind_str in kinds, f"{label}: expected kind={kind_str!r}, got {kinds!r}"


def test_ltm_message_routing_four_word_profile_kinds_dispatch():
    """The 6 ``... profile router/session`` kinds are four-word
    kinds (6-token headers).  Verify they land in the right
    container via the new ``_FOUR_WORD_TYPES`` path.
    """
    for label, name in [
        ("message-routing-diameter-profile-router", "dia_pr1"),
        ("message-routing-diameter-profile-session", "dia_ps1"),
        ("message-routing-sip-profile-router", "sip_pr1"),
        ("message-routing-sip-profile-session", "sip_ps1"),
        ("message-routing-mqtt-profile-router", "mqtt_pr1"),
        ("message-routing-mqtt-profile-session", "mqtt_ps1"),
    ]:
        res = _run(f'.ltm.{label}["/Common/{name}"].name', _LTM15_CONF)
        assert res.values_per_file["mem://1"] == [name]


def test_ltm_message_routing_descriptions_round_trip():
    """Unquoted and unquoted-with-spaces descriptions both
    round-trip through the minimal shape.
    """
    res = _run('.ltm.message-routing-diameter-peer["/Common/dia_peer1"].description', _LTM15_CONF)
    assert res.values_per_file["mem://1"] == ["diameter peer"]
    res = _run('.ltm.message-routing-generic-protocol["/Common/gen_p1"].description', _LTM15_CONF)
    assert res.values_per_file["mem://1"] == ["generic protocol"]


# ---------------------------------------------------------------------------
# Bundle 16 — ltm auth.* profiles (11 kinds).  All share
# BigipLtmAuthObject + _LTM_AUTH_FIELDS.
# ---------------------------------------------------------------------------

_LTM16_CONF = """ltm auth profile /Common/auth_p1 {
    description "auth profile"
    defaults-from /Common/auth_default
}
ltm auth ldap /Common/auth_ldap1 { description "ldap" }
ltm auth radius /Common/auth_radius1 { description "radius" }
ltm auth radius-server /Common/auth_rs1 { description "radius-server" }
ltm auth tacacs /Common/auth_tac1 { description "tacacs" }
ltm auth crldp-server /Common/auth_crldp1 { description "crldp" }
ltm auth ocsp-responder /Common/auth_ocsp1 { description "ocsp" }
ltm auth kerberos-delegation /Common/auth_krb1 { description "krb" }
ltm auth ssl-cc-ldap /Common/auth_sslccldap1 { description "ssl-cc-ldap" }
ltm auth ssl-crldp /Common/auth_sslcrldp1 { description "ssl-crldp" }
ltm auth ssl-ocsp /Common/auth_sslocsp1 { description "ssl-ocsp" }
"""


def test_ltm_auth_all_11_kinds_dispatch_correctly():
    """Every ``ltm auth X`` dispatch label round-trips the kind
    label through ``.kind``.
    """
    cases: list[tuple[str, str]] = [
        ("auth-profile", "ltm auth profile"),
        ("auth-ldap", "ltm auth ldap"),
        ("auth-radius", "ltm auth radius"),
        ("auth-radius-server", "ltm auth radius-server"),
        ("auth-tacacs", "ltm auth tacacs"),
        ("auth-crldp-server", "ltm auth crldp-server"),
        ("auth-ocsp-responder", "ltm auth ocsp-responder"),
        ("auth-kerberos-delegation", "ltm auth kerberos-delegation"),
        ("auth-ssl-cc-ldap", "ltm auth ssl-cc-ldap"),
        ("auth-ssl-crldp", "ltm auth ssl-crldp"),
        ("auth-ssl-ocsp", "ltm auth ssl-ocsp"),
    ]
    for label, kind_str in cases:
        res = _run(f".ltm.{label}[].kind", _LTM16_CONF)
        kinds = res.values_per_file["mem://1"]
        assert kind_str in kinds, f"{label}: expected kind={kind_str!r}, got {kinds!r}"


def test_ltm_auth_profile_surfaces_defaults_from():
    """Auth profiles inherit from a default profile via
    ``defaults-from``; the shared shape captures it."""
    res = _run('.ltm.auth-profile["/Common/auth_p1"]."defaults-from"', _LTM16_CONF)
    assert res.values_per_file["mem://1"] == ["/Common/auth_default"]


def test_ltm_auth_isolation_from_ltm_message_routing_dispatch():
    """The ``auth-*`` dispatch path is independent of the
    ``message-routing-*`` path even though both share a minimal
    shape; confirm names and containers don't bleed across.
    """
    res = _run('.ltm.auth-ldap[]."full-path"', _LTM16_CONF)
    assert res.values_per_file["mem://1"] == ["/Common/auth_ldap1"]


# ---------------------------------------------------------------------------
# Bundle 17 — ltm CGNAT / LSN (3 kinds, generic minimal shape).
# ---------------------------------------------------------------------------

_LTM17_CONF = """ltm lsn-pool /Common/lsn1 { description "lsn pool" }
ltm lsn-log-profile /Common/lsnlog1 { description "lsn log profile" }
ltm alg-log-profile /Common/alglog1 { description "alg log profile" }
"""


def test_ltm_bundle_17_lsn_kinds_dispatch():
    for label, kind_str in [
        ("lsn-pool", "ltm lsn-pool"),
        ("lsn-log-profile", "ltm lsn-log-profile"),
        ("alg-log-profile", "ltm alg-log-profile"),
    ]:
        res = _run(f".ltm.{label}[].kind", _LTM17_CONF)
        assert kind_str in res.values_per_file["mem://1"], f"{label}"


# Bundle 18 — ltm global-settings + misc singletons (6 kinds).

_LTM18_CONF = """ltm default-node-monitor { rule none }
ltm global-settings connection { description "conn" }
ltm global-settings general { description "general" }
ltm global-settings rule { description "rule" }
ltm global-settings traffic-control { description "tc" }
ltm rule-profiler { description "profiler" }
"""


def test_ltm_bundle_18_global_settings_dispatch():
    for label, kind_str in [
        ("default-node-monitor", "ltm default-node-monitor"),
        ("global-settings-connection", "ltm global-settings connection"),
        ("global-settings-general", "ltm global-settings general"),
        ("global-settings-rule", "ltm global-settings rule"),
        ("global-settings-traffic-control", "ltm global-settings traffic-control"),
        ("rule-profiler", "ltm rule-profiler"),
    ]:
        res = _run(f".ltm.{label}[].kind", _LTM18_CONF)
        assert kind_str in res.values_per_file["mem://1"], f"{label}"


# Bundle 19+20 — ltm classification + clientssl + tacdb (14 kinds).

_LTM19_CONF = """ltm classification application /Common/ca1 { }
ltm classification auto-update settings { description "auto-up" }
ltm classification category /Common/cc1 { }
ltm classification ce /Common/ce1 { }
ltm classification signature-update-schedule /Common/sus1 { }
ltm classification url-cat-policy /Common/ucp1 { }
ltm classification url-category /Common/uc1 { }
ltm classification urldb-feed-list /Common/ufl1 { }
ltm classification urldb-file /Common/uf1 { }
ltm clientssl ocsp-stapling-responses /Common/csor1 { }
ltm clientssl-proxy cached-certs /Common/cpcc1 { }
ltm tacdb customdb /Common/td1 { }
ltm tacdb customdb-file /Common/tdf1 { }
ltm tacdb licenseddb /Common/tld1 { }
"""


def test_ltm_bundle_19_and_20_dispatch():
    for label in [
        "classification-application",
        "classification-auto-update-settings",
        "classification-category",
        "classification-ce",
        "classification-signature-update-schedule",
        "classification-url-cat-policy",
        "classification-url-category",
        "classification-urldb-feed-list",
        "classification-urldb-file",
        "clientssl-ocsp-stapling-responses",
        "clientssl-proxy-cached-certs",
        "tacdb-customdb",
        "tacdb-customdb-file",
        "tacdb-licenseddb",
    ]:
        res = _run(f".ltm.{label}[].kind", _LTM19_CONF)
        assert res.values_per_file["mem://1"], f"{label}: no values"


# Bundles 21-26 — net.* minimal kinds (65 across routing, tunnels,
# ipsec, BWC/cos/rate-shaping, L2/misc, SFC).

_NET_BUNDLES_CONF = """net routing access-list /Common/al1 { description "al" }
net routing bgp /Common/bgp1 { description "bgp" }
net routing profile bgp /Common/pb1 { description "pb" }
net router-advertisement /Common/ra1 { description "ra" }
net tunnels gre /Common/gre1 { description "gre" }
net tunnels vxlan /Common/vx1 { description "vx" }
net ipsec ike-daemon /Common/iked1 { description "iked" }
net ipsec ike-peer /Common/ikepeer1 { description "ikepeer" }
net bwc policy /Common/bwc1 { description "bwc" }
net cos global-settings { description "cos" }
net rate-shaping class /Common/rsc1 { description "rsc" }
net address-list /Common/aln1 { description "addrl" }
net lacp-globals { description "lacp" }
net fdb tunnel /Common/fdbt1 { description "fdb-t" }
net sfc chain /Common/sfc1 { description "sfc" }
"""


def test_net_bundles_21_to_26_dispatch():
    for label in [
        "routing-access-list",
        "routing-bgp",
        "routing-profile-bgp",
        "router-advertisement",
        "tunnels-gre",
        "tunnels-vxlan",
        "ipsec-ike-daemon",
        "ipsec-ike-peer",
        "bwc-policy",
        "cos-global-settings",
        "rate-shaping-class",
        "address-list",
        "lacp-globals",
        "fdb-tunnel",
        "sfc-chain",
    ]:
        res = _run(f".net.{label}[].kind", _NET_BUNDLES_CONF)
        assert res.values_per_file["mem://1"], f"{label}: no values"


# Bundles 27-31 — apm.* minimal kinds (82 across aaa, profile/sso, resource,
# oauth, saml/ntlm/policy customisation).
_APM_BUNDLES_CONF = """apm aaa active-directory /Common/ad1 { description "ad" }
apm aaa kerberos /Common/krb1 { description "k" }
apm aaa ldap /Common/al1 { description "l" }
apm aaa radius /Common/ar1 { description "r" }
apm profile access /Common/pa1 { description "pa" }
apm profile connectivity /Common/pc1 { description "pc" }
apm sso basic /Common/sb1 { description "sb" }
apm sso saml /Common/ss1 { description "ss" }
apm resource address-space /Common/ras1 { description "ras" }
apm resource portal-access /Common/rpa1 { description "rpa" }
apm resource remote-desktop citrix /Common/rdc1 { description "rdc" }
apm resource remote-desktop rdp /Common/rrdp1 { description "rrdp" }
apm oauth jwk-config /Common/jwk1 { description "j" }
apm oauth oauth-claim /Common/oc1 { description "oc" }
apm saml artifact-resolution-service /Common/ars1 { description "ars" }
apm ntlm machine-account /Common/nm1 { description "nm" }
apm acl /Common/acl1 { description "acl" }
apm log-setting /Common/ls1 { description "ls" }
apm url-filter /Common/uf1 { description "uf" }
apm swg-scheme /Common/ss2 { description "ss" }
apm client image /Common/ci1 { description "ci" }
apm policy customization-group /Common/cg1 { description "cg" }
"""


def test_apm_bundles_27_to_31_dispatch():
    for label in [
        "aaa-active-directory",
        "aaa-kerberos",
        "aaa-ldap",
        "aaa-radius",
        "profile-access",
        "profile-connectivity",
        "sso-basic",
        "sso-saml",
        "resource-address-space",
        "resource-portal-access",
        "resource-remote-desktop-citrix",
        "resource-remote-desktop-rdp",
        "oauth-jwk-config",
        "oauth-oauth-claim",
        "saml-artifact-resolution-service",
        "ntlm-machine-account",
        "acl",
        "log-setting",
        "url-filter",
        "swg-scheme",
        "client-image",
        "policy-customization-group",
    ]:
        res = _run(f".apm.{label}[].kind", _APM_BUNDLES_CONF)
        assert res.values_per_file["mem://1"], f"{label}: no values"


# Bundle 32 — pem.* globals + protocol (16 kinds).
_PEM_BUNDLE_CONF = """pem global-settings analytics { description "a" }
pem global-settings policy { description "p" }
pem protocol diameter-avp /Common/da1 { description "da" }
pem protocol profile gx /Common/pgx { description "pgx" }
pem reporting format-script /Common/fs1 { description "fs" }
pem subscriber /Common/s1 { description "s" }
pem subscriber-attribute /Common/sa1 { description "sa" }
"""


def test_pem_bundle_32_dispatch():
    for label in [
        "global-settings-analytics",
        "global-settings-policy",
        "protocol-diameter-avp",
        "protocol-profile-gx",
        "reporting-format-script",
        "subscriber",
        "subscriber-attribute",
    ]:
        res = _run(f".pem.{label}[].kind", _PEM_BUNDLE_CONF)
        assert res.values_per_file["mem://1"], f"{label}: no values"


# Bundles 33-45 — sys/vcmp/cm/cli/api-protection minimal kinds.
_BUNDLES_33_45_CONF = """sys ha-group /Common/hg1 { description "hg" }
sys application service /Common/as1 { description "as" }
sys httpd { description "httpd" }
sys url-db download-schedule /Common/uds1 { description "uds" }
sys file ifile /Common/fi1 { description "fi" }
sys log-config destination splunk /Common/lds1 { description "lds" }
sys log-config publisher /Common/lcp1 { description "lcp" }
sys daemon-log-settings mcpd /Common/dlsm1 { description "dlsm" }
sys crypto cert /Common/cc1 { description "cc" }
sys crypto cert-validator ocsp /Common/cvo1 { description "cvo" }
sys ipfix destination /Common/id1 { description "id" }
sys icall handler periodic /Common/ihp1 { description "ihp" }
sys icall script /Common/is1 { description "is" }
sys management-dhcp /Common/md1 { description "md" }
sys sflow receiver /Common/sr1 { description "sr" }
sys sflow global-settings http { description "sgh" }
sys software image /Common/si1 { description "si" }
sys cluster { description "c" }
sys aom { description "aom" }
vcmp guest /Common/vg1 { description "vg" }
vcmp traffic-profile /Common/vtp1 { description "vtp" }
cm ha-group /Common/cmh1 { description "cmh" }
cm config-sync { description "cs" }
cli admin-partitions { description "ap" }
cli alias shared /Common/cas1 { description "cas" }
cli version { description "v" }
api-protection profile apiprotection /Common/aap1 { description "aap" }
api-protection response /Common/apr1 { description "apr" }
api-protection server /Common/aps1 { description "aps" }
"""


def test_bundles_33_to_45_dispatch():
    for path, expected_kind in [
        (".sys.ha-group[].kind", "sys ha-group"),
        (".sys.application-service[].kind", "sys application service"),
        (".sys.httpd[].kind", "sys httpd"),
        (".sys.url-db-download-schedule[].kind", "sys url-db download-schedule"),
        (".sys.file-ifile[].kind", "sys file ifile"),
        (".sys.log-config-destination-splunk[].kind", "sys log-config destination splunk"),
        (".sys.log-config-publisher[].kind", "sys log-config publisher"),
        (".sys.crypto-cert[].kind", "sys crypto cert"),
        (".sys.crypto-cert-validator-ocsp[].kind", "sys crypto cert-validator ocsp"),
        (".sys.icall-handler-periodic[].kind", "sys icall handler periodic"),
        (".sys.management-dhcp[].kind", "sys management-dhcp"),
        (".sys.sflow-global-settings-http[].kind", "sys sflow global-settings http"),
        (".sys.software-image[].kind", "sys software image"),
        (".sys.cluster[].kind", "sys cluster"),
        (".vcmp.guest[].kind", "vcmp guest"),
        (".cm.ha-group[].kind", "cm ha-group"),
        (".cm.config-sync[].kind", "cm config-sync"),
        (".cli.admin-partitions[].kind", "cli admin-partitions"),
        (".cli.alias-shared[].kind", "cli alias shared"),
        (".cli.version[].kind", "cli version"),
        (".api-protection.profile-apiprotection[].kind", "api-protection profile apiprotection"),
        (".api-protection.response[].kind", "api-protection response"),
        (".api-protection.server[].kind", "api-protection server"),
    ]:
        res = _run(path, _BUNDLES_33_45_CONF)
        assert expected_kind in res.values_per_file["mem://1"], (
            f"{path}: expected {expected_kind!r}, got {res.values_per_file['mem://1']!r}"
        )


# Audit follow-up — kinds found in real BIG-IP configs that the
# projection-gaps doc didn't enumerate, plus dispatch bugs for
# previously-registered monitor / profile subtypes that fell out
# of the strict TWO_WORD_TYPES check.

_AUDIT_FOLLOWUP_CONF = """ltm monitor diameter /Common/m_dia { interval 30 }
ltm monitor dns /Common/m_dns { interval 30 }
ltm monitor mqtt /Common/m_mqtt { interval 30 }
ltm monitor smb /Common/m_smb { interval 30 }
ltm monitor virtual-location /Common/m_vl { interval 30 }
ltm profile analytics /Common/p_an { app-service none }
ltm profile classification /Common/p_cl { app-service none }
ltm profile ipother /Common/p_ip { app-service none }
ltm profile request-log /Common/p_rl { app-service none }
ltm profile tcp-analytics /Common/p_ta { app-service none }
ltm html-rule comment-raise-event /Common/hr1 { description "h" }
ltm html-rule tag-remove /Common/hr2 { description "h" }
security shared-objects port-list /Common/sopl1 { description "spl" }
security shared-objects address-list /Common/soal1 { description "sal" }
security dos ipv6-ext-hdr /Common/dosipv6 { description "ipv6" }
sys ecm cloud-provider /Common/aws-ec2 { description "aws" }
sys software update { description "sw" }
sys dynad settings { description "dynad" }
sys compatibility-level { description "cl" }
sys diags ihealth { description "diags" }
apm client-packaging /Common/cp1 { description "cp" }
pem irule /Common/pi1 { description "pi" }
asm policy /Common/ap1 { description "asm" }
ilx global-settings { description "ilx" }
wom endpoint-discovery { description "wom" }
"""


def test_audit_followup_dispatch_bugs_resolved():
    """The 11 missing ``ltm monitor`` subtypes and 5 missing
    ``ltm profile`` subtypes were registered in
    ``_KIND_FIELD_MAPS`` but ``_TWO_WORD_TYPES`` never listed
    them, so the parser fell back to single-word ``monitor`` /
    ``profile`` and the match-block ``case _ if obj_type.startswith
    ("monitor "):`` never fired.  This test verifies the dispatch
    now produces populated containers.
    """
    cfg = parse_query  # noqa: F841 — placeholder to keep imports stable
    from core.bigip.parser import parse_bigip_conf

    cfg = parse_bigip_conf(_AUDIT_FOLLOWUP_CONF)
    monitor_types = sorted(m.monitor_type for m in cfg.monitors.values())
    assert "diameter" in monitor_types
    assert "dns" in monitor_types
    assert "mqtt" in monitor_types
    assert "virtual-location" in monitor_types
    profile_paths = sorted(cfg.profiles.keys())
    assert "/Common/p_an" in profile_paths
    assert "/Common/p_cl" in profile_paths


def test_audit_followup_new_kinds_dispatch():
    """The 21 truly-unhandled kinds the audit found in real
    BIG-IP configs (html-rule.*, shared-objects.*, sys ecm
    cloud-provider, sys software update, dynad, compatibility-
    level, diags ihealth, apm client-packaging, pem irule, plus
    new modules asm/ilx/wom) all dispatch to typed containers.
    ``pem irule`` parses via the dedicated ``_parse_pem_irule``
    path into ``cfg.pem_rules`` (BigipPemRule, with ``source``
    field projected as ``body``).
    """
    from core.bigip.parser import parse_bigip_conf

    cfg = parse_bigip_conf(_AUDIT_FOLLOWUP_CONF)
    # ltm html-rule subtypes route to dedicated containers.
    assert cfg.ltm_html_rule_comment_raise_event
    assert cfg.ltm_html_rule_tag_remove
    # security follow-ups.
    assert cfg.security_shared_objects_port_lists
    assert cfg.security_shared_objects_address_lists
    assert cfg.security_dos_ipv6_ext_hdr
    # sys follow-ups.
    assert cfg.sys_ecm_cloud_provider
    assert cfg.sys_software_update
    assert cfg.sys_dynad_settings.get("")  # singleton
    assert cfg.sys_compatibility_level.get("")  # singleton
    assert cfg.sys_diags_ihealth.get("")  # singleton
    # apm + pem follow-ups.  ``pem irule`` is the only kind in the
    # PEM iRule family; it parses via the dedicated parser into
    # ``pem_rules`` so the body remains addressable as ``.body``.
    assert cfg.apm_client_packaging
    assert cfg.pem_rules
    # New modules.
    assert cfg.asm_policies
    assert cfg.ilx_global_settings.get("")  # singleton
    assert cfg.wom_endpoint_discovery.get("")  # singleton


_SIBLING_FOLLOWUP_CONF = """\
net routing as-path /Common/asp1 {
    entries {
        10 { action permit regex "^65001$" }
    }
}
security dos profile-signatures /Common/dpsig1 { description "dps" }
apm aaa localdb /Common/my_local_db { description "ldb" }
analytics global-settings {
    avrd-debug-mode enabled
    avrd-interval 60
    offbox-protocol tcp
}
"""


def test_sibling_completeness_new_kinds_dispatch():
    """Four sibling-completeness follow-ups found by a wider corpus
    scan against HOL-2571 + BigIPReport + SSL Orchestrator .scf
    fixtures + F5 ACC doConverter test configs:

    - ``net routing as-path`` — sibling of net routing family.
    - ``security dos profile-signatures`` — sibling of dos family.
    - ``apm aaa localdb`` — sibling of apm aaa family.
    - ``analytics global-settings`` — top-level ``analytics``
      module (distinct from ``ltm dns analytics global-settings``,
      which is a different kind under ``ltm``).
    """
    from core.bigip.parser import parse_bigip_conf

    cfg = parse_bigip_conf(_SIBLING_FOLLOWUP_CONF)
    assert "/Common/asp1" in cfg.net_routing_as_paths
    assert "/Common/dpsig1" in cfg.security_dos_profile_signatures
    assert "/Common/my_local_db" in cfg.apm_aaa_localdb
    assert cfg.analytics_global_settings.get("")  # singleton

    # Kind labels on the typed objects reflect the TMSH kind.
    assert cfg.net_routing_as_paths["/Common/asp1"].kind == "net routing as-path"
    assert (
        cfg.security_dos_profile_signatures["/Common/dpsig1"].kind
        == "security dos profile-signatures"
    )
    assert cfg.apm_aaa_localdb["/Common/my_local_db"].kind == "apm aaa localdb"
    assert cfg.analytics_global_settings[""].kind == "analytics global-settings"

    # Projection navigates the new kinds end-to-end.
    desc = _run(".security.dos-profile-signatures.dpsig1.description", _SIBLING_FOLLOWUP_CONF)
    assert desc.values_per_file["mem://1"] == ["dps"]
    full = _run(".analytics.global-settings[].full-path", _SIBLING_FOLLOWUP_CONF)
    assert full.values_per_file["mem://1"] == [""]


# Aggregate kinds — ``_MODULE_KINDS`` exposes the family name
# (``ltm monitor``, ``ltm profile``, …) but TMSH requires a
# subtype on every stanza (``ltm monitor http``, ``ltm profile
# tcp``, …).  The sweep uses these subtype hints to probe the
# aggregate containers.
_AGGREGATE_KIND_PROBES: dict[str, str] = {
    "ltm profile": "ltm profile tcp",
    "ltm monitor": "ltm monitor http",
    "ltm persistence": "ltm persistence cookie",
    "ltm data-group": "ltm data-group internal",
    "ltm dns cache records": "ltm dns cache records rrset",
    "gtm pool": "gtm pool a",
    "gtm wideip": "gtm wideip a",
    "gtm monitor": "gtm monitor http",
    "apm policy agent": "apm policy agent message-box",
    "pem profile": "pem profile spm",
}


def test_every_module_kind_round_trips():
    """Sweep every ``_MODULE_KINDS`` entry, parse a minimal stanza
    for it, and confirm the typed container is populated.

    Guards against the failure mode where a kind is registered in
    ``_MODULE_KINDS`` / ``_KIND_FIELD_MAPS`` but the parser never
    routes to its container — navigation would silently return an
    empty stream rather than the parsed objects.  Each kind is
    tried in three forms (named ``/Common/t``, bare singleton, or
    a subtype probe from :data:`_AGGREGATE_KIND_PROBES`); any one
    populating the container counts as success.
    """
    from core.bigip.parser import parse_bigip_conf
    from core.bigip.query.projection import MODULE_KINDS

    failed: list[tuple[str, str, str, str]] = []
    for module, kinds in MODULE_KINDS.items():
        for label, (attr, tmsh_kind) in kinds.items():
            header = _AGGREGATE_KIND_PROBES.get(tmsh_kind, tmsh_kind)
            # Named form first.
            cfg = parse_bigip_conf(f"{header} /Common/t {{ }}\n")
            if getattr(cfg, attr):
                continue
            # Singleton form (no path).
            cfg = parse_bigip_conf(f"{header} {{ }}\n")
            if getattr(cfg, attr):
                continue
            failed.append((module, label, tmsh_kind, attr))

    if failed:
        rendered = "\n".join(
            f"  {mod}.{label}  ({kind!r} → {attr})" for mod, label, kind, attr in failed
        )
        raise AssertionError(
            f"{len(failed)} module-kind entr{'y' if len(failed) == 1 else 'ies'} "
            f"did not round-trip:\n{rendered}"
        )


# ---------------------------------------------------------------------------
# Stress-test regressions (g6827 report — keep these pinned so the
# fixes don't silently regress on the next refactor).
# ---------------------------------------------------------------------------


def test_stress_bug1_any_over_mapped_stream_returns_false_when_all_false():
    # Bug 1 — ``any(stream | map(predicate))`` used to collapse to
    # "is the per-iteration map output non-empty?" because each
    # ``map`` call returns a single-element list and ``[False]`` is
    # truthy.  ``any`` / ``all`` now flatten one level when every
    # element is a list, so the predicate truthiness wins.
    scf = (
        "ltm virtual /Common/a { destination /Common/1.1.1.1:80 }\n"
        "ltm virtual /Common/b { destination /Common/1.1.1.2:80 }\n"
        "ltm virtual /Common/c { destination /Common/1.1.1.3:80 }\n"
    )
    [hit] = _run('any(.ltm.virtual[].name | map(. == "nope"))', source=scf).values_per_file[
        "mem://1"
    ]
    assert hit is False
    [hit_wrapped] = _run(
        'any([.ltm.virtual[].name | map(. == "nope")])', source=scf
    ).values_per_file["mem://1"]
    assert hit_wrapped is False
    # Predicate form (the idiomatic shape) stays correct too.
    [hit_predicate] = _run('any(.ltm.virtual[].name | (. == "nope"))', source=scf).values_per_file[
        "mem://1"
    ]
    assert hit_predicate is False
    # ``all`` over an all-true mapped stream is true.
    [pass_all] = _run(
        'all(.ltm.virtual[].name | map(startswith(., "")))', source=scf
    ).values_per_file["mem://1"]
    assert pass_all is True


def test_stress_bug2_comma_in_list_literal_has_targeted_error():
    # Bug 2 — ``count([1, 2])`` used to report "expected ']' to
    # close list literal" pointing past the comma.  The error now
    # names the comma and explains the DSL doesn't expose ``,``
    # as a pipeline operator.
    with pytest.raises(ParseError) as exc:
        _run("count([1, 2])", source="")
    assert "','" in str(exc.value)
    assert "pipeline operator" in str(exc.value)


def test_stress_bug3_profile_rename_hits_stanza_and_attachments():
    # Bug 3 — ``.ltm.profile[X].name = Y`` used to skip the
    # sub-typed stanza header (``ltm profile tcp ...``) and the
    # VS ``profiles { /Common/X { } }`` attachment, renaming only
    # the ``defaults-from`` reference.  Now all three sites land.
    scf = (
        "ltm profile tcp /Common/parent_tcp { idle-timeout 600 }\n"
        "ltm profile tcp /Common/child_tcp  { defaults-from /Common/parent_tcp }\n"
        "ltm virtual /Common/vs1 {\n"
        "    destination /Common/1.1.1.1:80\n"
        "    profiles {\n"
        "        /Common/parent_tcp { }\n"
        "    }\n"
        "}\n"
    )
    result = _run(
        '.ltm.profile["/Common/parent_tcp"].name = "/Common/parent_renamed"',
        source=scf,
    )
    applied = result.edits_per_file["mem://1"]
    [report] = applied.rename_reports
    assert report.occurrences == 3
    assert "ltm profile tcp /Common/parent_renamed {" in applied.new_source
    assert "defaults-from /Common/parent_renamed" in applied.new_source
    assert "/Common/parent_renamed { }" in applied.new_source
    # Stanzas that *don't* match must not be touched.
    assert "/Common/child_tcp" in applied.new_source


def test_stress_bug4_data_group_rename_hits_stanza_and_irule_body():
    # Bug 4 — same regex root cause as bug 3: the stanza header
    # ``ltm data-group internal /Common/dg_a`` was skipped because
    # the on-disk kind ``ltm data-group internal`` didn't equal
    # the projection kind ``ltm data-group``.  Family-prefix check
    # in :func:`_other_kind_header_spans` now passes both.
    scf = (
        "ltm data-group internal /Common/dg_a {\n"
        '    type string\n    records { "x" { data 1 } }\n}\n'
        "ltm rule /Common/r1 {\n"
        "when HTTP_REQUEST {\n"
        "    if { [class match [HTTP::uri] equals /Common/dg_a] } "
        '{ log local0. "hit" }\n'
        "}\n"
        "}\n"
    )
    result = _run(
        '.ltm["data-group"]["/Common/dg_a"].name = "/Common/dg_renamed"',
        source=scf,
    )
    applied = result.edits_per_file["mem://1"]
    [report] = applied.rename_reports
    assert report.occurrences == 2
    assert "ltm data-group internal /Common/dg_renamed {" in applied.new_source
    assert "equals /Common/dg_renamed" in applied.new_source


def test_stress_bug5_module_subscript_yields_objects_not_containers():
    # Bug 5 — ``.pem[]`` (and ``.ltm[]`` / ``.[]``) used to return
    # the per-kind ``Container`` wrapper instead of the addressable
    # objects, so the renderer fell back to ``repr`` and dumped the
    # entire BigipConfig.  ``_subscript_root`` now recursively
    # flattens Container nesting down to ObjectRefs.
    scf = "pem policy /Common/p { }\nltm pool /Common/pool1 { }\n"
    pem_values = _run(".pem[]", source=scf).values_per_file["mem://1"]
    assert len(pem_values) == 1
    assert pem_values[0].kind == "pem policy"
    assert pem_values[0].full_path == "/Common/p"
    # Root ``.[]`` walks through both module → kind levels.
    root_values = _run(".[]", source=scf).values_per_file["mem://1"]
    full_paths = sorted(v.full_path for v in root_values)
    assert full_paths == ["/Common/p", "/Common/pool1"]
    # And the renderer no longer leaks ``Container(...)`` text.
    rendered = render(root_values, mode="paths")
    assert "Container(" not in rendered


def test_stress_bug6_persist_insert_path_is_detected_as_persistence_ref():
    # Bug 6 — ``persist <kind> insert /Common/X`` is technically a
    # session-key value per F5 docs, but real configs use a
    # TMSH-shaped path there as a convention.  The candidate ref
    # is now emitted (when args[2] is path-shaped) and gated by
    # the downstream ``compute_grep`` to actual config objects.
    scf = (
        "ltm persistence cookie /Common/persist_a { defaults-from /Common/cookie }\n"
        "ltm rule /Common/r {\n"
        "when HTTP_REQUEST { persist cookie insert /Common/persist_a }\n"
        "}\n"
    )
    refs = _run('.ltm.rule["/Common/r"].refs.persists[]', source=scf).values_per_file["mem://1"]
    assert [ref.full_path for ref in refs] == ["/Common/persist_a"]

    # Non-existent persistence: candidate emitted but filtered to
    # empty by the grep-driven object resolution.
    scf_missing = (
        "ltm rule /Common/r {\nwhen HTTP_REQUEST { persist cookie insert /Common/missing }\n}\n"
    )
    refs = _run('.ltm.rule["/Common/r"].refs.persists[]', source=scf_missing).values_per_file[
        "mem://1"
    ]
    assert refs == []


def test_stress_bug7_string_int_concatenation_coerces():
    # Bug 7 — ``str + int`` used to error with "cannot add str
    # and int" — no implicit coercion, no ``str()`` builtin.
    # ``_add`` now coerces the non-string side; an explicit
    # ``str()`` builtin handles non-string-on-both-sides cases.
    scf = (
        "ltm pool /Common/p1 { members { /Common/n:80 { address 1.1.1.1 } } }\n"
        "ltm pool /Common/p2 {\n"
        "    members {\n"
        "        /Common/n:81 { address 1.1.1.2 }\n"
        "        /Common/n:82 { address 1.1.1.3 }\n"
        "    }\n"
        "}\n"
    )
    rendered = _run('.ltm.pool[] | .name + ": " + count(.members)', source=scf).values_per_file[
        "mem://1"
    ]
    assert rendered == ["p1: 1", "p2: 2"]
    # Explicit ``str()`` builtin handles the both-non-strings case.
    [s] = _run("str(42)", source="").values_per_file["mem://1"]
    assert s == "42"
    [s] = _run("str(true)", source="").values_per_file["mem://1"]
    assert s == "true"
    [s] = _run("str(null)", source="").values_per_file["mem://1"]
    assert s == "null"


# ---------------------------------------------------------------------------
# Code-review regressions (queryreview.md — keep these pinned so future
# refactors of the editor / mutation paths can't silently regress).
# ---------------------------------------------------------------------------


def test_review_bug1_field_edit_rejects_newline_injection():
    # Bug 1 — splicing raw user strings into SCF without quoting let
    # ``description = "ok\nltm pool /Common/injected { }"`` synthesise
    # a fresh stanza on a new line, escaping the surrounding scope.
    # The SCF scalar encoder now rejects any value containing
    # newlines / braces / control chars.
    from core.bigip.query.errors import EditError

    src = (
        "ltm virtual /Common/web_vs {\n"
        "    destination /Common/10.0.0.10:443\n"
        "    description old\n"
        "}\n"
    )
    payload = "ok\nltm pool /Common/injected { }"
    with pytest.raises(EditError) as exc:
        _run(
            f'.ltm.virtual.web_vs.description = "{payload}"',
            source=src,
        )
    assert "cannot be safely represented" in str(exc.value)


def test_review_bug1_field_edit_quotes_whitespace_values():
    # Bug 1, partner case — values that *can* be safely encoded must
    # land double-quoted with proper escapes, so the rewritten config
    # both parses and round-trips.
    src = (
        "ltm virtual /Common/web_vs {\n"
        "    destination /Common/10.0.0.10:443\n"
        "    description old\n"
        "}\n"
    )
    out = (
        _run(
            '.ltm.virtual.web_vs.description = "hello world"',
            source=src,
        )
        .edits_per_file["mem://1"]
        .new_source
    )
    assert 'description "hello world"' in out
    # And a value with embedded quotes must escape them.
    out = (
        _run(
            '.ltm.virtual.web_vs.description = "say \\"hi\\""',
            source=src,
        )
        .edits_per_file["mem://1"]
        .new_source
    )
    assert 'description "say \\"hi\\""' in out


def test_review_bug1_field_edit_validates_post_splice_parse():
    # Bug 1, defence-in-depth — even when the encoder lets a value
    # through, the post-splice ``parse_bigip_conf`` call must catch
    # any malformed result and raise instead of returning the broken
    # source to ``--write`` / ``--in-place``.  Hard to trigger from
    # legal user input now that the encoder is in place; verify the
    # check is wired up by inspecting the error path on a forbidden
    # value (which would otherwise have been silently spliced before
    # the encoder existed).
    from core.bigip.query.errors import EditError

    src = "ltm virtual /Common/v { destination /Common/1.1.1.1:80 description x }\n"
    with pytest.raises(EditError):
        _run('.ltm.virtual.v.description = "a{b"', source=src)


def test_review_bug2_query_in_place_rejects_tmsh_format(tmp_path, capsys):
    # Bug 2 — ``f5 query --in-place --format tmsh`` would overwrite
    # the on-disk SCF source with a tmsh script, with the dry-run
    # diff (always SCF↔SCF) hiding the format change.  CLI now
    # refuses the combination before any work runs.
    p = tmp_path / "x.conf"
    p.write_text("ltm pool /Common/p { }\n", encoding="utf-8")
    original = p.read_text(encoding="utf-8")
    rc, _out, err = _cli(
        [
            "query",
            "--in-place",
            "--format",
            "tmsh",
            '.ltm.pool["/Common/p"].description = "x"',
            str(p),
        ],
        capsys,
    )
    assert rc == 2
    assert "incompatible with --format tmsh" in err
    # File untouched.
    assert p.read_text(encoding="utf-8") == original


def test_review_bug2_rename_in_place_rejects_tmsh_format(tmp_path, capsys):
    # Bug 2, partner case — same destructive mode on ``f5 rename``.
    p = tmp_path / "x.conf"
    p.write_text("ltm pool /Common/old { }\n", encoding="utf-8")
    original = p.read_text(encoding="utf-8")
    rc, _out, err = _cli(
        [
            "rename",
            "--in-place",
            "--format",
            "tmsh",
            "/Common/old",
            "/Common/new",
            str(p),
        ],
        capsys,
    )
    assert rc == 2
    assert "incompatible with --format tmsh" in err
    assert p.read_text(encoding="utf-8") == original


def test_review_bug7_regex_pattern_length_capped():
    # Bug 7 — every user-supplied regex (``match`` / ``sub`` / ``gsub``
    # / ``[~"..."]``) routes through ``_safe_regex_compile`` which
    # caps pattern length to keep compile + match cost bounded.
    from core.bigip.query.errors import BuiltinError

    src = "ltm node /Common/n { address 1.1.1.1 }\n"
    long_pattern = "a" * 2000
    with pytest.raises(BuiltinError) as exc:
        _run(f'.ltm.node[].name | match(., "{long_pattern}")', source=src)
    assert "pattern too long" in str(exc.value)


def test_review_bug7_regex_rejects_nested_quantifier_redos():
    # Bug 7, partner case — refuse the textbook ``(a+)+`` catastrophic-
    # backtracking shape.  Not a complete ReDoS detector (that's
    # undecidable), but copy-paste CVE patterns get caught at the door.
    from core.bigip.query.errors import BuiltinError

    src = "ltm node /Common/n { address 1.1.1.1 }\n"
    with pytest.raises(BuiltinError) as exc:
        _run('.ltm.node[].name | match(., "(a+)+")', source=src)
    assert "catastrophic backtracking" in str(exc.value)


def test_review_bug8_active_root_lookup_uses_contextvar():
    # Bug 8 — graph-builtin context map is now a ``ContextVar`` so
    # the runner is safe under concurrent ``run_query`` calls.
    # Verify the new lookup helper is the only public knob.
    from core.bigip.query import runner

    assert hasattr(runner, "_lookup_active_root")
    # The legacy module-global ``_ACTIVE_ROOTS`` dict is gone.
    var = runner._ACTIVE_ROOTS
    assert var.__class__.__name__ == "ContextVar"


def test_review_bug4_merge_picks_up_every_dict_field():
    # Bug 4 — the legacy hand-rolled ``merged.X.update(cfg.X)`` list
    # in the LSP scanner missed every kind beyond the v1 ten, so
    # workspace consumers of ``merged_bigip_config`` lost (e.g.)
    # every PEM, security, GTM, APM, CM, sys, and net object.
    # ``BigipConfig.merge()`` is now introspection-driven and picks
    # up new kinds automatically.
    from core.bigip.model import BigipConfig
    from core.bigip.parser import parse_bigip_conf

    a = parse_bigip_conf("ltm pool /Common/p { }\n")
    b = parse_bigip_conf("pem policy /Common/q { }\nnet vlan /Common/v { tag 100 }\n")
    merged = BigipConfig()
    merged.merge(a)
    merged.merge(b)
    # Legacy v1 kind survived.
    assert "/Common/p" in merged.pools
    # Newly added per-F5-module kinds also flow through.
    assert "/Common/q" in merged.pem_policies
    assert "/Common/v" in merged.net_vlans


def test_review_bug4_merge_overlay_semantics_match_dict_update():
    # Bug 4, semantics check — keys from the second ``merge`` win on
    # conflict (standard ``dict.update`` semantics).
    from core.bigip.model import BigipConfig
    from core.bigip.parser import parse_bigip_conf

    earlier = parse_bigip_conf("ltm pool /Common/p { description first }\n")
    later = parse_bigip_conf("ltm pool /Common/p { description second }\n")
    merged = BigipConfig()
    merged.merge(earlier)
    merged.merge(later)
    assert merged.pools["/Common/p"].description == "second"


def test_review_bug6_query_projection_kinds_covered_by_object_registry():
    # Bug 6 — every TMSH kind the query projection exposes should be
    # *covered* by the deep-linking object registry, either as the
    # exact kind label or as a subtyped form.  The two layers
    # historically drifted because each was edited independently —
    # the projection collapses TMSH sub-kinds (``ltm profile tcp`` /
    # ``ltm profile http`` / …) to a single navigable family
    # (``ltm profile``), while the registry enumerates each subtype.
    #
    # This test enforces "either the projection's exact kind is in
    # the registry, or at least one ``<projection-kind> <subtype>``
    # is".  A failure means a contributor added a new kind to one
    # layer without the other — fix by adding the missing entry.
    from core.bigip.query.projection import MODULE_KINDS
    from core.bigip.registry.data import OBJECT_KIND_SPECS

    proj_kinds = {
        tmsh for module_table in MODULE_KINDS.values() for _attr, tmsh in module_table.values()
    }
    reg_kinds = {
        f"{spec.module} {object_type}"
        for spec in OBJECT_KIND_SPECS.values()
        for object_type in spec.object_types
    }

    # A projection kind is "covered" if the registry has either the
    # exact label or any ``<label> <subtype>`` entry.
    def _covered(kind: str) -> bool:
        if kind in reg_kinds:
            return True
        prefix = kind + " "
        return any(rk.startswith(prefix) for rk in reg_kinds)

    # Headline LTM kinds — these MUST be covered today.
    headline = {
        "ltm virtual",
        "ltm pool",
        "ltm node",
        "ltm rule",
        "ltm monitor",
        "ltm profile",
        "ltm persistence",
        "ltm snatpool",
        "ltm data-group",
        "ltm policy",
    }
    uncovered_headline = {k for k in headline if not _covered(k)}
    assert not uncovered_headline, (
        f"headline projection kinds not covered by the object registry: "
        f"{sorted(uncovered_headline)}.  Either add the kind to the "
        f"registry (``core/bigip/registry/specs/<kind>.py``) or fix "
        f"the projection's TMSH-form spelling."
    )

    # Lower-bar coverage for the wider projection: at least 50% of
    # projection kinds must be covered today.  This is a regression
    # tripwire — if a refactor accidentally drops registry coverage
    # below the current floor we want to know.
    covered_total = sum(1 for k in proj_kinds if _covered(k))
    coverage_ratio = covered_total / len(proj_kinds) if proj_kinds else 1.0
    assert coverage_ratio >= 0.5, (
        f"object-registry coverage of the query projection has dropped "
        f"to {coverage_ratio:.0%} (was at least 50%).  Recent changes "
        f"broke parity — add registry entries for the missing kinds."
    )


def test_review_jq1_select_drop_does_not_leak_into_scalar_pipe():
    # jq-semantics review note: ``select(false) | .field`` used to
    # error with "cannot read field" because the ``_DROP`` sentinel
    # leaked into the next pipe stage.  ``_pipe_through`` now treats
    # ``_DROP`` as an empty stream, matching jq's "empty output
    # short-circuits" semantics.
    src = "ltm node /Common/n { address 1.1.1.1 }\n"
    # Bare-LHS form — used to error.
    out = _run("select(false) | .ltm.node[].name", source=src).values_per_file["mem://1"]
    assert out == []
    # Scalar select chain — used to error when predicate failed.
    out = _run(
        '.ltm.node["/Common/n"] | select(.address == "none") | .name',
        source=src,
    ).values_per_file["mem://1"]
    assert out == []
    # Predicate-true case still works.
    out = _run(
        '.ltm.node["/Common/n"] | select(.address == "1.1.1.1") | .name',
        source=src,
    ).values_per_file["mem://1"]
    assert out == ["n"]


def test_review_jq2_map_filters_with_select_drops_zero_outputs():
    # jq-semantics review note: ``map`` is many-to-many via the
    # same ``_flatten`` machinery the pipe uses, so a body that
    # yields ``select(false)`` (the ``_DROP`` sentinel) contributes
    # zero outputs.  This is the canonical filter-and-transform
    # idiom and should now work end-to-end.
    src = "ltm node /Common/a { address 1.1.1.1 }\nltm node /Common/b { address 2.2.2.2 }\n"
    # Drop the node whose name doesn't equal "a"; project name on survivors.
    out = _run(
        '[.ltm.node[]] | map(select(.name == "a") | .name)',
        source=src,
    ).values_per_file["mem://1"]
    assert out == [["a"]]


# ---------------------------------------------------------------------------
# IP/port manipulation builtins backed by the typed value layer
# (``is_ipv4`` / ``is_ipv6`` / ``is_fqdn`` / ``is_private`` /
# ``is_loopback`` / ``is_unspecified`` / ``is_wildcard_port`` /
# ``prefix_length`` / ``subnet_of`` / ``overlaps`` / ``with_port`` /
# ``with_host`` / ``folder`` / ``with_folder``).
# ---------------------------------------------------------------------------


def test_typed_ip_builtins_classify_address_family():
    src = (
        "ltm virtual /Common/vs4 { destination /Common/10.0.0.1:80 }\n"
        "ltm virtual /Common/vs6 { destination /Common/[2001:db8::1].443 }\n"
        "ltm pool /Common/p1 {\n"
        "    members {\n"
        "        /Common/host.example.com:443 { address host.example.com }\n"
        "    }\n"
        "}\n"
    )
    # IPv4 / IPv6 partitioning.
    ipv4 = _run(
        "[.ltm.virtual[] | select(is_ipv4(.destination)) | .name]",
        source=src,
    ).values_per_file["mem://1"][0]
    ipv6 = _run(
        "[.ltm.virtual[] | select(is_ipv6(.destination)) | .name]",
        source=src,
    ).values_per_file["mem://1"][0]
    assert ipv4 == ["vs4"]
    assert ipv6 == ["vs6"]
    # FQDN pool members.
    [is_fqdn] = _run(
        'is_fqdn(.ltm.pool["/Common/p1"].members[].address)',
        source=src,
    ).values_per_file["mem://1"]
    assert is_fqdn is True


def test_typed_ip_classification_predicates():
    src = (
        "ltm virtual /Common/private_vs { destination /Common/10.0.0.1:80 }\n"
        "ltm virtual /Common/public_vs { destination /Common/8.8.8.8:80 }\n"
        "ltm virtual /Common/wildcard_vs { destination /Common/0.0.0.0:80 }\n"
        "ltm virtual /Common/loopback_vs { destination /Common/127.0.0.1:80 }\n"
    )
    [private] = _run(
        'is_private(.ltm.virtual["/Common/private_vs"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert private is True
    [public] = _run(
        'is_private(.ltm.virtual["/Common/public_vs"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert public is False
    [wildcard] = _run(
        'is_unspecified(.ltm.virtual["/Common/wildcard_vs"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert wildcard is True
    [loopback] = _run(
        'is_loopback(.ltm.virtual["/Common/loopback_vs"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert loopback is True


def test_typed_is_wildcard_port():
    src = (
        "ltm virtual /Common/any_port { destination /Common/0.0.0.0:any }\n"
        "ltm virtual /Common/http { destination /Common/10.0.0.1:80 }\n"
    )
    [any_port] = _run(
        'is_wildcard_port(.ltm.virtual["/Common/any_port"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert any_port is True
    [http_port] = _run(
        'is_wildcard_port(.ltm.virtual["/Common/http"].destination)',
        source=src,
    ).values_per_file["mem://1"]
    assert http_port is False


def test_typed_prefix_length():
    src = (
        "net self /Common/self_24 { address 10.0.0.1/24 vlan /Common/v }\n"
        "net self /Common/self_16 { address 10.0.0.1/16 vlan /Common/v }\n"
    )
    [twenty_four] = _run(
        'prefix_length(.net.self["/Common/self_24"].address)',
        source=src,
    ).values_per_file["mem://1"]
    [sixteen] = _run(
        'prefix_length(.net.self["/Common/self_16"].address)',
        source=src,
    ).values_per_file["mem://1"]
    assert twenty_four == 24
    assert sixteen == 16


def test_typed_subnet_of_and_overlaps():
    src = "net self /Common/s1 { address 10.0.0.1/24 vlan /Common/v }\n"
    [sub] = _run(
        'subnet_of("10.1.0.0/16", "10.0.0.0/8")',
        source=src,
    ).values_per_file["mem://1"]
    assert sub is True
    [not_sub] = _run(
        'subnet_of("10.0.0.0/8", "10.0.0.0/16")',
        source=src,
    ).values_per_file["mem://1"]
    assert not_sub is False
    # Different families never overlap.
    [no_family_overlap] = _run(
        'overlaps("10.0.0.0/24", "2001:db8::/32")',
        source=src,
    ).values_per_file["mem://1"]
    assert no_family_overlap is False


def test_typed_with_port_round_trips_through_destination_shapes():
    src = (
        "ltm virtual /Common/vs4 { destination /Common/10.0.0.1:80 }\n"
        "ltm virtual /Common/vs6 { destination /Common/[2001:db8::1].80 }\n"
    )
    [vs4] = _run(
        'with_port(.ltm.virtual["/Common/vs4"].destination, 443)',
        source=src,
    ).values_per_file["mem://1"]
    assert vs4 == "/Common/10.0.0.1:443"
    # IPv6 keeps its bracket form and ``.``-port separator.
    [vs6] = _run(
        'with_port(.ltm.virtual["/Common/vs6"].destination, 443)',
        source=src,
    ).values_per_file["mem://1"]
    assert vs6 == "/Common/[2001:db8::1].443"


def test_typed_with_host_switches_address_family():
    src = "ltm virtual /Common/vs1 { destination /Common/10.0.0.1:443 }\n"
    [swap] = _run(
        'with_host(.ltm.virtual["/Common/vs1"].destination, "2001:db8::1")',
        source=src,
    ).values_per_file["mem://1"]
    # Family switched to IPv6 — the canonical bare-IPv6 form drops
    # brackets and uses ``:`` port separator (IPv4 default).
    assert "2001:db8::1" in swap


def test_typed_folder_and_with_folder():
    src = "ltm pool /Common/web_pool { }\nltm pool /Common/iApps/Tenant.app/api_pool { }\n"
    [root_folder] = _run(
        'folder(.ltm.pool["/Common/web_pool"]."full-path")',
        source=src,
    ).values_per_file["mem://1"]
    assert root_folder == "/Common"
    [nested_folder] = _run(
        'folder(.ltm.pool["/Common/iApps/Tenant.app/api_pool"]."full-path")',
        source=src,
    ).values_per_file["mem://1"]
    assert nested_folder == "/Common/iApps/Tenant.app"
    # ``with_folder`` swaps the prefix; the leaf is preserved.
    [moved] = _run(
        'with_folder("/Common/web_pool", "/Tenant_A")',
        source=src,
    ).values_per_file["mem://1"]
    assert moved == "/Tenant_A/web_pool"
