# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
from .recovery import detect_recovery

__all__ = ["tokenise_recovering"]

# Structural tokens that are never the swallowing tail of an open delimiter.
_NON_CONTENT_TYPES = (TokenType.SEP, TokenType.EOL, TokenType.EOF)


def _recovery_possible(
    tokens: tuple[Token, ...],
    source: str,
    base_offset: int,
) -> bool:
    """Sound (over-approximate) test: could this stream need recovery?

    Every recovery detector — unterminated ``[`` (``_is_unterminated_cmd``),
    suspicious ``"`` (``_is_suspicious_quote``) and suspicious ``{``
    (``_is_suspicious_str``) — has a necessary precondition that the offending
    *command* runs to the end of the region: the lexer only fails to find a
    closer at EOF, and the quote/brace heuristics explicitly require the command
    to reach EOF.  So if no content token reaches EOF, ``detect_recovery`` is
    guaranteed to find nothing and the bare parse already equals the recovered
    parse.

    Checking "some content token reaches EOF" is therefore a *sound* superset:
    never a false negative (it cannot skip a real recovery), and at worst a
    false positive for a word that legitimately ends at EOF (e.g. a file with no
    trailing newline) — which only routes to the precise detection that then
    inserts nothing.  Note the tail need not be a delimiter token: an
    unterminated ``"`` can end in a ``VAR``/``CMD`` substitution, and a
    suspicious ``"`` may even close at a later stray quote yet still leave the
    command reaching EOF.
    """
    if not source:
        return False
    eof_local = len(source) - 1
    for tok in tokens:
        if tok.type in _NON_CONTENT_TYPES:
            continue
        if tok.end.offset - base_offset >= eof_local:
            return True
    return False


def tokenise_recovering(
    source: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
    *,
    body_token: Token | None = None,
    line_starts: list[int] | None = None,
) -> tuple[tuple[Token, ...], object]:
    """Tokenise *source*, recovering unterminated delimiters.

    Drop-in for the two-pass recovery a consumer would otherwise spell out as
    ``detect_recovery(source, body_token).insertions`` followed by
    ``tokenise(source, ..., virtual_insertions=vi)`` — returns the same
    ``(tokens, warnings)``, verified by the differential oracle.

    *body_token* anchors recovery when *source* is a body substring (a braced
    proc/if/while body etc.): its position seeds the base offset the recovery
    heuristics reason about, exactly as ``detect_recovery`` uses it.
    """
    tokens, warnings = tokenise(source, base_offset, base_line, base_col, line_starts=line_starts)
    # Single-pass fast path: at top level the bare parse and the recovered parse
    # coincide whenever nothing is left unterminated, so return the bare stream
    # untouched with no recovery work.  (Mid-stream lexer warnings, e.g. "extra
    # characters after close-quote", do not change the *tokens*, so they don't
    # disqualify this path for the token-only caller.)  Inside a body,
    # segmentation runs under the body's mode/anchoring, which can surface an
    # unterminated delimiter the bare top-level lex does not — so always detect.
    if body_token is None and not _recovery_possible(tokens, source, base_offset):
        return tokens, warnings
    det = detect_recovery(source, body_token)
    if not det.insertions:
        return tokens, warnings
    return tokenise(
        source,
        base_offset,
        base_line,
        base_col,
        virtual_insertions=det.insertions,
        line_starts=line_starts,
    )
