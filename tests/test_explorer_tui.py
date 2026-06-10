"""Tests for the compiler-explorer output modes (TUI / text / JSON).

The TUI, flat text, and JSON all flow from one pipeline + one set of view
renderers (``cli.render_view`` / ``pipeline.run_pipeline`` / ``serialise``), so
these tests pin that shared behaviour, not the (intentionally unstable) UI
layout.
"""

from __future__ import annotations

import asyncio
import json

from tooling.explorer._render import render_view
from tooling.explorer.cli import parse_args, run_pipeline
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

        from tooling.explorer.tui import _format_detail

        args = parse_args(["/dev/null", "--show", "greentree,ir"])
        app = ExplorerApp(_SRC, args)

        async def drive():
            async with app.run_test() as pilot:
                await pilot.pause()
                app._show("greentree")
                await pilot.pause()
                tree = app.query_one("#view-tree", Tree)
                # The single root group is the region; its children are tokens,
                # opaque {...}/[...] tokens nesting their re-lexed region.
                region = tree.root.children[0]
                tokens = region.children
                nested = any(t.children for t in tokens)
                tree_visible = bool(app.query_one("#tree-pane").display)
                opaque = next(t for t in tokens if t.children)
                detail = str(_format_detail(opaque.data))
                return len(tokens), nested, tree_visible, detail

        ntokens, nested, tree_visible, detail = asyncio.run(drive())
        assert ntokens > 0
        assert nested  # the proc body / command substitution descended
        assert tree_visible
        assert "token type" in detail and "byte range" in detail and "region" in detail

    def test_analysis_views_build_trees_with_detail(self):
        from textual.widgets import Tree

        from tooling.explorer.tui import _format_detail

        args = parse_args(["/dev/null", "--show", "ir,cfg,types,interproc"])
        app = ExplorerApp(_SRC, args)

        async def drive():
            seen = {}
            async with app.run_test() as pilot:
                await pilot.pause()
                for view in ("ir", "cfg", "types", "interproc"):
                    app._show(view)
                    await pilot.pause()
                    assert not app._is_text_view(view)
                    tree = app.query_one("#view-tree", Tree)

                    def deepest(n):
                        return deepest(n.children[0]) if n.children else n

                    leaf = deepest(tree.root)
                    seen[view] = (len(tree.root.children), str(_format_detail(leaf.data)))
            return seen

        seen = asyncio.run(drive())
        for view, (nchildren, detail) in seen.items():
            assert nchildren > 0, f"{view} produced an empty tree"
            assert detail.strip(), f"{view} leaf had no detail"

    def test_asm_stays_text_surface(self):
        args = parse_args(["/dev/null", "--show", "ir,asm"])
        app = ExplorerApp(_SRC, args)

        async def drive():
            async with app.run_test() as pilot:
                await pilot.pause()
                app._show("ir")
                await pilot.pause()
                ir_tree = not app._is_text_view("ir") and bool(app.query_one("#tree-pane").display)
                app._show("asm")
                await pilot.pause()
                asm_text = app._is_text_view("asm") and bool(
                    app.query_one("#content-scroll").display
                )
                return ir_tree, asm_text

        ir_tree, asm_text = asyncio.run(drive())
        assert ir_tree
        assert asm_text

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
