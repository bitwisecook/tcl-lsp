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
from textual.containers import Horizontal, VerticalScroll
from textual.widgets import Footer, Header, Label, ListItem, ListView, Static

from tooling.explorer.cli import (
    _VIEW_ORDER,
    LineIndex,
    _summary_parts,
    load_source,
    render_view,
)
from tooling.explorer.pipeline import CompilerExplorerResult, run_pipeline


def _render_view_ansi(
    view: str, result: CompilerExplorerResult, source: str, args: argparse.Namespace
) -> Text:
    """Capture ``render_view``'s ANSI output for *view* and parse it for Rich."""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        try:
            render_view(
                view,
                result,
                source,
                use_colour=True,
                line_index=LineIndex(source),
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
    """

    BINDINGS = [
        ("q", "quit", "Quit"),
        ("r", "refresh", "Re-run"),
    ]

    def __init__(self, source: str, args: argparse.Namespace) -> None:
        super().__init__()
        self._source = source
        self._args = args
        self._result: CompilerExplorerResult | None = None
        self._views = [v for v in _VIEW_ORDER if v in args.views]
        self._current = self._views[0] if self._views else "ir"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        yield Label("", id="summary")
        with Horizontal():
            with VerticalScroll(id="sidebar"):
                yield ListView(*(ListItem(Label(v), id=f"view-{v}") for v in self._views))
            with VerticalScroll():
                yield Static("", id="content", markup=False)
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
        self.sub_title = self._args.dialect
        self.query_one("#summary", Label).update(
            "  ".join(_summary_parts(self._result, self._args.dialect))
        )

    def _show(self, view: str) -> None:
        self._current = view
        if self._result is None:
            return
        content = self.query_one("#content", Static)
        content.update(_render_view_ansi(view, self._result, self._source, self._args))

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
