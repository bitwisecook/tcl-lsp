"""Shared flat-text surface for the compiler explorer.

This is the one library both flat-text consumers import from — the explorer's
own argparse CLI (:mod:`tooling.explorer.cli`) and the unified ``tcl explore``
verb (:mod:`tooling.tcl.verbs.misc`).  It owns exactly the parts those two
share: view selection (:func:`resolve_views`), the render options
(:class:`ExploreOptions`), and the compile-and-print entry point
(:func:`run_text_report`) — including the colour/TTY policy, so neither caller
re-derives it.

By depending only on the pipeline + render *libraries* (never argparse or the
Textual TUI), this keeps the ``tcl`` command free of the explorer's CLI import
surface — which is what let the optional rich/textual TUI deps (absent from the
``tcl`` zipapp) leak in before.  The ``--tui`` (Textual) and ``--json``
surfaces stay in the CLI adapter, since the TUI is CLI-only and JSON
serialisation already lives beside the pipeline.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass

from tooling.cli.formatters import LineIndex
from tooling.explorer._render import (
    _VIEW_ORDER,
    Ansi,
    _summary_parts,
    optimised_result,
    render_view_opt,
    style,
)
from tooling.explorer.pipeline import (
    ALL_VIEWS,
    VIEW_GROUPS,
    CompilerExplorerResult,
    run_pipeline,
)

try:
    from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""


def resolve_views(raw: str) -> frozenset[str]:
    """Expand a comma-separated ``--show`` value into a set of view names.

    Raises :class:`ValueError` (not ``argparse``) for an unknown view, so both
    the explorer's argparse CLI and the ``tcl explore`` verb share view
    selection without coupling either to argparse.  Tokens may name an
    individual view (see :data:`~tooling.explorer.pipeline.ALL_VIEWS`) or a
    group (see :data:`~tooling.explorer.pipeline.VIEW_GROUPS`); empty tokens are
    ignored.
    """
    views: set[str] = set()
    for token in raw.split(","):
        token = token.strip()
        if not token:
            continue
        if token in VIEW_GROUPS:
            views |= VIEW_GROUPS[token]
        elif token in ALL_VIEWS:
            views.add(token)
        else:
            raise ValueError(
                f"unknown view {token!r}; choose from: "
                f"{', '.join(sorted(ALL_VIEWS | set(VIEW_GROUPS)))}"
            )
    return frozenset(views)


@dataclass(frozen=True)
class ExploreOptions:
    """Render options for the flat-text explorer report.

    Built either from the explorer CLI's argparse namespace or directly by the
    ``tcl explore`` verb, so neither path re-implements view selection or
    rendering.  ``opt`` is the optimisation lens (``"off"`` / ``"on"`` /
    ``"diff"``) honoured by the IR/CFG/SSA/ASM/WASM views.  ``no_colour``
    requests plain output; the actual colour decision also depends on whether
    stdout is a TTY (see :func:`run_text_report`).
    """

    views: frozenset[str]
    dialect: str = "tcl8.6"
    opt: str = "off"
    show_optimised_source: bool = False
    no_source_callouts: bool = False
    max_annotations: int = 80
    no_colour: bool = False


def render_text_report(
    result: CompilerExplorerResult,
    source: str,
    options: ExploreOptions,
    *,
    use_colour: bool,
) -> None:
    """Print the flat-text explorer report (to stdout) for a computed *result*."""
    line_index = LineIndex(source)
    version = FULL_VERSION + (f" ({BUILD_TIMESTAMP})" if BUILD_TIMESTAMP else "")
    print(style(f"compiler-optimiser-explorer {version}", Ansi.BOLD, use_colour))
    print(style(" ".join(_summary_parts(result, options.dialect)), Ansi.DIM, use_colour))
    opt_label = "" if options.opt == "off" else f"  opt={options.opt}"
    print(style(f"views: {','.join(sorted(options.views))}{opt_label}", Ansi.DIM, use_colour))
    print()
    opt = optimised_result(result, options.dialect) if options.opt != "off" else None
    for view in _VIEW_ORDER:
        if view not in options.views:
            continue
        if view == "callouts" and options.no_source_callouts:
            continue
        render_view_opt(
            view,
            result,
            source,
            use_colour=use_colour,
            line_index=line_index,
            opt_mode=options.opt,
            opt=opt,
            views=options.views,
            show_optimised_source=options.show_optimised_source,
            max_annotations=options.max_annotations,
        )
        print()


def run_text_report(source: str, options: ExploreOptions) -> int:
    """Compile *source* and print its flat-text report; return an exit code.

    Colour is enabled only when not suppressed *and* stdout is a TTY — the
    single home for that policy, shared by every flat-text caller.
    """
    use_colour = (not options.no_colour) and sys.stdout.isatty()
    try:
        result = run_pipeline(source, dialect=options.dialect)
    except Exception as exc:
        print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
        return 2
    render_text_report(result, source, options, use_colour=use_colour)
    return 0
