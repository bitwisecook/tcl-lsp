"""Tests for compiler/optimiser exploration client."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tooling.cli.formatters import LineIndex, preview
from tooling.explorer.cli import Ansi, load_source, main, parse_args, style
from tooling.explorer.pipeline import ALL_VIEWS

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


class TestGreenTreeView:
    def test_greentree_view_shows_mode_tagged_regions(self, capsys):
        code, out = _run_source("proc f {} { puts hi }", capsys, extra=["--show", "greentree"])
        assert code == 0
        assert "green-tree" in out
        assert "root [script]" in out
        assert "braced [script]" in out  # the proc body region

    def test_greentree_in_all_views(self):
        assert "greentree" in ALL_VIEWS


class TestLoopsView:
    def test_loops_view_lists_natural_loop(self, capsys):
        src = (
            "proc f {n} { set s 0; "
            "for {set i 0} {$i < $n} {incr i} { set s [expr {$s + $i}] }; "
            "return $s }"
        )
        code, out = _run_source(src, capsys, extra=["--show", "loops"])
        assert code == 0
        assert "loops" in out
        assert "function ::f" in out
        assert "header" in out and "block(s)" in out

    def test_loops_view_reports_no_loops(self, capsys):
        code, out = _run_source("set x 1\nputs $x", capsys, extra=["--show", "loops"])
        assert code == 0
        assert "(no loops)" in out

    def test_loops_in_all_views(self):
        assert "loops" in ALL_VIEWS


class TestIntervalsView:
    def test_intervals_view_shows_bounded_range(self, capsys):
        src = "proc f {} { set n 5; set m [expr {$n + 3}]; return $m }"
        code, out = _run_source(src, capsys, extra=["--show", "intervals"])
        assert code == 0
        assert "intervals" in out
        assert "function ::f" in out
        # n folds to [5,5] and m = n+3 to [8,8].
        assert "m#1: [8, 8]" in out

    def test_intervals_view_widens_loop_induction(self, capsys):
        src = "proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }"
        code, out = _run_source(src, capsys, extra=["--show", "intervals"])
        assert code == 0
        # The loop-header phi is widened: lower bound stays 0, upper goes +inf
        # (sound; guard-based narrowing to [0,9] is future work).
        assert "i#2: [0, +inf]" in out

    def test_intervals_in_all_views(self):
        assert "intervals" in ALL_VIEWS


class TestBoundsView:
    def test_bounds_view_shows_lset_out_of_range(self, capsys):
        src = "proc g {v} { set l {a b c}\n for {set j 4} {$j < 9} {incr j} { lset l $j $v } }"
        code, out = _run_source(src, capsys, extra=["--show", "bounds"])
        assert code == 0
        assert "bounds" in out
        assert "W231" in out
        assert "past_append" in out

    def test_bounds_view_shows_lindex_out_of_range(self, capsys):
        src = "proc h {} { set j 5\n set x [lindex {a b} $j] }"
        code, out = _run_source(src, capsys, extra=["--show", "bounds"])
        assert code == 0
        assert "W230" in out
        assert "past_end" in out

    def test_bounds_view_silent_in_range(self, capsys):
        src = "proc f {l} { for {set j 0} {$j < [llength $l]} {incr j} { set x [lindex $l $j] } }"
        code, out = _run_source(src, capsys, extra=["--show", "bounds"])
        assert code == 0
        assert "no provable out-of-range" in out

    def test_bounds_view_shows_divide_by_zero(self, capsys):
        src = "proc f {} { set d 0\n return [expr {10 / $d}] }"
        code, out = _run_source(src, capsys, extra=["--show", "bounds"])
        assert code == 0
        assert "W233" in out

    def test_bounds_in_all_views(self):
        assert "bounds" in ALL_VIEWS

    def test_types_view_annotates_range(self, capsys):
        # The types view shows the Phase 3 integer interval inline.
        src = "proc f {} { set n 5\n set m [expr {$n + 3}]\n return $m }"
        code, out = _run_source(src, capsys, extra=["--show", "types"])
        assert code == 0
        assert "range [8, 8]" in out


class TestOptLens:
    # ``set a 1; set b [expr {$a+2}]; puts $b`` folds to ``puts 3`` with a dead
    # store removed, so the optimised path differs from the original.
    SRC = "set a 1\nset b [expr {$a + 2}]\nputs $b"

    def test_opt_off_renders_original_ir(self, capsys):
        code, out = _run_source(self.SRC, capsys, extra=["--show", "ir", "--opt", "off"])
        assert code == 0
        assert "assign-const a = 1" in out  # original statements present

    def test_opt_on_renders_optimised_ir(self, capsys):
        code, out = _run_source(self.SRC, capsys, extra=["--show", "ir", "--opt", "on"])
        assert code == 0
        assert "puts 3" in out  # constant-folded call
        assert "assign-const a = 1" not in out  # dead store gone

    def test_opt_diff_shows_unified_diff(self, capsys):
        code, out = _run_source(self.SRC, capsys, extra=["--show", "ir", "--opt", "diff"])
        assert code == 0
        assert "ir (original)" in out and "ir (optimised)" in out
        assert "-  ├── assign-const a = 1" in out or "-  └── call puts ${b}" in out
        assert "+  └── call puts 3" in out

    def test_opt_diff_on_unchanged_source_says_so(self, capsys):
        # No optimiser rewrites here, so the diff has nothing to show.
        code, out = _run_source("puts hi", capsys, extra=["--show", "ir", "--opt", "diff"])
        assert code == 0
        assert "unchanged" in out or "no change" in out

    def test_non_opt_view_ignores_lens(self, capsys):
        code, out = _run_source(self.SRC, capsys, extra=["--show", "types", "--opt", "on"])
        assert code == 0  # types view renders normally regardless of lens

    # Folding the first two statements to ``puts 3`` deletes two lines, so the
    # trailing ``puts done`` slides from line 4 to line 2.  Its IR summary is
    # byte-for-byte identical — only its source range moved.  A raw text diff
    # would flag it (the ``[4:1-4:9]`` vs ``[2:1-2:9]`` range differs); the
    # node-level diff must leave it as quiet context.
    SHIFT_SRC = "set a 1\nset b [expr {$a + 2}]\nputs $b\nputs done"

    def test_opt_diff_ignores_offset_shift(self, capsys):
        code, out = _run_source(self.SHIFT_SRC, capsys, extra=["--show", "ir", "--opt", "diff"])
        assert code == 0
        # The genuinely rewritten statements are still surfaced.
        assert "+  ├── call puts 3" in out
        # ``puts done`` only moved line:col — it must not appear as a +/- line.
        moved = [line for line in out.splitlines() if line[:1] in "+-" and "puts done" in line]
        assert moved == [], f"offset-only shift leaked into the diff: {moved}"


class TestDiffNormalisation:
    """Unit tests for the offset-ignoring diff key (``_normalise_diff_line``)."""

    def test_tree_connector_and_range_collapse(self):
        from tooling.explorer.cli import _normalise_diff_line

        # Same node, different sibling position (├── vs └──) and source range.
        a = _normalise_diff_line("│   ├── call puts ${b} [3:1-3:7]")
        b = _normalise_diff_line("    └── call puts ${b} [9:9-9:99]")
        assert a == b

    def test_byte_offset_and_literal_index_collapse(self):
        from tooling.explorer.cli import _normalise_diff_line

        # Same instruction, shifted byte offset and literal-pool index.
        a = _normalise_diff_line('    (15) push1 4\t# "puts"')
        b = _normalise_diff_line('    (2) push1 0\t# "puts"')
        assert a == b

    def test_variable_slot_collapses_but_arity_is_kept(self):
        from tooling.explorer.cli import _normalise_diff_line

        # %vN slot shifts when a variable is dropped — the comment names it.
        assert _normalise_diff_line('    (4) loadScalar1 %v2\t# var "a"') == _normalise_diff_line(
            '    (9) loadScalar1 %v1\t# var "a"'
        )
        # Arity (invokeStk1 2) is semantic — distinct arities stay distinct.
        assert _normalise_diff_line("    (4) invokeStk1 2\t# puts") != _normalise_diff_line(
            "    (4) invokeStk1 3\t# puts"
        )

    def test_distinct_content_stays_distinct(self):
        from tooling.explorer.cli import _normalise_diff_line

        assert _normalise_diff_line('  └── call puts "a"') != _normalise_diff_line(
            '  └── call puts "b"'
        )


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
        assert "╰─▶ O102" in out

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
