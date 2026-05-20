"""Public formatting API."""

from __future__ import annotations

from .config import FormatterConfig

# ``format_body`` is the core recursive snippet formatter.  It is part of
# the public surface (re-exported from the package) for callers that need
# to format a fragment at a known indent level — e.g. the LSP
# ``rangeFormatting`` provider — without the document-level normalisation
# (final newline / line-ending rewrite) that :func:`format_tcl` applies.
from .engine import format_body

__all__ = ["format_body", "format_tcl"]


def format_tcl(source: str, config: FormatterConfig | None = None) -> str:
    """Format a Tcl source string.

    Pure function: source in, formatted source out.

    Args:
        source: The Tcl source code to format.
        config: Formatting options. Uses F5 iRules defaults if None.

    Returns:
        The formatted source code.
    """
    if config is None:
        config = FormatterConfig()

    result = format_body(source, config, indent_level=0)

    # Trim trailing whitespace per line
    if config.trim_trailing_whitespace:
        result = "\n".join(line.rstrip() for line in result.split("\n"))

    # Ensure final newline
    if config.ensure_final_newline:
        if not result.endswith("\n"):
            result += "\n"

    # Normalise line endings
    if config.line_ending != "\n":
        result = result.replace("\n", config.line_ending)

    return result
