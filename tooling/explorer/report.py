"""CLI-agnostic flat-text report for the compiler explorer.

Shared by the explorer's own argparse CLI (:mod:`tooling.explorer.cli`) and
the unified ``tcl explore`` verb (:mod:`tooling.tcl.verbs.misc`).  Centralising
view selection + rendering here means ``tcl`` depends only on the explorer's
*library* layer (this module + :mod:`tooling.explorer.pipeline`), never its
argparse entry point — which would drag in the optional Textual/rich TUI
dependencies that the ``tcl`` zipapp deliberately does not bundle.

This is the ``--text`` surface only.  The ``--tui`` (Textual) and ``--json``
surfaces stay in the CLI adapter, since the TUI is CLI-only and JSON
serialisation already lives next to the pipeline.
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
from tooling.explorer.pipeline import CompilerExplorerResult, run_pipeline

try:
    from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""


@dataclass(frozen=True)
class ExploreOptions:
    """Render options for the flat-text explorer report.

    Built either from the explorer CLI's argparse namespace or directly by the
    ``tcl explore`` verb, so neither path re-implements view selection or
    rendering.  ``opt`` is the optimisation lens (``"off"`` / ``"on"`` /
    ``"diff"``) honoured by the IR/CFG/SSA/ASM/WASM views.
    """

    views: frozenset[str]
    dialect: str = "tcl8.6"
    opt: str = "off"
    show_optimised_source: bool = False
    no_source_callouts: bool = False
    max_annotations: int = 80


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


def run_text_report(
    source: str,
    options: ExploreOptions,
    *,
    use_colour: bool,
) -> int:
    """Compile *source* and print its flat-text report; return an exit code."""
    try:
        result = run_pipeline(source, dialect=options.dialect)
    except Exception as exc:
        print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
        return 2
    render_text_report(result, source, options, use_colour=use_colour)
    return 0
