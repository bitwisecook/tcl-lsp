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
