"""Public formatting API."""

from __future__ import annotations

import logging

from .config import FormatterConfig

# ``format_body`` is the core recursive snippet formatter.  It is part of
# the public surface (re-exported from the package) for callers that need
# to format a fragment at a known indent level — e.g. the LSP
# ``rangeFormatting`` provider — without the document-level normalisation
# (final newline / line-ending rewrite) that :func:`format_tcl` applies.
from .engine import format_body

log = logging.getLogger(__name__)

__all__ = ["format_body", "format_tcl"]

# Upper bound on stabilisation passes (see :func:`format_tcl`).  Valid Tcl is
# already idempotent, so it never iterates; malformed input either reaches a
# fixed point within a couple of passes or is declared un-formattable.
_STABILISE_PASSES = 8


def _format_once(source: str, config: FormatterConfig) -> str:
    """Format *source* a single time (document-level normalisation included)."""
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


def format_tcl(source: str, config: FormatterConfig | None = None) -> str:
    """Format a Tcl source string.

    Pure, and **idempotent**: ``format_tcl(format_tcl(x)) == format_tcl(x)`` for
    every input.  Valid Tcl reaches its formatted form in one pass, so the
    confirming pass returns it immediately.  Structurally malformed input (an
    unbalanced ``{`` / ``[`` / ``"`` the reconstruction cannot round-trip) can
    format non-idempotently — the lexer swallows an unterminated region to EOF
    and the reconstruction fabricates a closer, which a re-parse re-mangles.  So
    iterate to a fixed point, and if none exists within :data:`_STABILISE_PASSES`
    (the reconstruction keeps changing it — formerly an *unbounded* grow-by-one-
    delimiter-per-save), return the input unchanged rather than mangle or grow
    un-formattable code on every save.

    Args:
        source: The Tcl source code to format.
        config: Formatting options. Uses F5 iRules defaults if None.

    Returns:
        The formatted source code (a fixed point of the formatter), or *source*
        unchanged when it cannot be formatted to a stable result.
    """
    if config is None:
        config = FormatterConfig()

    result = _format_once(source, config)
    # Fast path: already a fixed point (all valid Tcl, the overwhelming case).
    if _format_once(result, config) == result:
        return result
    # Otherwise iterate, watching for a fixed point *or* a transform loop: track
    # every output seen so an oscillation (the passes fight between two-or-more
    # forms) is caught immediately instead of running to the cap, and is
    # distinguishable from never-converging growth (each pass strictly larger,
    # which the cap bounds).  Both fall back to the unchanged source.
    seen = {source, result}
    current = result
    for _ in range(_STABILISE_PASSES):
        nxt = _format_once(current, config)
        if nxt == current:
            return current  # fixed point
        if nxt in seen:
            log.warning(
                "format_tcl did not converge: the formatting passes form a "
                "transform loop on this input; returning it unchanged."
            )
            return source
        seen.add(nxt)
        current = nxt
    # No fixed point and no loop within the cap — the input is not cleanly
    # formattable (delimiter imbalance the reconstruction keeps growing); leave
    # it as-is so repeated formatting is stable (returns the same source).
    log.debug(
        "format_tcl did not converge within %d passes; returning input unchanged.",
        _STABILISE_PASSES,
    )
    return source
