"""Tests for the unified verb-based Tcl CLI."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from explorer import tcl_cli
from explorer.tcl_cli import main


def _run(args: list[str], capsys) -> tuple[int, str, str]:
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def test_opt_writes_optimised_output(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set a 1\nset b [expr {$a + 2}]\n")
    output = tmp_path / "optimised.tcl"

    code, out, err = _run(["opt", str(script), "-o", str(output)], capsys)

    assert code == 0
    assert out == ""
    assert "rewrites=" in err
    assert output.read_text().strip() == "set b 3"


@pytest.mark.parametrize("verb", ["diag", "lint"])
def test_diag_like_verbs_report_warnings_from_directory(verb, tmp_path, capsys):
    source_dir = tmp_path / "src"
    source_dir.mkdir()
    bad_script = source_dir / "bad.tcl"
    bad_script.write_text("string frobulate x\n")

    code, out, err = _run([verb, str(source_dir)], capsys)

    assert code == 1
    assert str(bad_script) in out
    assert "W001" in out
    assert "diagnostics=" in err


def test_validate_returns_nonzero_on_errors(tmp_path, capsys):
    script = tmp_path / "broken.tcl"
    script.write_text("set x [expr {1 + 2}\n")

    code, out, err = _run(["validate", str(script)], capsys)

    assert code == 1
    assert "E201" in out
    assert "validation failed" in err


def test_validate_json_returns_structured_errors(tmp_path, capsys):
    script = tmp_path / "broken.tcl"
    script.write_text("set x [expr {1 + 2}\n")

    code, out, err = _run(["validate", "--json", str(script)], capsys)

    assert code == 1
    payload = json.loads(out)
    assert payload["ok"] is False
    assert payload["error_count"] >= 1
    assert payload["errors"][0]["code"] == "E201"
    assert err == ""


def test_format_rewrites_source_with_formatter(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("proc p {} {set x 1;set y 2}\n")

    code, out, err = _run(["format", str(script)], capsys)

    assert code == 0
    assert out == "proc p {} {\n    set x 1\n    set y 2\n}\n"
    assert err == ""


def test_minify_rewrites_source_with_minifier(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("# note\nset x 1\nset y 2\n")

    code, out, err = _run(["minify", str(script)], capsys)

    assert code == 0
    assert out == "set x 1;set y 2\n"
    assert err == ""


def test_minify_aggressive_writes_symbol_map(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("proc calculate {alpha beta} {\n    return [expr {$alpha + $beta}]\n}\n")
    output = tmp_path / "minified.tcl"
    symbol_map = tmp_path / "symbols.txt"

    code, out, err = _run(
        [
            "minify",
            "--aggressive",
            "--symbol-map",
            str(symbol_map),
            str(script),
            "-o",
            str(output),
        ],
        capsys,
    )

    assert code == 0
    assert out == ""
    assert output.exists()
    assert symbol_map.exists()
    assert "# Procs" in symbol_map.read_text()
    assert err == ""


def test_unminify_error_translates_with_symbol_map_file(tmp_path, capsys):
    map_file = tmp_path / "map.txt"
    map_file.write_text("# Variables in ::demo\n  a <- request_uri\n")

    code, out, err = _run(
        [
            "unminify-error",
            "--symbol-map",
            str(map_file),
            "--error",
            'can\'t read "a": no such variable',
        ],
        capsys,
    )

    assert code == 0
    assert 'can\'t read "request_uri": no such variable' in out
    assert err == ""


def test_symbols_json_reports_symbol_entries(tmp_path, capsys):
    script = tmp_path / "symbols.tcl"
    script.write_text("namespace eval demo { proc greet {name} { puts $name } }\n")

    code, out, err = _run(["symbols", "--json", str(script)], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["count"] >= 1
    assert any(
        item["kind"] == "function" and item["name"] == "greet" for item in payload["symbols"]
    )
    assert err == ""


def test_diagram_json_reports_event_flow(capsys):
    source = "when HTTP_REQUEST { return }\n"

    code, out, err = _run(
        ["diagram", "--dialect", "f5-irules", "--source", source, "--json"], capsys
    )

    assert code == 0
    payload = json.loads(out)
    events = payload.get("events", [])
    assert any(item.get("name") == "HTTP_REQUEST" for item in events)
    assert err == ""


def test_callgraph_json_reports_proc_edges(capsys):
    source = "proc alpha {} { beta }\nproc beta {} { return }\n"

    code, out, err = _run(["callgraph", "--source", source, "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    names = {item["name"] for item in payload["nodes"]}
    edges = {(item["caller"], item["callee"]) for item in payload["edges"]}
    assert "::alpha" in names
    assert "::beta" in names
    assert ("::alpha", "::beta") in edges
    assert err == ""


def test_symbolgraph_json_reports_summary(capsys):
    source = "namespace eval demo { proc greet {name} { puts $name } }\n"

    code, out, err = _run(["symbolgraph", "--source", source, "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["summary"]["total_procs"] >= 1
    assert err == ""


def test_dataflow_json_reports_summary(capsys):
    source = "proc greet {name} { puts $name }\n"

    code, out, err = _run(["dataflow", "--source", source, "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert "summary" in payload
    assert "total_taint_warnings" in payload["summary"]
    assert err == ""


def test_event_order_json_reports_events(capsys):
    source = "when HTTP_RESPONSE { return }\nwhen HTTP_REQUEST { return }\n"

    code, out, err = _run(
        ["event-order", "--dialect", "f5-irules", "--source", source, "--json"],
        capsys,
    )

    assert code == 0
    payload = json.loads(out)
    assert payload["count"] == 2
    names = [item["name"] for item in payload["events"]]
    assert "HTTP_REQUEST" in names
    assert "HTTP_RESPONSE" in names
    assert err == ""


def test_event_info_json_known_event(capsys):
    code, out, err = _run(["event-info", "HTTP_REQUEST", "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["event"] == "HTTP_REQUEST"
    assert payload["known"] is True
    assert payload["validCommandCount"] >= 1
    assert err == ""


def test_command_info_json_known_command(capsys):
    code, out, err = _run(
        ["command-info", "HTTP::uri", "--dialect", "f5-irules", "--json"],
        capsys,
    )

    assert code == 0
    payload = json.loads(out)
    assert payload["found"] is True
    assert payload["command"] == "HTTP::uri"
    assert err == ""


def test_convert_json_reports_convertible_diagnostics(capsys):
    source = "set x [expr $y + 1]\n"

    code, out, err = _run(["convert", "--source", source, "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["count"] >= 1
    assert any(item["code"] == "W100" for item in payload["issues"])
    assert err == ""


def test_dis_outputs_bytecode_listing(tmp_path, capsys):
    script = tmp_path / "tiny.tcl"
    script.write_text("set x 1\n")

    code, out, _err = _run(["dis", str(script)], capsys)

    assert code == 0
    assert "ByteCode ::top" in out
    assert "done" in out


def test_compwasm_writes_binary_file(tmp_path, capsys):
    script = tmp_path / "tiny.tcl"
    script.write_text("set x 1\n")
    output = tmp_path / "out.wasm"

    code, out, err = _run(["compwasm", str(script), "-o", str(output)], capsys)

    assert code == 0
    assert out == ""
    assert "wrote wasm binary" in err
    assert output.read_bytes().startswith(b"\x00asm")


def test_compwasm_defaults_to_out_wasm(tmp_path, capsys, monkeypatch):
    script = tmp_path / "tiny.tcl"
    script.write_text("set x 1\n")
    monkeypatch.chdir(tmp_path)

    code, out, err = _run(["compwasm", str(script)], capsys)

    assert code == 0
    assert out == ""
    assert "out.wasm" in err
    assert (tmp_path / "out.wasm").read_bytes().startswith(b"\x00asm")


def test_highlight_force_colour_emits_ansi_codes(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set x 1\nputs $x\n")

    code, out, err = _run(["highlight", str(script), "--colour"], capsys)

    assert code == 0
    assert "\x1b[" in out
    assert "set" in out
    assert err == ""


def test_highlight_no_colour_outputs_plain_source(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    source = "set x 1\nputs $x\n"
    script.write_text(source)

    code, out, err = _run(["highlight", str(script), "--no-colour"], capsys)

    assert code == 0
    assert out == source
    assert err == ""


def test_highlight_html_output_wraps_in_pre(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set x 1\nputs $x\n")

    code, out, err = _run(["highlight", str(script), "--format", "html"], capsys)

    assert code == 0
    assert out.startswith("<pre>\n")
    assert out.endswith("</pre>\n")
    assert "<span style=" in out
    assert err == ""


def test_opt_resolves_package_name_inputs(tmp_path, capsys):
    package_dir = tmp_path / "demo_pkg"
    package_dir.mkdir()
    (package_dir / "pkgIndex.tcl").write_text(
        'package ifneeded demo 1.0 "source [file join $dir demo.tcl]"\n'
    )
    (package_dir / "demo.tcl").write_text("set z [expr {1 + 2}]\n")
    output = tmp_path / "package_optimised.tcl"

    code, _out, _err = _run(
        [
            "opt",
            "demo",
            "--package-path",
            str(package_dir),
            "-o",
            str(output),
        ],
        capsys,
    )

    assert code == 0
    assert output.read_text().strip() == "set z 3"


def test_explore_verb_bridges_existing_explorer_cli(capsys):
    code, out, _err = _run(
        [
            "explore",
            "--source",
            "set x 1",
            "--show",
            "asm",
            "--no-colour",
            "--no-source-callouts",
        ],
        capsys,
    )

    assert code == 0
    assert "bytecode-asm" in out


def test_help_search_uses_kcs_db_results(monkeypatch, capsys):
    def _fake_queries():
        def _list_features():
            return {}

        def _search_help(query: str, *, limit: int = 20):
            assert query == "taint"
            assert limit == 20
            return [
                {
                    "name": "Taint Analysis",
                    "summary": "Track untrusted data.",
                    "category": "LSP Features (all editors)",
                    "file": "kcs-feature-taint-analysis.md",
                }
            ]

        return _list_features, _search_help

    monkeypatch.setattr(tcl_cli, "_load_help_queries", _fake_queries)

    code, out, err = _run(["help", "taint"], capsys)

    assert code == 0
    assert "Taint Analysis" in out
    assert "kcs-feature-taint-analysis.md" in out
    assert err == ""


def test_help_lists_sections_without_query(monkeypatch, capsys):
    def _fake_queries():
        def _list_features():
            return {
                "LSP Features (all editors)": [
                    {"name": "Hover", "summary": "Symbol and command hover docs."},
                    {"name": "Diagnostics", "summary": "Static diagnostics."},
                ]
            }

        def _search_help(_query: str, *, limit: int = 20):  # pragma: no cover - unused
            return []

        return _list_features, _search_help

    monkeypatch.setattr(tcl_cli, "_load_help_queries", _fake_queries)

    code, out, err = _run(["help"], capsys)

    assert code == 0
    assert "LSP Features (all editors)" in out
    assert "Hover" in out
    assert err == ""


def test_help_returns_nonzero_when_no_match(monkeypatch, capsys):
    def _fake_queries():
        def _list_features():
            return {"LSP Features (all editors)": [{"name": "Hover", "summary": "x"}]}

        def _search_help(_query: str, *, limit: int = 20):
            return []

        return _list_features, _search_help

    monkeypatch.setattr(tcl_cli, "_load_help_queries", _fake_queries)

    code, out, err = _run(["help", "missing-topic"], capsys)

    assert code == 1
    assert out == ""
    assert "no KCS help matches" in err
    assert "available sections:" in err


def test_help_search_filters_by_dialect(monkeypatch, capsys):
    def _fake_queries():
        def _list_features():
            return {}

        def _search_help(query: str, *, limit: int = 20):
            assert query == "event"
            assert limit == 100
            return [
                {
                    "name": "iRules Events",
                    "summary": "Event model for F5 BIG-IP iRules.",
                    "category": "LSP + AI Features",
                    "surface": "lsp,mcp,all-editors",
                    "file": "kcs-feature-irules-events.md",
                },
                {
                    "name": "Tk Preview",
                    "summary": "Preview Tk GUI layouts.",
                    "category": "VS Code AI Chat",
                    "surface": "vscode-chat",
                    "file": "kcs-feature-tk-preview.md",
                },
            ]

        return _list_features, _search_help

    monkeypatch.setattr(tcl_cli, "_load_help_queries", _fake_queries)

    code, out, err = _run(["help", "event", "--dialect", "f5-irules"], capsys)

    assert code == 0
    assert "iRules Events" in out
    assert "Tk Preview" not in out
    assert err == ""


def test_help_list_filters_by_dialect(monkeypatch, capsys):
    def _fake_queries():
        def _list_features():
            return {
                "LSP + AI Features": [
                    {
                        "name": "iRules Events",
                        "summary": "Event model for F5 BIG-IP iRules.",
                        "surface": "lsp,mcp,all-editors",
                        "file": "kcs-feature-irules-events.md",
                    },
                    {
                        "name": "Tk Preview",
                        "summary": "Preview Tk GUI layouts.",
                        "surface": "vscode-chat",
                        "file": "kcs-feature-tk-preview.md",
                    },
                ]
            }

        def _search_help(_query: str, *, limit: int = 20):  # pragma: no cover - unused
            return []

        return _list_features, _search_help

    monkeypatch.setattr(tcl_cli, "_load_help_queries", _fake_queries)

    code, out, err = _run(["help", "--dialect", "f5-irules"], capsys)

    assert code == 0
    assert "iRules Events" in out
    assert "Tk Preview" not in out
    assert err == ""


def test_help_subcommand_supports_help_flag(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["help", "--help"])

    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Search KCS help docs from the bundled SQLite index." in captured.out
    assert "--dialect" in captured.out


@pytest.mark.parametrize("verb", ["diag", "lint"])
def test_irule_prog_defaults_diag_like_verbs_to_f5_irules_dialect(verb):
    args = tcl_cli.parse_args(
        [verb, "--source", "when HTTP_REQUEST { return }"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_irule_prog_defaults_help_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["help", "event"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_prog_name_inference_for_irule_alias():
    assert tcl_cli._infer_prog_name("/usr/local/bin/irule") == "irule"  # noqa: SLF001
    assert tcl_cli._default_dialect_for_prog("irule") == "f5-irules"  # noqa: SLF001


def test_irule_prog_defaults_diff_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["diff", "left.irule", "right.irule"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_irule_prog_defaults_highlight_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["highlight", "--source", "when HTTP_REQUEST { return }", "--no-colour"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_irule_prog_defaults_format_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["format", "--source", "when HTTP_REQUEST { return }"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_irule_prog_defaults_minify_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["minify", "--source", "when HTTP_REQUEST { return }"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_irule_prog_defaults_callgraph_dialect_to_f5_irules():
    args = tcl_cli.parse_args(
        ["callgraph", "--source", "when HTTP_REQUEST { return }", "--json"],
        prog_name="irule",
        default_dialect="f5-irules",
    )

    assert args.dialect == "f5-irules"


def test_diff_reports_ir_changes(tmp_path, capsys):
    left = tmp_path / "left.irule"
    right = tmp_path / "right.irule"
    left.write_text("set x 1\n")
    right.write_text("set x 2\n")

    code, out, err = _run(["diff", str(left), str(right), "--show", "ir"], capsys)

    assert code == 1
    assert "=== ir diff ===" in out
    assert "assign-const x = 1" in out
    assert "assign-const x = 2" in out
    assert err == ""


def test_diff_json_reports_equal_for_identical_inputs(tmp_path, capsys):
    left = tmp_path / "left.irule"
    right = tmp_path / "right.irule"
    source = "when HTTP_REQUEST { return }\n"
    left.write_text(source)
    right.write_text(source)

    code, out, err = _run(
        ["diff", str(left), str(right), "--show", "ast,ir,cfg", "--json"],
        capsys,
    )

    assert code == 0
    payload = json.loads(out)
    assert payload["equal"] is True
    assert payload["layers"]["ast"]["equal"] is True
    assert payload["layers"]["ir"]["equal"] is True
    assert payload["layers"]["cfg"]["equal"] is True
    assert err == ""


def test_diff_help_supports_help_flag(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["diff", "--help"])

    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Diff two sources using AST, IR, and CFG representations." in captured.out
    assert "--show" in captured.out


def test_highlight_help_supports_help_flag(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["highlight", "--help"])

    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Emit syntax-highlighted source output." in captured.out
    assert "--format {ansi,html}" in captured.out
    assert "--colour" in captured.out


def test_format_help_supports_help_flag(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["format", "--help"])

    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Format source and emit rewritten Tcl." in captured.out
    assert "--output OUTPUT" in captured.out


def test_command_info_help_supports_help_flag(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["command-info", "--help"])

    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Look up command registry metadata." in captured.out
    assert "--dialect" in captured.out


# ---------------------------------------------------------------------------
# Config file loading
# ---------------------------------------------------------------------------


def test_config_file_loading(tmp_path, monkeypatch, capsys):
    """Config file values are read and applied."""
    ini = tmp_path / "tcl.ini"
    ini.write_text("[output]\ncolour = never\ntabs = 2\n[formatter]\nindent_size = 2\n")
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path))

    code, out, err = _run(
        ["format", "--source", "proc foo {} {\n    set x 1\n}"],
        capsys,
    )
    assert code == 0
    # With indent_size=2, the body indentation should be 2 spaces
    assert "  set x 1" in out


def test_config_file_project_local(tmp_path, monkeypatch, capsys):
    """A .tcl.ini in CWD overrides global settings."""
    global_dir = tmp_path / "global"
    global_dir.mkdir()
    global_ini = global_dir / "tcl.ini"
    global_ini.write_text("[formatter]\nindent_size = 8\n")

    local_ini = tmp_path / ".tcl.ini"
    local_ini.write_text("[formatter]\nindent_size = 2\n")

    monkeypatch.setenv("XDG_CONFIG_HOME", str(global_dir))
    monkeypatch.chdir(tmp_path)

    code, out, err = _run(
        ["format", "--source", "proc foo {} {\n    set x 1\n}"],
        capsys,
    )
    assert code == 0
    assert "  set x 1" in out


# ---------------------------------------------------------------------------
# Formatter CLI flags
# ---------------------------------------------------------------------------


def test_format_indent_size_flag(capsys):
    code, out, err = _run(
        ["format", "--indent-size", "2", "--source", "proc foo {} {\n    set x 1\n}"],
        capsys,
    )
    assert code == 0
    assert "  set x 1" in out


def test_format_indent_style_tabs(capsys):
    code, out, err = _run(
        [
            "format",
            "--indent-style",
            "tabs",
            "--tabs",
            "0",
            "--source",
            "proc foo {} {\n    set x 1\n}",
        ],
        capsys,
    )
    assert code == 0
    assert "\tset x 1" in out


# ---------------------------------------------------------------------------
# Diagnostic --disable / --enable
# ---------------------------------------------------------------------------


def test_diag_disable_suppresses_code(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    # W100 = unbraced expr
    script.write_text("set x [expr $y + 1]\n")

    code1, out1, _ = _run(["diag", str(script)], capsys)
    assert "W100" in out1

    code2, out2, _ = _run(["diag", str(script), "--disable", "W100"], capsys)
    assert "W100" not in out2


# ---------------------------------------------------------------------------
# Optimisation --disable and summary block
# ---------------------------------------------------------------------------


def test_opt_disable_suppresses_code(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set a 1\nset b [expr {$a + 2}]\n")

    code, out, err = _run(["opt", str(script), "--disable", "O100,O101"], capsys)
    assert code == 0
    # The disabled codes should not appear in the summary
    for line in out.splitlines():
        if line.startswith("# O100") or line.startswith("# O101"):
            pytest.fail(f"Disabled code found in summary: {line}")


def test_opt_summary_comment_block_on_stdout(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set a 1\nset b [expr {$a + 2}]\n")

    code, out, err = _run(["opt", str(script)], capsys)
    assert code == 0
    assert "# -------------" in out
    assert "# optimised:" in out


def test_opt_summary_on_stderr_for_file_output(tmp_path, capsys):
    script = tmp_path / "sample.tcl"
    script.write_text("set a 1\nset b [expr {$a + 2}]\n")
    output = tmp_path / "out.tcl"

    code, out, err = _run(["opt", str(script), "-o", str(output)], capsys)
    assert code == 0
    assert "# -------------" not in output.read_text()
    assert "rewrites=" in err


# ---------------------------------------------------------------------------
# Two-tier help
# ---------------------------------------------------------------------------


def test_brief_help(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["--help"])
    assert exc_info.value.code is None or exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "Verbs:" in captured.out
    assert "format (fmt)" in captured.out
    assert "--help-all" in captured.out


def test_help_all(capsys):
    with pytest.raises(SystemExit) as exc_info:
        main(["--help-all"])
    assert exc_info.value.code is None or exc_info.value.code == 0
    captured = capsys.readouterr()
    # Should contain section headers for each verb
    assert "opt" in captured.out
    assert "format" in captured.out
    assert "highlight" in captured.out


# ---------------------------------------------------------------------------
# Highlight recovery=False (bodies not misclassified)
# ---------------------------------------------------------------------------


def test_highlight_bodies_not_tagged_as_strings(capsys):
    """Braced command bodies should not be entirely highlighted as strings."""
    source = "proc foo {} {\n    set x 1\n    puts $x\n}\n"
    code, out, err = _run(["highlight", "--colour", "--source", source], capsys)
    assert code == 0
    # The 'set' command inside the body should be highlighted as a command
    # (bold blue), not just plain green (string/braced).
    assert "\033[1;34mset\033[0m" in out
