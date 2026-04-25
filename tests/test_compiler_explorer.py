"""Tests for compiler/optimiser exploration client."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from explorer.cli import Ansi, load_source, main, parse_args, style
from explorer.formatters import LineIndex, preview
from explorer.pipeline import ALL_VIEWS

# Helpers


def _run(args: list[str], capsys) -> tuple[int, str]:
    code = main(args)
    out = capsys.readouterr().out
    return code, out


def _run_source(source: str, capsys, *, extra: list[str] | None = None) -> tuple[int, str]:
    args = ["--source", source, "--no-colour"]
    if extra:
        args.extend(extra)
    return _run(args, capsys)


# LineIndex unit tests


class TestLineIndex:
    def test_single_line(self):
        idx = LineIndex("hello")
        assert idx.line_count() == 1
        assert idx.line_text(0) == "hello"
        assert idx.offset_to_line_col(0) == (0, 0)
        assert idx.offset_to_line_col(4) == (0, 4)

    def test_multiple_lines(self):
        idx = LineIndex("ab\ncd\nef")
        assert idx.line_count() == 3
        assert idx.line_text(0) == "ab"
        assert idx.line_text(1) == "cd"
        assert idx.line_text(2) == "ef"
        assert idx.offset_to_line_col(0) == (0, 0)
        assert idx.offset_to_line_col(3) == (1, 0)
        assert idx.offset_to_line_col(6) == (2, 0)

    def test_trailing_newline(self):
        idx = LineIndex("ab\n")
        assert idx.line_count() == 2
        assert idx.line_text(0) == "ab"
        assert idx.line_text(1) == ""

    def test_empty_source(self):
        idx = LineIndex("")
        assert idx.line_count() == 1
        assert idx.line_text(0) == ""

    def test_line_text_out_of_bounds(self):
        idx = LineIndex("hello")
        assert idx.line_text(-1) == ""
        assert idx.line_text(99) == ""

    def test_offset_clamped(self):
        idx = LineIndex("abc")
        assert idx.offset_to_line_col(-5) == (0, 0)
        assert idx.offset_to_line_col(999) == (0, 3)

    def test_line_start_and_end(self):
        idx = LineIndex("ab\ncd\nef")
        assert idx.line_start(0) == 0
        assert idx.line_start(1) == 3
        assert idx.line_end_exclusive(0) == 3
        assert idx.line_end_exclusive(1) == 6


# Utility function tests


class TestUtilities:
    def test_preview_short(self):
        assert preview("hello") == "hello"

    def test_preview_truncates(self):
        result = preview("a" * 100, limit=20)
        assert len(result) == 20
        assert result.endswith("...")

    def test_preview_escapes(self):
        assert "\\n" in preview("line1\nline2")
        assert "\\t" in preview("col1\tcol2")

    def test_style_enabled(self):
        result = style("text", Ansi.RED, enabled=True)
        assert Ansi.RED in result
        assert Ansi.RESET in result

    def test_style_disabled(self):
        result = style("text", Ansi.RED, enabled=False)
        assert result == "text"

    def test_load_source_from_arg(self):
        assert load_source(None, "hello") == "hello"

    def test_load_source_from_file(self, tmp_path):
        f = tmp_path / "test.tcl"
        f.write_text("set x 1\n")
        assert load_source(str(f), None) == "set x 1\n"

    def test_load_source_no_input(self, monkeypatch):
        monkeypatch.setattr(sys.stdin, "isatty", lambda: True)
        with pytest.raises(ValueError, match="No Tcl input"):
            load_source(None, None)

    def test_parse_args_defaults(self):
        args = parse_args([])
        assert args.views == ALL_VIEWS
        assert args.max_annotations == 80
        assert not args.no_colour
        assert not args.show_optimised_source


# CLI integration tests (main)


class TestCompilerExplorer:
    def test_compiler_focus_shows_pre_and_post_ssa_cfg(self, capsys):
        source = "if {$cond} {set a 1} else {set a 2}\nset b [expr {$a + 0}]"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "cfg-pre-ssa" in out
        assert "cfg-post-ssa" in out
        assert "phi a#" in out
        assert "uses=" in out
        assert "defs=" in out

    def test_smoke_optimiser_focus(self, capsys):
        source = "set a 1\nset b [expr {$a + 2}]"
        code, out = _run_source(
            source, capsys, extra=["--focus", "optimiser", "--no-source-callouts"]
        )

        assert code == 0
        assert "compiler-optimiser-explorer" in out
        assert "optimiser" in out
        assert "O102" in out

    def test_source_callouts_use_arrow_style(self, capsys):
        source = "set a 1\nset b [expr {$a + 2}]"
        code, out = _run_source(
            source, capsys, extra=["--focus", "optimiser", "--max-annotations", "10"]
        )

        assert code == 0
        assert "source-callouts" in out
        assert "+--> O102" in out

    def test_all_focus_includes_both_sections(self, capsys):
        source = "proc foo {x} { return $x }\nset y [foo 1]"
        code, out = _run_source(source, capsys, extra=["--no-source-callouts"])

        assert code == 0
        assert "compiler-ir" in out
        assert "cfg-pre-ssa" in out
        assert "cfg-post-ssa" in out
        assert "interprocedural" in out
        assert "optimiser" in out

    def test_ir_shows_proc_with_params(self, capsys):
        source = "proc add {a b} { expr {$a + $b} }"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "::add" in out
        assert "a b" in out

    def test_interprocedural_pure_proc(self, capsys):
        source = "proc double {x} { expr {$x * 2} }"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "interprocedural" in out
        assert "::double" in out
        assert "pure=" in out

    def test_show_optimised_source(self, capsys):
        source = "set a 1\nset b [expr {$a + 2}]"
        code, out = _run_source(
            source,
            capsys,
            extra=["--focus", "optimiser", "--show-optimised-source", "--no-source-callouts"],
        )

        assert code == 0
        assert "optimised-source" in out

    def test_max_annotations_limits_output(self, capsys):
        # Generate enough annotations to hit the limit.
        source = "\n".join(f"set v{i} [expr {{$v{i} + 0}}]" for i in range(20))
        code, out = _run_source(source, capsys, extra=["--max-annotations", "2"])

        assert code == 0
        assert "more annotations omitted" in out

    def test_no_source_callouts_suppresses_section(self, capsys):
        source = "set a 1"
        code, out = _run_source(source, capsys, extra=["--no-source-callouts"])

        assert code == 0
        assert "source-callouts" not in out

    def test_shimmer_detection_in_output(self, capsys):
        source = "proc test {x} {\n  set len [string length $x]\n  expr {$x + 1}\n}"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "shimmer-detection" in out

    def test_dead_store_reported(self, capsys):
        source = "proc f {} { set a 1\nset a 2\nreturn $a }"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "dead-stores" in out

    def test_summary_line_counts(self, capsys):
        source = "proc p {x} { return $x }\nset y [p 1]"
        code, out = _run_source(source, capsys, extra=["--no-source-callouts"])

        assert code == 0
        assert "procedures=" in out
        assert "blocks=" in out
        assert "rewrites=" in out

    def test_error_returns_nonzero(self, capsys):
        # No source provided (and stdin is a tty) → error.
        code = main(["--no-colour"])

        captured = capsys.readouterr()
        assert code == 2
        assert "error" in captured.err.lower()

    def test_file_input(self, capsys, tmp_path):
        f = tmp_path / "test.tcl"
        f.write_text("set x 42\n")
        code, out = _run([str(f), "--no-colour", "--no-source-callouts"], capsys)

        assert code == 0
        assert "compiler-optimiser-explorer" in out

    def test_for_loop_ir(self, capsys):
        source = "for {set i 0} {$i < 10} {incr i} { puts $i }"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "for (" in out

    def test_switch_ir(self, capsys):
        source = "switch -- $x { a { puts a } b { puts b } }"
        code, out = _run_source(
            source, capsys, extra=["--focus", "compiler", "--no-source-callouts"]
        )

        assert code == 0
        assert "switch" in out
        assert "arm" in out

    def test_asm_view(self, capsys):
        code, out = _run_source("set x 1; puts $x", capsys, extra=["--show", "asm"])

        assert code == 0
        assert "bytecode-asm" in out
        assert "push1" in out
        assert "invokeStk1" in out
        assert "done" in out
