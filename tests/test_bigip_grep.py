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


def test_sample_bigip_conf_grep_returns_full_neighbourhood(tmp_path: Path) -> None:
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


def test_cli_grep_prints_text_report(tmp_path: Path, capsys, monkeypatch) -> None:
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
