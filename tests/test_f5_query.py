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
