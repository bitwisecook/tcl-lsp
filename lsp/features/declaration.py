"""Go-to-declaration provider.

For variables: return the first ``variable``/``global``/``upvar`` declaration
that names the target variable.  Falls back to :func:`get_definition` when no
such declaration exists.  For procs: identical to definition.
"""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Range
from core.common.lsp import to_lsp_location
from core.parsing.tokens import SourcePosition

from .definition import get_definition
from .symbol_resolution import find_var_at_position

# Matches `variable NAME` / `global NAME` / `upvar ?level? OTHER NAME` statements.
# The name is captured as group 1.
_DECL_RE = re.compile(
    r"\b(variable|global|upvar)\b(?P<tail>[^\n;]*)",
    re.IGNORECASE,
)


def _scan_declarations(source: str, var_name: str) -> list[Range]:
    """Return ranges of every declaration site for ``var_name`` in ``source``.

    Handles ``variable``, ``global``, and ``upvar`` command forms.  The range
    points at the variable name token, not the declaring command.
    """
    stripped = var_name.lstrip(":")
    results: list[Range] = []
    for line_no, line in enumerate(source.splitlines()):
        m = _DECL_RE.search(line)
        if not m:
            continue
        kind = m.group(1).lower()
        tail = m.group("tail")
        tail_start_col = m.end("tail") - len(tail)
        words_with_cols: list[tuple[str, int]] = [
            (t.group(0), tail_start_col + t.start()) for t in re.finditer(r"\S+", tail)
        ]
        if not words_with_cols:
            continue
        # Narrow to the names that are actually declared (not command args
        # like upvar's *level* and *other*).
        candidates: list[tuple[str, int]] = []
        if kind in ("variable", "global"):
            candidates = list(words_with_cols)
        elif kind == "upvar":
            # upvar ?level? ?otherVar myVar? ...
            # The declared variables are every *other* word starting at
            # index 1 (or 0 if the first word is not a level).
            rest = words_with_cols
            # Skip an optional numeric level.
            if rest and (rest[0][0].isdigit() or rest[0][0] in {"#0", "#1", "#2"}):
                rest = rest[1:]
            # Take every second word starting from index 1 (myVar positions).
            for i in range(1, len(rest), 2):
                candidates.append(rest[i])
        for name_token, col_start in candidates:
            # Strip $ and namespace prefix.
            bare = name_token.lstrip("$").lstrip(":")
            if bare == stripped:
                results.append(
                    Range(
                        start=SourcePosition(
                            line=line_no, character=col_start, offset=0
                        ),
                        end=SourcePosition(
                            line=line_no,
                            character=col_start + len(name_token),
                            offset=0,
                        ),
                    )
                )
    return results


def get_declaration(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.Location]:
    """Return declaration locations for the symbol at ``(line, character)``."""
    if analysis is None:
        analysis = analyse(source)

    var_name = find_var_at_position(source, line, character)
    if var_name:
        ranges = _scan_declarations(source, var_name)
        if ranges:
            return [to_lsp_location(uri, r) for r in ranges]
    # Fall back to regular definition for procs / classes / variables without
    # an explicit declaration.
    return get_definition(source, uri, line, character, analysis=analysis)
