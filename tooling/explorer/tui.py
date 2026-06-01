"""Textual live UI for the compiler explorer.

The default ``tcl-explorer`` surface on an interactive terminal.  It is a thin
presentation layer: it runs the *same* ``run_pipeline`` and renders each view
with the *same* ``cli.render_view`` text renderers (captured and shown as ANSI
via Rich), so the TUI, the flat ``--text`` output, and the web GUI all stay in
lockstep with one pipeline + one set of renderers.

This is a debugging / code-understanding aid, not a stable interface — views
may be added, removed, or reshaped freely.

Keys: ↑/↓ or click to switch view · ``r`` re-run (re-reads the file) · ``q`` quit.
"""

from __future__ import annotations

import argparse
import contextlib
import io

from rich.text import Text
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.widgets import Footer, Header, Label, ListItem, ListView, Static, Tree

from tooling.explorer.cli import (
    _VIEW_ORDER,
    LineIndex,
    _summary_parts,
    load_source,
    optimised_result,
    render_view_opt,
)
from tooling.explorer.pipeline import CompilerExplorerResult, run_pipeline

_OPT_CYCLE = ("off", "on", "diff")
_GT_OPAQUE = ("STR", "CMD")


def _gt_label(tok: dict) -> Text:
    """A one-line tree label for a green-tree token: ``TYPE 'preview' [a:b]``."""
    opaque = tok["type"] in _GT_OPAQUE
    one_line = tok["text"].replace("\n", "⏎")
    preview = one_line if len(one_line) <= 40 else one_line[:40] + "…"
    label = Text()
    label.append(tok["type"], style="cyan" if opaque else "bright_black")
    label.append(" ")
    label.append(repr(preview), style="")
    label.append(f" [{tok['startOffset']}:{tok['endOffset']}]", style="dim")
    return label


def _gt_detail(data: dict) -> Text:
    """Full detail for a highlighted green-tree node (token or root region)."""
    t = Text()
    if "type" in data:  # a token
        t.append("token type  ", style="bold")
        t.append(f"{data['type']}\n")
        t.append("byte range  ", style="bold")
        t.append(
            f"{data['startOffset']}…{data['endOffset']} "
            f"({max(0, data['endOffset'] - data['startOffset'])} bytes)\n"
        )
        t.append("line:col    ", style="bold")
        t.append(
            f"{data['startLine'] + 1}:{data['startCol'] + 1} → "
            f"{data['endLine'] + 1}:{data['endCol'] + 1}\n"
        )
        child = data.get("child")
        if child is not None:
            t.append("region      ", style="bold")
            style = "red" if child["isError"] else "cyan"
            t.append(
                f"{child['kind'].lower()} / {child['mode'].lower()}"
                + (" (ERROR — unterminated)" if child["isError"] else "")
                + "\n",
                style=style,
            )
        t.append("text\n", style="bold")
        t.append(data["text"] or "(empty)", style="bright_white")
    else:  # the root region
        t.append("region      ", style="bold")
        t.append(f"{data['kind'].lower()} / {data['mode'].lower()}\n")
        t.append("width       ", style="bold")
        t.append(f"{data['width']} bytes\n")
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
    """Live compiler-explorer TUI: a view sidebar + a scrolling render pane."""

    CSS = """
    #sidebar { width: 24; border-right: solid $accent; }
    #sidebar > ListView { height: 1fr; }
    #content { padding: 0 1; }
    #summary { color: $text-muted; padding: 0 1; height: auto; }
    #gt-pane { display: none; }
    #gt-tree { width: 1fr; }
    #gt-detail-scroll { width: 45%; border-left: solid $accent; }
    #gt-detail { padding: 0 1; }
    """

    BINDINGS = [
        ("q", "quit", "Quit"),
        ("r", "refresh", "Re-run"),
        ("o", "cycle_opt", "Opt off/on/diff"),
    ]

    def __init__(self, source: str, args: argparse.Namespace) -> None:
        super().__init__()
        self._source = source
        self._args = args
        self._result: CompilerExplorerResult | None = None
        self._opt: tuple[CompilerExplorerResult, str] | None = None
        self._opt_mode = getattr(args, "opt", "off") or "off"
        self._views = [v for v in _VIEW_ORDER if v in args.views]
        self._current = self._views[0] if self._views else "ir"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        yield Label("", id="summary")
        with Horizontal():
            with VerticalScroll(id="sidebar"):
                yield ListView(*(ListItem(Label(v), id=f"view-{v}") for v in self._views))
            with Vertical(id="main"):
                # Default surface: captured-ANSI render of the active view.
                with VerticalScroll(id="content-scroll"):
                    yield Static("", id="content", markup=False)
                # Interactive surface used only for the green tree: a real tree
                # whose entries expand (↑/↓ + Enter/Space/click) and pop their
                # full token detail into the side panel as you move.
                with Horizontal(id="gt-pane"):
                    yield Tree("green-tree", id="gt-tree")
                    with VerticalScroll(id="gt-detail-scroll"):
                        yield Static("", id="gt-detail", markup=False)
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
        except Exception as exc:
            self.query_one("#content", Static).update(
                Text(f"compiler exploration failed: {exc}", style="red")
            )
            self._result = None
            return
        # Recompute the optimised pipeline lazily — only when a lens needs it.
        self._opt = (
            optimised_result(self._result, self._args.dialect)
            if self._opt_mode != "off"
            else None
        )
        self._update_subtitle()
        self.query_one("#summary", Label).update(
            "  ".join(_summary_parts(self._result, self._args.dialect))
        )

    def _update_subtitle(self) -> None:
        opt = "" if self._opt_mode == "off" else f"  ·  opt: {self._opt_mode}"
        self.sub_title = f"{self._args.dialect}{opt}"

    def _show(self, view: str) -> None:
        self._current = view
        if self._result is None:
            return
        # The green tree gets the interactive Tree surface; everything else the
        # captured-ANSI Static.  Toggle which one is visible.
        gt_pane = self.query_one("#gt-pane")
        content_scroll = self.query_one("#content-scroll")
        if view == "greentree":
            content_scroll.display = False
            gt_pane.display = True
            self._populate_greentree()
            # Leave focus on the sidebar so ↑/↓ keep switching views; click or
            # Tab into the tree to navigate/expand its entries.
            return
        gt_pane.display = False
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

    def _populate_greentree(self) -> None:
        """(Re)build the interactive green tree from the shared serialiser."""
        from tooling.cli.serialise import _serialise_greentree

        tree = self.query_one("#gt-tree", Tree)
        tree.clear()
        root = _serialise_greentree(self._source)
        if root is None:
            tree.root.set_label(Text("(green tree unavailable)", style="red"))
            return
        tree.root.set_label(f"{root['kind'].lower()} [{root['mode'].lower()}] w={root['width']}")
        tree.root.data = root
        self._add_gt_tokens(tree.root, root["tokens"])
        tree.root.expand()
        self.query_one("#gt-detail", Static).update(_gt_detail(root))

    def _add_gt_tokens(self, node, tokens: list[dict]) -> None:
        for tok in tokens:
            child = tok.get("child")
            if child is not None:
                branch = node.add(_gt_label(tok), data=tok)
                self._add_gt_tokens(branch, child["tokens"])
            else:
                node.add_leaf(_gt_label(tok), data=tok)

    def on_tree_node_highlighted(self, event: Tree.NodeHighlighted) -> None:
        if event.node.data is not None:
            self.query_one("#gt-detail", Static).update(_gt_detail(event.node.data))

    def action_cycle_opt(self) -> None:
        """Cycle the optimisation lens off → on → diff and re-render."""
        self._opt_mode = _OPT_CYCLE[(_OPT_CYCLE.index(self._opt_mode) + 1) % len(_OPT_CYCLE)]
        if self._opt_mode != "off" and self._opt is None and self._result is not None:
            self._opt = optimised_result(self._result, self._args.dialect)
        self._update_subtitle()
        self._show(self._current)

    def on_list_view_highlighted(self, event: ListView.Highlighted) -> None:
        if event.item is not None and event.item.id and event.item.id.startswith("view-"):
            self._show(event.item.id[len("view-") :])

    def action_refresh(self) -> None:
        # Re-read the file (if any) so the view tracks edits, then recompute.
        if self._args.path and self._args.path != "-":
            with contextlib.suppress(Exception):
                self._source = load_source(self._args.path, None)
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
