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

    def test_unknown_view_raises(self):
        # ``render_view`` now errors loudly on a view it has no renderer
        # for — the dataflow silent-no-op bug used to hide this.  A
        # typo in ``--show`` is normally caught earlier (argparse) but
        # the dispatcher belt-and-braces.
        import pytest

        with pytest.raises(ValueError, match="no renderer for view"):
            _capture("does-not-exist")


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

    def test_greentree_builds_interactive_tree(self):
        from textual.widgets import Tree

        from tooling.explorer.tui import _gt_detail

        args = parse_args(["/dev/null", "--show", "greentree,ir"])
        app = ExplorerApp(_SRC, args)

        async def drive():
            async with app.run_test() as pilot:
                await pilot.pause()
                app._show("greentree")
                await pilot.pause()
                tree = app.query_one("#gt-tree", Tree)
                # Tokens hang off the root; opaque {...}/[...] tokens nest their
                # re-lexed child region's tokens.
                token_nodes = len(tree.root.children)
                nested = any(c.children for c in tree.root.children)
                gt_visible = bool(app.query_one("#gt-pane").display)
                opaque = next(c for c in tree.root.children if c.children)
                detail = str(_gt_detail(opaque.data))
                # Switching away hides the interactive pane again.
                app._show("ir")
                await pilot.pause()
                gt_hidden = not app.query_one("#gt-pane").display
                return token_nodes, nested, gt_visible, detail, gt_hidden

        token_nodes, nested, gt_visible, detail, gt_hidden = asyncio.run(drive())
        assert token_nodes > 0
        assert nested  # the proc body / command substitution descended
        assert gt_visible and gt_hidden
        assert "token type" in detail and "byte range" in detail and "region" in detail

    def test_cycle_opt_changes_mode(self):
        args = parse_args(["/dev/null", "--show", "ir"])
        app = ExplorerApp(_SRC, args)

        async def drive():
            async with app.run_test() as pilot:
                await pilot.pause()
                seq = [app._opt_mode]
                for _ in range(3):
                    app.action_cycle_opt()
                    await pilot.pause()
                    seq.append(app._opt_mode)
                return seq

        assert asyncio.run(drive()) == ["off", "on", "diff", "off"]
