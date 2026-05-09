"""Tests for BIG-IP related-object grep analysis."""

from __future__ import annotations

import io
import json
import sys
import textwrap
from pathlib import Path

import pytest

from core.bigip.grep import compute_grep, report_to_dict
from core.bigip.parser import parse_bigip_conf
from core.commands.registry.runtime import active_signature_profile
from explorer import f5_cli


def _run(source: str, pattern: str, **kwargs):
    cfg = parse_bigip_conf(source)
    uri = "mem://test.conf"
    return compute_grep(
        sources={uri: source},
        configs={uri: cfg},
        pattern=pattern,
        **kwargs,
    )


def _full_paths(items) -> list[str]:
    return [item.full_path for item in items]


def test_pattern_matches_seed_and_pulls_in_forward_neighbours() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { defaults-from /Common/http }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/p1", direction="forward")
    assert _full_paths(report.seeds) == ["/Common/p1"]
    assert set(_full_paths(report.related)) == {"/Common/n1", "/Common/m1"}


def test_reverse_direction_reaches_referrers_only() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { defaults-from /Common/http }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/n1", direction="reverse")
    assert _full_paths(report.seeds) == ["/Common/n1"]
    # Reverse from the node reaches the pool, then the virtual that uses the pool.
    related_paths = set(_full_paths(report.related))
    assert "/Common/p1" in related_paths
    assert "/Common/vs" in related_paths
    # The monitor isn't reachable from the node via reverse edges alone.
    assert "/Common/m1" not in related_paths


def test_both_direction_collects_full_neighbourhood() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { defaults-from /Common/http }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/p1", direction="both")
    related_paths = set(_full_paths(report.related))
    assert related_paths == {"/Common/n1", "/Common/m1", "/Common/vs"}


def test_substring_pattern_matches_multiple_seeds() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/web_pool { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm pool /Common/api_pool { members { /Common/n1:81 { address 10.0.0.1 } } }
        ltm pool /Common/db_pool { members { /Common/n1:82 { address 10.0.0.1 } } }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    report = _run(source, "_pool", direction="forward")
    assert set(_full_paths(report.seeds)) == {
        "/Common/web_pool",
        "/Common/api_pool",
        "/Common/db_pool",
    }


def test_regex_pattern_anchors_match() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/web_pool { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm pool /Common/api_pool { members { /Common/n1:81 { address 10.0.0.1 } } }
        ltm pool /Common/db_pool { members { /Common/n1:82 { address 10.0.0.1 } } }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    report = _run(source, r"^/Common/(web|api)_pool$", use_regex=True, direction="forward")
    assert set(_full_paths(report.seeds)) == {"/Common/web_pool", "/Common/api_pool"}


def test_no_match_yields_empty_report() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    report = _run(source, "nonexistent")
    assert report.seeds == ()
    assert report.related == ()
    assert "No objects match" in report.text_report


def test_max_depth_limits_bfs() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { defaults-from /Common/http }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    # depth-1 BFS from the virtual reaches the pool but not the node/monitor.
    report = _run(source, "/Common/vs", direction="forward", max_depth=1)
    assert _full_paths(report.related) == ["/Common/p1"]


def test_max_nodes_caps_result_size() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm pool /Common/p2 { members { /Common/n1:81 { address 10.0.0.1 } } }
        ltm pool /Common/p3 { members { /Common/n1:82 { address 10.0.0.1 } } }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    report = _run(source, "/Common/n1", direction="reverse", max_nodes=2)
    assert len(report.seeds) + len(report.related) <= 2


def test_irule_body_references_count_as_edges() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/used_pool {
            members { /Common/n1:80 { address 10.0.0.1 } }
        }
        ltm data-group internal /Common/used_dg {
            type string
            records { x { data y } }
        }
        ltm rule /Common/r1 {
        when HTTP_REQUEST {
            pool /Common/used_pool
            if { [class match [HTTP::host] equals /Common/used_dg] } { reject }
        }
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            rules { /Common/r1 }
        }
        """
    )
    report = _run(source, "/Common/r1", direction="forward")
    related = set(_full_paths(report.related))
    assert "/Common/used_pool" in related
    assert "/Common/used_dg" in related


def test_invalid_direction_raises() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError):
        _run(source, "vs", direction="sideways")


def test_invalid_regex_pattern_raises_value_error() -> None:
    """Bad regex must surface as a user error, not a silent zero-match result."""
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError, match="invalid regex pattern"):
        _run(source, "[unclosed", use_regex=True)


def test_max_nodes_below_one_raises() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError):
        _run(source, "vs", max_nodes=0)


def test_max_depth_negative_raises() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError):
        _run(source, "vs", max_depth=-1)


def test_max_nodes_truncates_seeds_when_pattern_matches_too_many() -> None:
    """When more seeds match than max_nodes allows, the result is capped strictly."""
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm pool /Common/p2 { members { /Common/n1:81 { address 10.0.0.1 } } }
        ltm pool /Common/p3 { members { /Common/n1:82 { address 10.0.0.1 } } }
        ltm pool /Common/p4 { members { /Common/n1:83 { address 10.0.0.1 } } }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    # Pattern matches every pool (4 seeds), but cap is 2.
    report = _run(source, "/Common/p", direction="forward", max_nodes=2)
    total = len(report.seeds) + len(report.related)
    assert total <= 2
    assert len(report.seeds) == 2


def test_compute_grep_restores_signature_profile() -> None:
    saved = active_signature_profile()
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    _run(source, "/Common/n1")
    assert active_signature_profile() == saved


def test_report_to_dict_round_trips_through_json() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/p1", direction="both")
    payload = report_to_dict(report)
    again = json.loads(json.dumps(payload))
    assert again["pattern"] == "/Common/p1"
    assert again["direction"] == "both"
    assert {item["fullPath"] for item in again["seeds"]} == {"/Common/p1"}
    assert {item["fullPath"] for item in again["related"]} >= {"/Common/n1", "/Common/vs"}
    assert isinstance(again["edges"], list)
    # By default, body is omitted to keep the JSON payload compact.
    assert "body" not in again["seeds"][0]


def test_report_to_dict_with_include_body_emits_object_bodies() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/p1", direction="both")
    payload = report_to_dict(report, include_body=True)
    assert "body" in payload["seeds"][0]
    assert payload["seeds"][0]["body"]
    for item in payload["related"]:
        assert "body" in item


def test_text_report_marks_seed_and_related_objects() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, "/Common/p1", direction="forward")
    text = report.text_report
    assert "* [ltm_pool] /Common/p1" in text
    assert "  [ltm_node] /Common/n1" in text


def test_include_body_emits_object_body_in_text_report() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, "/Common/n1", direction="forward", include_body=True)
    assert "address 10.0.0.1" in report.text_report


def test_sample_bigip_conf_grep_returns_full_neighbourhood() -> None:
    sample = Path(__file__).parent.parent / "samples" / "bigip" / "bigip.conf"
    src = sample.read_text(encoding="utf-8")
    cfg = parse_bigip_conf(src)
    report = compute_grep(
        sources={sample.as_uri(): src},
        configs={sample.as_uri(): cfg},
        pattern="/Common/web_pool",
        direction="both",
    )
    assert any(s.full_path == "/Common/web_pool" for s in report.seeds)
    related_paths = {r.full_path for r in report.related}
    # The web_pool's nodes and monitor are forward neighbours, and the virtuals
    # that use it are reverse neighbours.
    assert "/Common/web1" in related_paths
    assert "/Common/web2" in related_paths
    assert any("vs" in path for path in related_paths)


# CLI integration tests


def test_cli_grep_prints_text_report(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        textwrap.dedent(
            """\
            ltm node /Common/n1 { address 10.0.0.1 }
            ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
            ltm virtual /Common/vs {
                destination /Common/10.0.0.10:80
                pool /Common/p1
            }
            """
        ),
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "/Common/p1", str(conf)])
    captured = capsys.readouterr()
    assert rc == 0
    assert "Pattern: /Common/p1" in captured.out
    assert "/Common/n1" in captured.out
    assert "/Common/vs" in captured.out


def test_cli_grep_no_match_returns_exit_code_one(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n",
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "no_such_object", str(conf)])
    captured = capsys.readouterr()
    assert rc == 1
    assert "No objects match pattern: no_such_object" in captured.out


def test_cli_grep_json_emits_valid_json(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        textwrap.dedent(
            """\
            ltm node /Common/n1 { address 10.0.0.1 }
            ltm virtual /Common/vs {
                destination /Common/10.0.0.10:80
            }
            """
        ),
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "--json", "/Common/n1", str(conf)])
    captured = capsys.readouterr()
    assert rc == 0
    data = json.loads(captured.out)
    assert data["pattern"] == "/Common/n1"
    assert data["direction"] == "both"
    assert data["seeds"][0]["fullPath"] == "/Common/n1"


def test_cli_grep_writes_to_output_file(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        textwrap.dedent(
            """\
            ltm node /Common/n1 { address 10.0.0.1 }
            ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
            """
        ),
        encoding="utf-8",
    )
    out = tmp_path / "report.txt"
    rc = f5_cli.main(["grep", "/Common/n1", str(conf), "-o", str(out)])
    assert rc == 0
    body = out.read_text(encoding="utf-8")
    assert "Pattern: /Common/n1" in body


def test_cli_grep_reads_stdin_with_dash(monkeypatch, capsys) -> None:
    src = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }
        """
    )
    monkeypatch.setattr(sys, "stdin", io.StringIO(src))
    rc = f5_cli.main(["grep", "/Common/n1", "-"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "/Common/n1" in captured.out


def test_cli_grep_alias_related_works(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n",
        encoding="utf-8",
    )
    rc = f5_cli.main(["related", "/Common/vs", str(conf)])
    captured = capsys.readouterr()
    assert rc == 0
    assert "/Common/vs" in captured.out


def test_cli_grep_invalid_regex_reports_error(tmp_path: Path, capsys) -> None:
    """A bad regex must fail loudly with a non-zero exit code."""
    conf = tmp_path / "x.conf"
    conf.write_text(
        "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n",
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "--regex", "[unclosed", str(conf)])
    captured = capsys.readouterr()
    assert rc == 2
    assert "invalid regex pattern" in captured.err


# CIDR mode


def test_cidr_matches_address_in_object_header() -> None:
    """A virtual whose destination falls inside the seed CIDR is a seed."""
    source = textwrap.dedent(
        """\
        ltm virtual /Common/vs_inside { destination /Common/10.5.5.10:80 }
        ltm virtual /Common/vs_outside { destination /Common/192.168.1.1:80 }
        """
    )
    report = _run(source, "10.0.0.0/8", use_cidr=True, direction="forward")
    paths = set(_full_paths(report.seeds))
    assert "/Common/vs_inside" in paths
    assert "/Common/vs_outside" not in paths


def test_cidr_matches_inside_irule_body() -> None:
    """An IP literal buried in an iRule script body is matched."""
    source = textwrap.dedent(
        """\
        ltm rule /Common/r_block {
        when HTTP_REQUEST {
            if { [IP::addr [IP::client_addr] equals "10.0.0.5"] } { reject }
        }
        }
        ltm rule /Common/r_other {
        when HTTP_REQUEST {
            if { [IP::addr [IP::client_addr] equals "192.168.1.1"] } { reject }
        }
        }
        """
    )
    report = _run(source, "10.0.0.0/8", use_cidr=True, direction="forward")
    paths = set(_full_paths(report.seeds))
    assert "/Common/r_block" in paths
    assert "/Common/r_other" not in paths


def test_cidr_matches_cidr_literal_inside_irule_body() -> None:
    """A `/prefix` literal inside an iRule body is matched when it overlaps."""
    source = textwrap.dedent(
        """\
        ltm rule /Common/r_subnet {
        when HTTP_REQUEST {
            if { [matchclass [IP::client_addr] equals "10.10.0.0/16"] } { reject }
        }
        }
        """
    )
    report = _run(source, "10.0.0.0/8", use_cidr=True, direction="forward")
    assert _full_paths(report.seeds) == ["/Common/r_subnet"]


def test_cidr_host_address_seed_matches_containing_network() -> None:
    """A bare host seed matches an object that mentions a containing CIDR."""
    source = textwrap.dedent(
        """\
        ltm data-group internal /Common/dg_block {
            type ip
            records {
                10.0.0.0/8 { }
            }
        }
        """
    )
    report = _run(source, "10.5.5.5", use_cidr=True, direction="forward")
    assert _full_paths(report.seeds) == ["/Common/dg_block"]


def test_cidr_multiple_networks_are_unioned() -> None:
    """Whitespace- and comma-separated CIDRs both seed the search."""
    source = textwrap.dedent(
        """\
        ltm virtual /Common/vs_a { destination /Common/10.0.0.1:80 }
        ltm virtual /Common/vs_b { destination /Common/192.168.1.1:80 }
        ltm virtual /Common/vs_c { destination /Common/172.20.5.10:80 }
        """
    )
    report = _run(source, "10.0.0.0/8, 192.168.0.0/16", use_cidr=True, direction="forward")
    paths = set(_full_paths(report.seeds))
    assert paths == {"/Common/vs_a", "/Common/vs_b"}


def test_cidr_matches_ipv6_in_irule() -> None:
    """IPv6 CIDR seeds match IPv6 literals inside iRule bodies."""
    source = textwrap.dedent(
        """\
        ltm rule /Common/r_v6 {
        when HTTP_REQUEST {
            if { [IP::addr [IP::client_addr] equals "2001:db8::dead"] } { reject }
        }
        }
        ltm rule /Common/r_v4_only {
        when HTTP_REQUEST {
            if { [IP::addr [IP::client_addr] equals "10.0.0.1"] } { reject }
        }
        }
        """
    )
    report = _run(source, "2001:db8::/32", use_cidr=True, direction="forward")
    paths = set(_full_paths(report.seeds))
    assert "/Common/r_v6" in paths
    assert "/Common/r_v4_only" not in paths


def test_cidr_pulls_in_referenced_neighbours() -> None:
    """CIDR seeds still expand to forward neighbours through reference edges."""
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
        }
        ltm virtual /Common/vs_inside {
            destination /Common/10.5.5.10:80
            pool /Common/p1
        }
        ltm virtual /Common/vs_outside {
            destination /Common/192.168.1.1:80
        }
        """
    )
    report = _run(source, "10.5.5.0/24", use_cidr=True, direction="forward")
    assert _full_paths(report.seeds) == ["/Common/vs_inside"]
    related = set(_full_paths(report.related))
    # The pool (and through it the node) must be reachable from the seed.
    assert "/Common/p1" in related
    assert "/Common/n1" in related


def test_cidr_invalid_pattern_raises_value_error() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError, match="invalid CIDR pattern"):
        _run(source, "999.999.999.999/8", use_cidr=True)


def test_cidr_empty_pattern_raises_value_error() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError, match="at least one IP"):
        _run(source, "   ", use_cidr=True)


def test_cidr_and_regex_are_mutually_exclusive() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    with pytest.raises(ValueError, match="mutually exclusive"):
        _run(source, "10.0.0.0/8", use_cidr=True, use_regex=True)


def test_cidr_text_report_marks_pattern_as_cidr() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    report = _run(source, "10.0.0.0/8", use_cidr=True)
    assert "Pattern: 10.0.0.0/8 (cidr)" in report.text_report


def test_cidr_report_to_dict_round_trips_use_cidr_flag() -> None:
    source = "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n"
    report = _run(source, "10.0.0.0/8", use_cidr=True)
    payload = report_to_dict(report)
    again = json.loads(json.dumps(payload))
    assert again["useCidr"] is True
    assert again["useRegex"] is False
    assert {item["fullPath"] for item in again["seeds"]} == {"/Common/vs"}


def test_cli_grep_cidr_finds_irule_match(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        textwrap.dedent(
            """\
            ltm rule /Common/r_block {
            when HTTP_REQUEST {
                if { [IP::addr [IP::client_addr] equals "10.0.0.5"] } { reject }
            }
            }
            ltm virtual /Common/vs_outside { destination /Common/192.168.1.1:80 }
            """
        ),
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "--cidr", "10.0.0.0/8", str(conf)])
    captured = capsys.readouterr()
    assert rc == 0
    assert "Pattern: 10.0.0.0/8 (cidr)" in captured.out
    assert "/Common/r_block" in captured.out
    assert "/Common/vs_outside" not in captured.out


def test_cli_grep_cidr_invalid_pattern_reports_error(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n",
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "--cidr", "999.999.999.999/8", str(conf)])
    captured = capsys.readouterr()
    assert rc == 2
    assert "invalid CIDR pattern" in captured.err


def test_cli_grep_cidr_and_regex_are_rejected(tmp_path: Path, capsys) -> None:
    conf = tmp_path / "x.conf"
    conf.write_text(
        "ltm virtual /Common/vs { destination /Common/10.0.0.10:80 }\n",
        encoding="utf-8",
    )
    with pytest.raises(SystemExit):
        f5_cli.main(["grep", "--cidr", "--regex", "10.0.0.0/8", str(conf)])
    captured = capsys.readouterr()
    assert "not allowed with argument" in captured.err or "mutually exclusive" in captured.err


def test_cli_grep_full_json_emits_object_bodies(tmp_path: Path, capsys) -> None:
    """`--full` with `--json` must include each object's body in the payload."""
    conf = tmp_path / "x.conf"
    conf.write_text(
        textwrap.dedent(
            """\
            ltm node /Common/n1 { address 10.0.0.1 }
            ltm virtual /Common/vs {
                destination /Common/10.0.0.10:80
            }
            """
        ),
        encoding="utf-8",
    )
    rc = f5_cli.main(["grep", "--json", "--full", "/Common/n1", str(conf)])
    captured = capsys.readouterr()
    assert rc == 0
    data = json.loads(captured.out)
    assert data["seeds"][0]["fullPath"] == "/Common/n1"
    assert "body" in data["seeds"][0]
    assert "address 10.0.0.1" in data["seeds"][0]["body"]
