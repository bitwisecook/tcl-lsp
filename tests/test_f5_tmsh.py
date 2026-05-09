"""Tests for ``f5 tmsh`` (alias scf2tmsh)."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.parser import parse_bigip_conf
from core.bigip.tmsh_emit import emit_tmsh
from explorer.f5_cli import main

SAMPLE = (
    textwrap.dedent(
        """
        auth partition /Common { }
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm node /Common/n2 { address 10.0.0.2 }
        ltm monitor http /Common/m_http {
            send "GET / HTTP/1.0"
            recv "200 OK"
        }
        ltm pool /Common/p1 {
            members {
                /Common/n1:80 { address 10.0.0.1 }
                /Common/n2:80 { address 10.0.0.2 }
            }
            load-balancing-mode least-connections-member
            monitor /Common/m_http
        }
        ltm rule /Common/r_irule {
            when HTTP_REQUEST {
                pool /Common/p1
            }
        }
        ltm virtual /Common/vs_app {
            destination /Common/10.0.0.5:80
            pool /Common/p1
            rules { /Common/r_irule }
        }
        """
    ).strip()
    + "\n"
)


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _write(tmp_path, name, body):
    path = tmp_path / name
    path.write_text(body)
    return path


# ── core ────────────────────────────────────────────────────────────


def test_emit_tmsh_orders_dependencies_correctly():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE)
    text = script.text
    # Required ordering: nodes before pools, pools before virtuals,
    # rules before virtuals, monitors before pools.
    pos_node = text.index("ltm node /Common/n1")
    pos_monitor = text.index("ltm monitor")
    pos_pool = text.index("ltm pool /Common/p1")
    pos_rule = text.index("ltm rule /Common/r_irule")
    pos_virtual = text.index("ltm virtual /Common/vs_app")
    assert pos_node < pos_pool
    assert pos_monitor < pos_pool
    assert pos_pool < pos_virtual
    assert pos_rule < pos_virtual


def test_emit_tmsh_uses_create_by_default():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE)
    assert "tmsh create" in script.text
    assert "tmsh modify" not in script.text


def test_emit_tmsh_modify_swaps_verb():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE, use_modify_for_existing=True)
    assert "tmsh modify" in script.text
    assert "tmsh create" not in script.text


def test_emit_tmsh_includes_pool_members_inline():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE)
    pool_line = [line for line in script.text.splitlines() if "ltm pool /Common/p1" in line][0]
    assert "/Common/n1:80" in pool_line
    assert "/Common/n2:80" in pool_line
    assert "least-connections-member" in pool_line
    assert "monitor /Common/m_http" in pool_line


def test_emit_tmsh_rule_body_preserved_verbatim():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE)
    # The rule body must contain the original Tcl verbatim.
    assert "when HTTP_REQUEST" in script.text
    assert "pool /Common/p1" in script.text


def test_emit_tmsh_include_filter_narrows_output():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE, include=("pool",))
    assert "ltm pool" in script.text
    assert "ltm virtual" not in script.text
    assert "ltm node" not in script.text


def test_emit_tmsh_command_count_matches_lines():
    cfg = parse_bigip_conf(SAMPLE)
    script = emit_tmsh(cfg, source=SAMPLE)
    # Each command starts with "tmsh ".
    cmd_starts = [line for line in script.text.splitlines() if line.startswith("tmsh ")]
    assert len(cmd_starts) == script.command_count


# ── verb ────────────────────────────────────────────────────────────


def test_tmsh_verb_text_output(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, out, _err = _run(["tmsh", str(p)], capsys)
    assert code == 0
    assert "tmsh create ltm pool /Common/p1" in out
    assert "tmsh create ltm virtual /Common/vs_app" in out


def test_tmsh_verb_modify_flag(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, out, _err = _run(["tmsh", str(p), "--modify"], capsys)
    assert code == 0
    assert "tmsh modify" in out


def test_tmsh_verb_include_filter(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, out, _err = _run(["tmsh", str(p), "--include", "pool"], capsys)
    assert code == 0
    assert "ltm pool" in out
    assert "ltm virtual" not in out


def test_tmsh_verb_invalid_include(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, _out, err = _run(["tmsh", str(p), "--include", "bogus"], capsys)
    assert code == 2
    assert "unknown kind" in err


def test_tmsh_alias_scf2tmsh(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, _out, _err = _run(["scf2tmsh", str(p)], capsys)
    assert code == 0
