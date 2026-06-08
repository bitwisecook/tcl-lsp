"""Textual live UI for the compiler explorer.

The default ``tcl-explorer`` surface on an interactive terminal.  It is a thin
presentation layer over one pipeline + one serialisation: every analysis view
is an interactive **tree** built from the same ``serialise_result`` payload the
web GUI consumes (see ``tooling.explorer.tui_views``), so each item expands to
its full detail exactly like the GUI's click/Space expansion — and the TUI,
the web GUI, and the flat ``--text`` output stay in lockstep.

A couple of views keep the captured-ANSI text renderer instead: ``asm`` /
``wasm`` (specialised disassembly, navigated rather than expanded) and any
``OPT_VIEWS`` shown in ``--opt diff`` mode (a rendered unified diff).

This is a debugging / code-understanding aid, not a stable interface — views
may be added, removed, or reshaped freely.

Keys: ↑/↓ or click to switch view · click / Enter / Space to expand a tree
item · ``o`` cycle the optimisation lens · ``r`` re-run · ``q`` quit.
"""

from __future__ import annotations

import argparse
import contextlib
import io

from rich.text import Text
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.widgets import Footer, Header, Label, ListItem, ListView, Static, Tree

from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import CompilerExplorerResult, run_pipeline
from tooling.explorer.tui_views import TREE_VIEWS, ViewNode, build_view

_OPT_CYCLE = ("off", "on", "diff")


def _format_detail(node: ViewNode | None) -> Text:
    """Render a highlighted node's detail table (and child hint) for the panel."""
    if node is None:
        return Text("↑/↓ to browse  ·  Enter / Space / click to expand", style="dim")
    t = Text()
    for key, value in node.detail:
        t.append(f"{key:<16}", style="bold")
        sval = str(value)
        if "\n" in sval:
            t.append("\n")
            t.append(sval, style="bright_white")
        else:
            t.append(sval)
        t.append("\n")
    if node.children:
        if node.detail:
            t.append("\n")
        t.append(f"{len(node.children)} child item(s) — Enter / Space to expand", style="dim")
    return t


def _render_view_ansi(
    view: str,
    result: CompilerExplorerResult,
    source: str,
    args: argparse.Namespace,
    *,
    opt_mode: str = "off",
    opt: tuple[CompilerExplorerResult, str] | None = None,
) -> Text:
    """Capture ``render_view_opt``'s ANSI output for *view* and parse it for Rich."""
    from tooling.explorer.cli import LineIndex, render_view_opt  # deferred (cycle)

    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        try:
            render_view_opt(
                view,
                result,
                source,
                use_colour=True,
                line_index=LineIndex(source),
                opt_mode=opt_mode,
                opt=opt,
                views=args.views,
                show_optimised_source=args.show_optimised_source,
                max_annotations=args.max_annotations,
            )
        except Exception as exc:  # a single view failing must not kill the UI
            return Text(f"view '{view}' failed to render: {exc}", style="red")
    out = buf.getvalue()
    return Text.from_ansi(out) if out.strip() else Text(f"(no output for '{view}')", style="dim")


class ExplorerApp(App):
    """Live compiler-explorer TUI: a view sidebar + an interactive render pane."""

    CSS = """
    #sidebar { width: 24; border-right: solid $accent; }
    #sidebar > ListView { height: 1fr; }
    #content { padding: 0 1; }
    #summary { color: $text-muted; padding: 0 1; height: auto; }
    #tree-pane { display: none; }
    #view-tree { width: 1fr; }
    #view-detail-scroll { width: 45%; border-left: solid $accent; }
    #view-detail { padding: 0 1; }
    """

    BINDINGS = [
        ("q", "quit", "Quit"),
        ("r", "refresh", "Re-run"),
        ("o", "cycle_opt", "Opt off/on/diff"),
    ]

    def __init__(self, source: str, args: argparse.Namespace) -> None:
        super().__init__()

        from tooling.explorer.cli import (  # deferred (cycle)
            _VIEW_ORDER,
            OPT_VIEWS,
            _summary_parts,
            load_source,
            optimised_result,
        )
        self._OPT_VIEWS = OPT_VIEWS
        self._VIEW_ORDER = _VIEW_ORDER
        self._summary_parts = _summary_parts
        self._load_source = load_source
        self._optimised_result = optimised_result

        self._source = source
        self._args = args
        self._result: CompilerExplorerResult | None = None
        self._data: dict = {}
        self._opt: tuple[CompilerExplorerResult, str] | None = None
        self._opt_mode = getattr(args, "opt", "off") or "off"
        self._views = [v for v in self._VIEW_ORDER if v in args.views]
        self._current = self._views[0] if self._views else "ir"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        yield Label("", id="summary")
        with Horizontal():
            with VerticalScroll(id="sidebar"):
                yield ListView(*(ListItem(Label(v), id=f"view-{v}") for v in self._views))
            with Vertical(id="main"):
                # Text surface: captured-ANSI render (asm/wasm and opt diff).
                with VerticalScroll(id="content-scroll"):
                    yield Static("", id="content", markup=False)
                # Interactive surface: a tree whose entries expand (↑/↓ + click /
                # Enter / Space) and pop their full detail into the side panel.
                with Horizontal(id="tree-pane"):
                    yield Tree("view", id="view-tree")
                    with VerticalScroll(id="view-detail-scroll"):
                        yield Static("", id="view-detail", markup=False)
        yield Footer()

    def on_mount(self) -> None:
        self.title = "Tcl Compiler Explorer"
        self._recompute()
        if self._views:
            lv = self.query_one(ListView)
            lv.index = 0
            lv.focus()  # so ↑/↓ navigate the view list
            self._show(self._current)

    def _recompute(self) -> None:
        try:
            self._result = run_pipeline(self._source, dialect=self._args.dialect)
            # One serialisation drives every tree view (same payload as the GUI).
            self._data = serialise_result(self._result)
        except Exception as exc:
            self.query_one("#content", Static).update(
                Text(f"compiler exploration failed: {exc}", style="red")
            )
            self._result = None
            self._data = {}
            return
        # The optimised pipeline feeds the text renderer's opt lens (asm/wasm
        # 'on', and any view's 'diff') — needed for every non-"off" mode, not
        # just diff.  Tree views get optimised data from the serialised payload.
        self._opt = (
            self._optimised_result(self._result, self._args.dialect) if self._opt_mode != "off" else None
        )
        self._update_subtitle()
        self.query_one("#summary", Label).update(
            "  ".join(self._summary_parts(self._result, self._args.dialect))
        )

    def _update_subtitle(self) -> None:
        opt = "" if self._opt_mode == "off" else f"  ·  opt: {self._opt_mode}"
        self.sub_title = f"{self._args.dialect}{opt}"

    def _is_text_view(self, view: str) -> bool:
        """Views the TUI renders as captured ANSI text instead of a tree."""
        if view not in TREE_VIEWS:
            return True  # asm / wasm / asm-opt / wasm-opt
        return self._opt_mode == "diff" and view in self._OPT_VIEWS

    def _view_data(self, view: str) -> dict:
        """Serialised data for *view*, swapping in optimised keys under ``opt on``."""
        data = self._data
        if self._opt_mode == "on" and view in self._OPT_VIEWS:
            data = dict(data)
            if data.get("irOptimised"):
                data["ir"] = data["irOptimised"]
            if data.get("cfgPreSsaOptimised"):
                data["cfgPreSsa"] = data["cfgPreSsaOptimised"]
            if data.get("cfgPostSsaOptimised"):
                data["cfgPostSsa"] = data["cfgPostSsaOptimised"]
        return data

    def _show(self, view: str) -> None:
        self._current = view
        if self._result is None:
            return
        tree_pane = self.query_one("#tree-pane")
        content_scroll = self.query_one("#content-scroll")
        if self._is_text_view(view):
            tree_pane.display = False
            content_scroll.display = True
            self.query_one("#content", Static).update(
                _render_view_ansi(
                    view,
                    self._result,
                    self._source,
                    self._args,
                    opt_mode=self._opt_mode,
                    opt=self._opt,
                )
            )
            return
        content_scroll.display = False
        tree_pane.display = True
        self._populate_view_tree(view)
        # Leave focus on the sidebar so ↑/↓ keep switching views; click or Tab
        # into the tree to navigate / expand its entries.

    def _populate_view_tree(self, view: str) -> None:
        tree = self.query_one("#view-tree", Tree)
        tree.clear()
        tree.root.set_label(view)
        tree.root.data = None
        nodes = build_view(view, self._view_data(view))
        self._add_view_nodes(tree.root, nodes)
        tree.root.expand()
        # Reveal the first level immediately when there's a single root group
        # (e.g. the green tree's region, or a one-function analysis view).
        if len(tree.root.children) == 1:
            tree.root.children[0].expand()
        self.query_one("#view-detail", Static).update(_format_detail(None))

    def _add_view_nodes(self, parent, nodes: list[ViewNode]) -> None:
        for n in nodes:
            label = Text(n.label, style=n.style) if n.style else n.label
            if n.children:
                branch = parent.add(label, data=n)
                self._add_view_nodes(branch, n.children)
            else:
                parent.add_leaf(label, data=n)

    def on_tree_node_highlighted(self, event: Tree.NodeHighlighted) -> None:
        self.query_one("#view-detail", Static).update(_format_detail(event.node.data))

    def action_cycle_opt(self) -> None:
        """Cycle the optimisation lens off → on → diff and re-render."""
        self._opt_mode = _OPT_CYCLE[(_OPT_CYCLE.index(self._opt_mode) + 1) % len(_OPT_CYCLE)]
        # Compute the optimised pipeline for any non-"off" mode so the text
        # renderer (asm/wasm 'on', diff) has it; clear it when back to "off".
        if self._opt_mode == "off":
            self._opt = None
        elif self._opt is None and self._result is not None:
            self._opt = self._optimised_result(self._result, self._args.dialect)
        self._update_subtitle()
        self._show(self._current)

    def on_list_view_highlighted(self, event: ListView.Highlighted) -> None:
        if event.item is not None and event.item.id and event.item.id.startswith("view-"):
            self._show(event.item.id[len("view-") :])

    def action_refresh(self) -> None:
        # Re-read the file (if any) so the view tracks edits, then recompute.
        if self._args.path and self._args.path != "-":
            with contextlib.suppress(Exception):
                self._source = self._load_source(self._args.path, None)
        self._recompute()
        self._show(self._current)


def run_tui(source: str, args: argparse.Namespace) -> int:
    """Launch the Textual explorer UI.  Falls back to flat text on failure."""
    try:
        ExplorerApp(source, args).run()
        return 0
    except Exception as exc:  # pragma: no cover - terminal/driver issues
        import sys

        print(f"error: TUI failed ({exc}); use --text for flat output.", file=sys.stderr)
        return 2
