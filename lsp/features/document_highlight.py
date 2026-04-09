"""Document highlight provider -- highlight all occurrences of the symbol at cursor.

Delegates to :func:`get_references` and filters to the current document so the
editor can highlight in-file matches without issuing a workspace-wide request.
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.semantic_model import AnalysisResult

from .references import get_references


def get_document_highlights(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.DocumentHighlight]:
    """Return highlight ranges for the symbol at ``(line, character)``.

    Returns an empty list when no symbol is under the cursor.  All returned
    ranges are kind ``Text``; distinguishing Read/Write is a future refinement.
    """
    locations = get_references(
        source,
        uri,
        line,
        character,
        analysis=analysis,
        include_declaration=True,
    )
    highlights: list[types.DocumentHighlight] = []
    seen: set[tuple[int, int, int, int]] = set()
    for loc in locations:
        if loc.uri != uri:
            continue
        key = (
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        )
        if key in seen:
            continue
        seen.add(key)
        highlights.append(
            types.DocumentHighlight(
                range=loc.range,
                kind=types.DocumentHighlightKind.Text,
            )
        )
    return highlights
