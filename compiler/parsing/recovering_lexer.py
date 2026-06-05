"""Single-pass recovering tokeniser — public entry point.

Today error recovery is a *two-pass* affair: ``compute_virtual_insertions``
parses the source once to locate unterminated ``[``/``{``/``"`` delimiters and
decide where a virtual closer belongs, then the real tokenise pass re-runs with
those zero-width insertions.  Both the analyser and the semantic-token provider
consume that recovered stream, while the chunk cache historically consumed the
*bare* parse — the divergence behind a class of stale-cache bugs.

This module is the seam for collapsing that into a single pass: one tokenise
that recovers inline as it goes.  It is being grown incrementally behind a
differential oracle (``tests/test_recovering_lexer_differential.py``) that pins
it, token-for-token and warning-for-warning, to the existing two-pass output —
so consumers can migrate onto it the moment parity holds, with no behaviour
change.

The well-formed path is already genuinely single-pass: when the first tokenise
produces no unterminated delimiter, ``vi`` is necessarily empty, so the bare
stream *is* the recovered stream and we return it directly.  Only a source that
actually carries an unterminated delimiter takes the (currently delegated)
recovery path, which subsequent commits replace with inline lexer recovery.
"""

from __future__ import annotations

from shared.tokens import Token, TokenType

from .green_tree import tokenise
from .recovery import compute_virtual_insertions

__all__ = ["tokenise_recovering"]

# Delimiter token types whose unterminated form (reaching EOF without a closer)
# is what error recovery acts on.
_DELIMITER_TYPES = (TokenType.CMD, TokenType.STR, TokenType.ESC)


def _has_unterminated_delimiter(
    tokens: tuple[Token, ...],
    source: str,
    base_offset: int,
) -> bool:
    """Cheap superset test: could this stream need recovery?

    An unterminated delimiter always shows up as a token group that runs to the
    end of *source* (the lexer consumes to EOF when no closer is found).  If no
    token reaches EOF inside an open delimiter, ``compute_virtual_insertions``
    would find nothing, so the bare parse already equals the recovered parse.

    The EOF-reaching token may not itself be a delimiter token: an unterminated
    ``"`` whose tail is a substitution ends in a ``VAR``/``CMD`` token carrying
    ``in_quote`` (the quote is still open), and an unterminated ``[`` ends in a
    ``CMD``.  So the signal is "an EOF-reaching token that is a delimiter type
    *or* still inside a quote".  This is a deliberate *superset*: it may answer
    ``True`` for a bare word that merely ends at EOF, which only routes to the
    precise recovery path (that then inserts nothing) — never a false negative.
    """
    if not source:
        return False
    eof_local = len(source) - 1
    for tok in tokens:
        if tok.end.offset - base_offset < eof_local:
            continue
        if tok.type in _DELIMITER_TYPES or tok.in_quote:
            return True
    return False


def tokenise_recovering(
    source: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
    *,
    line_starts: list[int] | None = None,
) -> tuple[tuple[Token, ...], object]:
    """Tokenise *source*, recovering unterminated delimiters inline.

    Returns ``(tokens, warnings)`` identical to the two-pass recovery
    (``tokenise(source, virtual_insertions=compute_virtual_insertions(source))``)
    — verified by the differential oracle.
    """
    tokens, warnings = tokenise(source, base_offset, base_line, base_col, line_starts=line_starts)
    if not _has_unterminated_delimiter(tokens, source, base_offset):
        # Well-formed (or trivially closed at EOF): the bare parse is already the
        # recovered parse.  Genuinely single-pass.
        return tokens, warnings
    # Recovery needed.  Until inline lexer recovery lands for every delimiter
    # type, fall back to the proven two-pass so the oracle stays green.
    vi = compute_virtual_insertions(source) or None
    if vi is None:
        # No insertion after all (heuristics declined) — the bare parse stands.
        return tokens, warnings
    return tokenise(
        source,
        base_offset,
        base_line,
        base_col,
        virtual_insertions=vi,
        line_starts=line_starts,
    )
