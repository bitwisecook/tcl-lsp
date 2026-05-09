"""Tests for the M3 lint verbs: f5 validate and f5 irule lint."""

from __future__ import annotations

import json
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from explorer.f5_cli import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _write(tmp_path: Path, name: str, body: str) -> Path:
    path = tmp_path / name
    path.write_text(body)
    return path


# ── validate: config rules ───────────────────────────────────────────


def test_validate_clean_config_returns_zero(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { send "GET / HTTP/1.0" }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs_app {
            destination /Common/10.0.0.5:80
            pool /Common/p1
        }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p)], capsys)
    assert code == 0, out
    assert "no findings" in out


def test_validate_empty_pool_warns(tmp_path, capsys):
    body = "ltm pool /Common/p_empty { }\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p)], capsys)
    assert code == 1  # warning -> 1
    assert "empty-pool" in out
    assert "/Common/p_empty" in out


def test_validate_orphan_monitor(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm monitor http /Common/orphan_mon { send "GET /" }
        ltm pool /Common/p { members { /Common/n:80 { } } }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--category", "config"], capsys)
    assert code in (0, 1)  # /Common/ monitors are info-level, not warning
    assert "orphan_mon" in out


def test_validate_pool_without_monitor_warns(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm pool /Common/p { members { /Common/n:80 { } } }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p)], capsys)
    assert code == 1
    assert "pool-without-monitor" in out


def test_validate_virtual_without_pool_info(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm virtual /Common/vs_naked { destination /Common/10.0.0.1:80 }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p)], capsys)
    # info findings only -> exit 0
    assert code == 0
    assert "virtual-without-pool" in out


# ── validate: irule rules ────────────────────────────────────────────


def test_validate_irule_unknown_event(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm rule /Common/r_bad {
            when NOT_A_REAL_EVENT { return }
        }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--category", "irule"], capsys)
    assert code == 1
    assert "irule-unknown-event" in out
    assert "NOT_A_REAL_EVENT" in out


def test_validate_irule_empty_when_block_info(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm rule /Common/r {
            when HTTP_REQUEST { }
        }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--category", "irule"], capsys)
    assert code == 0  # info only
    assert "irule-empty-when" in out


# ── validate: output formats ─────────────────────────────────────────


def test_validate_json_format(tmp_path, capsys):
    body = "ltm pool /Common/empty { }\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--format", "json"], capsys)
    assert code == 1
    payload = json.loads(out)
    assert payload["summary"]["total"] >= 1
    assert any(f["ruleId"] == "empty-pool" for f in payload["findings"])


def test_validate_sarif_format(tmp_path, capsys):
    body = "ltm pool /Common/empty { }\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--format", "sarif"], capsys)
    assert code == 1
    payload = json.loads(out)
    assert payload["version"] == "2.1.0"
    assert payload["runs"][0]["tool"]["driver"]["name"] == "f5-validate"
    assert payload["runs"][0]["results"]


def test_validate_severity_filter(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm pool /Common/empty { }
        ltm virtual /Common/vs_naked { destination /Common/10.0.0.1:80 }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["validate", str(p), "--severity", "info", "--format", "json"], capsys)
    payload = json.loads(out)
    for f in payload["findings"]:
        assert f["severity"] == "info"


def test_validate_lint_alias(tmp_path, capsys):
    body = "ltm pool /Common/empty { }\n"
    p = _write(tmp_path, "c.conf", body)
    code, _out, _err = _run(["lint", str(p)], capsys)
    assert code == 1


# ── irule lint sub-verb ──────────────────────────────────────────────


def test_irule_lint_filters_to_irule_rules(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm pool /Common/empty { }
        ltm rule /Common/r {
            when HTTP_REQUEST { log_local0 "hi" }
        }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, _err = _run(["irule", "lint", str(p), "--json"], capsys)
    payload = json.loads(out)
    rule_ids = {f["ruleId"] for f in payload["findings"]}
    # empty-pool is config category — must be excluded
    assert "empty-pool" not in rule_ids
    # deprecated-command should fire on log_local0
    assert "irule-deprecated-command" in rule_ids


def test_irule_lint_clean_returns_zero(tmp_path, capsys):
    body = textwrap.dedent(
        """
        ltm rule /Common/r {
            when HTTP_REQUEST {
                pool /Common/p1
            }
        }
        """
    ).strip() + "\n"
    p = _write(tmp_path, "c.conf", body)
    code, _out, _err = _run(["irule", "lint", str(p)], capsys)
    assert code == 0
