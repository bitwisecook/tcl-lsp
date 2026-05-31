"""Tests for the compiler-explorer output modes (TUI / text / JSON).

The TUI, flat text, and JSON all flow from one pipeline + one set of view
renderers (``cli.render_view`` / ``pipeline.run_pipeline`` / ``serialise``), so
these tests pin that shared behaviour, not the (intentionally unstable) UI
layout.
"""

from __future__ import annotations

import asyncio
import json

from tooling.explorer.cli import parse_args, render_view, run_pipeline
from tooling.explorer.tui import ExplorerApp, _render_view_ansi

_SRC = "proc f {x} { set y [expr {$x + 1}]\n return $y }\nputs [f 3]\n"


def _capture(view: str) -> str:
    import contextlib
    import io

    from tooling.explorer.cli import LineIndex

    res = run_pipeline(_SRC)
    args = parse_args(["--show", "all"])
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        render_view(view, res, _SRC, use_colour=False, line_index=LineIndex(_SRC), views=args.views)
    return buf.getvalue()


class TestRenderView:
    def test_each_view_renders_something(self):
        # Every advertised view produces output via the shared renderer.
        for view in ("ir", "cfg", "ssa", "greentree", "types", "opt", "callouts"):
            out = _capture(view)
            assert out.strip(), f"view {view} produced no output"

    def test_unknown_view_is_silent(self):
        assert _capture("does-not-exist") == ""


class TestJsonMode:
    def test_serialise_is_valid_json_with_expected_views(self):
        from tooling.cli.serialise import serialise_result

        payload = json.loads(json.dumps(serialise_result(run_pipeline(_SRC))))
        for key in ("ir", "cfgPreSsa", "cfgPostSsa", "stats"):
            assert key in payload


class TestModeResolution:
    def test_explicit_flags_parse(self):
        assert parse_args(["x.tcl", "--json"]).format == "json"
        assert parse_args(["x.tcl", "--text"]).format == "text"
        assert parse_args(["x.tcl", "--tui"]).format == "tui"
        assert parse_args(["x.tcl"]).format is None  # resolved by main() from tty


class TestTui:
    def test_render_view_ansi_returns_rich_text(self):
        res = run_pipeline(_SRC)
        args = parse_args(["--show", "all"])
        t = _render_view_ansi("ir", res, _SRC, args)
        # The IR view must render the actual lowered module: its header plus the
        # proc and top-level calls from _SRC (proc f ...; puts [f 3]).
        plain = str(t)
        assert "compiler-ir" in plain
        assert "proc f" in plain
        assert "puts" in plain

    def test_tui_mounts_and_navigates_views(self):
        args = parse_args(["/dev/null", "--show", "ir,cfg,ssa,opt"])
        app = ExplorerApp(_SRC, args)
        # Sidebar follows the canonical view order (ir, cfg, ssa, opt).
        assert app._views == ["ir", "cfg", "ssa", "opt"]

        async def drive() -> list[str]:
            async with app.run_test() as pilot:
                await pilot.pause()
                seq = [app._current]
                for _ in range(3):
                    await pilot.press("down")
                    await pilot.pause()
                    seq.append(app._current)
                app.action_refresh()
                await pilot.pause()
                return seq

        seq = asyncio.run(drive())
        assert seq == ["ir", "cfg", "ssa", "opt"]
        assert app._result is not None  # pipeline ran inside the app
