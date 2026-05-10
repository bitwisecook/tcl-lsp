"""Tests for ``f5 irule format`` and ``f5 irule minify`` sub-verbs."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from explorer.f5_cli import main
from explorer.f5_remote.ucs import make_test_ucs


def _run(args: list[str], capsys) -> tuple[int, str, str]:
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


# ---------------------------------------------------------------------------
# format
# ---------------------------------------------------------------------------


def test_irule_format_inline_source_to_stdout(capsys):
    code, out, err = _run(
        ["irule", "format", "--source", "when HTTP_REQUEST {return}"],
        capsys,
    )
    assert code == 0, err
    assert "when HTTP_REQUEST" in out
    # Formatter should emit a final newline.
    assert out.endswith("\n")


def test_irule_format_irule_file_to_stdout(tmp_path, capsys):
    rule = tmp_path / "rule.irule"
    rule.write_text("when HTTP_REQUEST {return}\n", encoding="utf-8")

    code, out, err = _run(["irule", "format", str(rule)], capsys)
    assert code == 0, err
    assert "when HTTP_REQUEST" in out


def test_irule_format_bigip_conf_emits_directory(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm rule /Common/foo {when HTTP_REQUEST {return}}\n"
        "ltm rule /Partition_a/bar {when HTTP_RESPONSE {return}}\n",
        encoding="utf-8",
    )
    out_dir = tmp_path / "out"
    out_dir.mkdir()

    code, _out, err = _run(["irule", "format", str(conf), "-o", str(out_dir)], capsys)
    assert code == 0, err
    files = sorted(p.name for p in out_dir.iterdir())
    assert files == ["Common__foo.irule", "Partition_a__bar.irule"]


def test_irule_format_multiple_rules_concat_to_stdout(tmp_path, capsys):
    """Multi-rule input + scalar -o (or stdout) emits a single
    concatenated stream with per-rule banner headers."""
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm rule /Common/foo {when HTTP_REQUEST {return}}\n"
        "ltm rule /Common/bar {when HTTP_RESPONSE {return}}\n",
        encoding="utf-8",
    )

    code, out, err = _run(["irule", "format", str(conf)], capsys)
    assert code == 0, err
    assert "# ===== /Common/foo =====" in out
    assert "# ===== /Common/bar =====" in out


def test_irule_format_multiple_rules_to_single_file(tmp_path, capsys):
    """Passing a non-directory ``-o FILE`` for multi-rule input writes
    the concatenated stream to that file (no error)."""
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm rule /Common/foo {when HTTP_REQUEST {return}}\n"
        "ltm rule /Common/bar {when HTTP_RESPONSE {return}}\n",
        encoding="utf-8",
    )
    target = tmp_path / "all.irule"

    code, _out, err = _run(["irule", "format", str(conf), "-o", str(target)], capsys)
    assert code == 0, err
    text = target.read_text(encoding="utf-8")
    assert "# ===== /Common/foo =====" in text
    assert "# ===== /Common/bar =====" in text


def test_irule_format_directory_target_uses_trailing_slash(tmp_path, capsys):
    """A path with a trailing separator is treated as a directory even
    if it doesn't yet exist."""
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm rule /Common/foo {when HTTP_REQUEST {return}}\n"
        "ltm rule /Common/bar {when HTTP_RESPONSE {return}}\n",
        encoding="utf-8",
    )
    out_dir_arg = str(tmp_path / "fmt") + "/"

    code, _out, err = _run(["irule", "format", str(conf), "-o", out_dir_arg], capsys)
    assert code == 0, err
    out_dir = tmp_path / "fmt"
    assert (out_dir / "Common__foo.irule").exists()
    assert (out_dir / "Common__bar.irule").exists()


# ---------------------------------------------------------------------------
# minify
# ---------------------------------------------------------------------------


def test_irule_minify_inline_source(capsys):
    code, out, err = _run(
        ["irule", "minify", "--source", "when HTTP_REQUEST { # hi\n  return\n}"],
        capsys,
    )
    assert code == 0, err
    # Comments should be stripped.
    assert "# hi" not in out
    assert "return" in out


def test_irule_minify_compact_writes_symbol_map(tmp_path, capsys):
    rule = tmp_path / "rule.irule"
    # --isolated is required to compact global-scope vars (iRules
    # event handlers are self-contained, so this is the typical mode).
    rule.write_text("when HTTP_REQUEST { set my_var 1; log local0. $my_var }\n", encoding="utf-8")
    sym_map = tmp_path / "syms.txt"

    code, _out, err = _run(
        [
            "irule",
            "minify",
            "--compact",
            "--isolated",
            str(rule),
            "--symbol-map",
            str(sym_map),
        ],
        capsys,
    )
    assert code == 0, err
    text = sym_map.read_text(encoding="utf-8")
    assert "my_var" in text


def test_irule_minify_aggressive_runs(tmp_path, capsys):
    rule = tmp_path / "r.irule"
    rule.write_text(
        "when HTTP_REQUEST {\n  set x 1\n  set y 2\n  log local0. $x\n}\n",
        encoding="utf-8",
    )
    code, out, err = _run(["irule", "minify", "--aggressive", str(rule)], capsys)
    assert code == 0, err
    assert "log" in out


def test_irule_minify_bigip_conf_to_directory(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm rule /Common/foo { when HTTP_REQUEST { return } }\n"
        "ltm rule /Common/bar { when HTTP_RESPONSE { return } }\n",
        encoding="utf-8",
    )
    out_dir = tmp_path / "min"
    out_dir.mkdir()
    code, _out, err = _run(["irule", "minify", str(conf), "-o", str(out_dir)], capsys)
    assert code == 0, err
    files = sorted(p.name for p in out_dir.iterdir())
    assert files == ["Common__bar.irule", "Common__foo.irule"]


def test_irule_minify_ucs_input(tmp_path, capsys):
    ucs = tmp_path / "device.ucs"
    ucs.write_bytes(
        make_test_ucs(
            {
                "config/bigip.conf": (
                    "ltm rule /Common/foo { when HTTP_REQUEST { return } }\n"
                    "ltm rule /Common/bar { when HTTP_RESPONSE { return } }\n"
                ),
            }
        )
    )
    out_dir = tmp_path / "out"
    out_dir.mkdir()

    code, _out, err = _run(["irule", "minify", str(ucs), "-o", str(out_dir)], capsys)
    assert code == 0, err
    assert (out_dir / "Common__foo.irule").exists()
    assert (out_dir / "Common__bar.irule").exists()


# ---------------------------------------------------------------------------
# context (smoke through the CLI)
# ---------------------------------------------------------------------------


def test_irule_context_text_smoke(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm pool /Common/web_pool { members { /Common/n1:80 { } } }\n"
        "ltm rule /Common/foo {\n"
        "    when HTTP_REQUEST {\n"
        "        pool /Common/web_pool\n"
        "        pool /Common/missing\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )
    code, out, err = _run(["irule", "context", str(conf)], capsys)
    assert code == 0, err
    assert "iRule /Common/foo" in out
    assert "referenced pool /Common/web_pool" in out
    assert "unresolved" in out
    assert "pool: /Common/missing" in out


def test_irule_context_json_smoke(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm pool /Common/web_pool { members { /Common/n1:80 { } } }\n"
        "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/web_pool } }\n",
        encoding="utf-8",
    )
    code, out, err = _run(["irule", "context", str(conf), "--json"], capsys)
    assert code == 0, err
    payload = json.loads(out)
    # Single-file JSON output wraps every rule in a ``bundles`` array
    # for consistency between single- and multi-rule input.
    assert len(payload["bundles"]) == 1
    bundle = payload["bundles"][0]
    assert bundle["rule"]["fullPath"] == "/Common/foo"
    assert any(p["fullPath"] == "/Common/web_pool" for p in bundle["pools"])


# ---------------------------------------------------------------------------
# trace references enrichment
# ---------------------------------------------------------------------------


def test_irule_trace_emits_references_array(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    conf.write_text(
        "ltm pool /Common/web_pool { members { /Common/n1:80 { } } }\n"
        "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/web_pool } }\n"
        "ltm rule /Common/bar { when HTTP_REQUEST { pool /Common/missing } }\n",
        encoding="utf-8",
    )
    code, out, err = _run(
        ["irule", "trace", "HTTP_REQUEST", str(conf), "--json"],
        capsys,
    )
    assert code == 0, err
    payload = json.loads(out)
    by_rule = {t["rule"]: t for t in payload["traces"]}
    assert by_rule["/Common/foo"]["references"][0] == {
        "kind": "pool",
        "name": "/Common/web_pool",
        "command": "pool",
        "resolved": True,
        "resolvedPath": "/Common/web_pool",
    }
    assert by_rule["/Common/bar"]["references"][0]["resolved"] is False


# ---------------------------------------------------------------------------
# lint missing-object findings
# ---------------------------------------------------------------------------


def test_irule_lint_emits_missing_pool(tmp_path, capsys):
    conf = tmp_path / "bigip.conf"
    # At least one pool exists so `_has_config_context` returns True.
    conf.write_text(
        "ltm pool /Common/real { members { /Common/n1:80 { } } }\n"
        "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/missing } }\n",
        encoding="utf-8",
    )
    code, out, _err = _run(["irule", "lint", str(conf), "--json"], capsys)
    # Warning → exit code 1.
    assert code == 1
    payload = json.loads(out)
    rule_ids = {f["ruleId"] for f in payload["findings"]}
    assert "irule-missing-pool" in rule_ids


def test_irule_extract_rejects_standalone_irule_files(tmp_path, capsys):
    """``f5 irule extract`` is documented as accepting bigip.conf /
    SCF / UCS only.  A standalone .irule path is meaningless (the body
    is already in the desired form) and must be refused."""
    rule = tmp_path / "x.irule"
    rule.write_text("when HTTP_REQUEST { return }\n", encoding="utf-8")
    out_dir = tmp_path / "out"

    code, _out, err = _run(["irule", "extract", str(rule), str(out_dir)], capsys)
    assert code == 2
    assert "standalone" in err
    assert "x.irule" in err


def test_irule_lint_skips_missing_object_for_standalone_irule(tmp_path, capsys):
    rule = tmp_path / "r.irule"
    rule.write_text("when HTTP_REQUEST { pool /Common/missing }\n", encoding="utf-8")
    code, _out, err = _run(["irule", "lint", str(rule)], capsys)
    # No surrounding context — missing-object check should be silent.
    assert code == 0, err
