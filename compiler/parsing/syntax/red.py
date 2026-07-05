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

"""The *red* layer: lazy absolute positions over a position-independent green tree.

A :class:`GreenNode` knows only widths.  A :class:`SyntaxTree` anchors one at an
absolute ``(offset, line, column)`` and resolves positions on demand: a node's
absolute start is its parent's start plus the full width of the siblings before
it, and line/column come from a line index built from the green tree's *own*
reconstructed text — so the red layer needs no reference to the original source
and reproduces exactly the positions the lexer emits.

The anchoring mirrors the lexer's ``base_offset`` / ``base_line`` / ``base_col``
contract: ``base_col`` shifts only the first line of the region (everything
after the first newline starts at column 0), exactly as
:class:`compiler.parsing.lexer.TclLexer` does in ``_pos_at``.
"""

from __future__ import annotations

from bisect import bisect_right
from typing import TYPE_CHECKING

from shared.tokens import SourcePosition, Token

from .green import GreenNode, GreenToken

if TYPE_CHECKING:
    from collections.abc import Iterator


def build_line_starts(text: str) -> list[int]:
    """Byte offsets at which each line begins (line 0 at offset 0).

    Identical to the lexer's own index, so a caller that has already built one
    (the segmenter) passes it to both the lexer and the red layer to avoid the
    O(n) rebuild.
    """
    starts = [0]
    for i, ch in enumerate(text):
        if ch == "\n":
            starts.append(i + 1)
    return starts


class SyntaxTree:
    """A green :class:`GreenNode` anchored at an absolute position.

    ``root`` is the :class:`SyntaxNode` for the green ``DOCUMENT``; walking it
    yields :class:`SyntaxNode` / :class:`SyntaxToken` views that compute their
    absolute positions lazily through this anchoring.
    """

    __slots__ = ("green", "base_offset", "base_line", "base_col", "_line_starts", "_text")

    def __init__(
        self,
        green: GreenNode,
        base_offset: int = 0,
        base_line: int = 0,
        base_col: int = 0,
        *,
        text: str | None = None,
        line_starts: list[int] | None = None,
    ) -> None:
        self.green = green
        self.base_offset = base_offset
        self.base_line = base_line
        self.base_col = base_col
        # The region text equals the green tree's reconstruction (losslessness);
        # callers that already hold it (and its line index) pass them to skip the
        # O(n) rebuilds.
        self._text = green.full_text if text is None else text
        self._line_starts = build_line_starts(self._text) if line_starts is None else line_starts

    @property
    def text(self) -> str:
        """The region's reconstructed source text (lossless)."""
        return self._text

    @property
    def root(self) -> SyntaxNode:
        return SyntaxNode(self, self.green, None, 0)

    def position_at(self, region_offset: int) -> SourcePosition:
        """Resolve a region-relative byte offset to an absolute position.

        Mirrors ``TclLexer._pos_at``: the line index is region-relative, then
        the anchoring shifts line by ``base_line`` and the *first* line's column
        by ``base_col``.
        """
        starts = self._line_starts
        line_idx = bisect_right(starts, region_offset) - 1
        col = region_offset - starts[line_idx]
        return SourcePosition(
            line=self.base_line + line_idx,
            character=(self.base_col + col) if line_idx == 0 else col,
            offset=self.base_offset + region_offset,
        )


class SyntaxToken:
    """A red view of a :class:`GreenToken` at a resolved region offset."""

    __slots__ = ("tree", "green", "parent", "_full_start")

    def __init__(
        self,
        tree: SyntaxTree,
        green: GreenToken,
        parent: SyntaxNode | None,
        full_start: int,
    ) -> None:
        self.tree = tree
        self.green = green
        self.parent = parent
        self._full_start = full_start

    @property
    def token_type(self):  # noqa: ANN201 - re-exports the green TokenType
        return self.green.token_type

    @property
    def text(self) -> str:
        return self.green.text

    @property
    def raw(self) -> str:
        return self.green.raw

    @property
    def in_quote(self) -> bool:
        return self.green.in_quote

    @property
    def raw_start(self) -> int:
        """Region offset of the fragment's first byte (past its leading trivia)."""
        return self._full_start + self.green.leading_width

    @property
    def start_position(self) -> SourcePosition:
        return self.tree.position_at(self.raw_start)

    @property
    def end_position(self) -> SourcePosition:
        """Absolute ``Token.end`` — the lexer's inner-end convention (#527)."""
        return self.tree.position_at(self.raw_start + self.green.end_rel)

    def to_token(self) -> Token:
        """Reconstruct the lexer :class:`Token` this fragment came from."""
        return Token(
            type=self.green.token_type,
            text=self.green.text,
            start=self.start_position,
            end=self.end_position,
            in_quote=self.green.in_quote,
        )


class SyntaxNode:
    """A red view of an interior :class:`GreenNode` at a resolved region offset."""

    __slots__ = ("tree", "green", "parent", "_full_start")

    def __init__(
        self,
        tree: SyntaxTree,
        green: GreenNode,
        parent: SyntaxNode | None,
        full_start: int,
    ) -> None:
        self.tree = tree
        self.green = green
        self.parent = parent
        self._full_start = full_start

    @property
    def kind(self):  # noqa: ANN201 - re-exports the green SyntaxKind
        return self.green.kind

    @property
    def full_start(self) -> int:
        return self._full_start

    def expand_markers(self) -> list[SyntaxToken]:
        """The ``{*}`` markers preceding this word's fragments, anchored."""
        out: list[SyntaxToken] = []
        offset = self._full_start
        for marker in self.green.expand_markers:
            out.append(SyntaxToken(self.tree, marker, self, offset))
            offset += marker.full_width
        return out

    def children(self) -> Iterator[SyntaxNode | SyntaxToken]:
        """Yield child red views, each anchored at its resolved region offset."""
        offset = self._full_start
        for marker in self.green.expand_markers:
            offset += marker.full_width
        for child in self.green.children:
            if isinstance(child, GreenToken):
                yield SyntaxToken(self.tree, child, self, offset)
            else:
                yield SyntaxNode(self.tree, child, self, offset)
            offset += child.full_width

    def tokens(self) -> Iterator[SyntaxToken]:
        """Yield every :class:`SyntaxToken` under this node, depth-first."""
        yield from self.expand_markers()
        for child in self.children():
            if isinstance(child, SyntaxToken):
                yield child
            else:
                yield from child.tokens()

    @property
    def start_position(self) -> SourcePosition:
        """Absolute start, *excluding* this node's leading trivia."""
        markers = self.green.expand_markers
        lead = markers[0].leading_width if markers else _first_leading_width(self.green)
        return self.tree.position_at(self._full_start + lead)


def _first_leading_width(green: GreenNode) -> int:
    """Leading-trivia width of the first fragment under *green* (recursively)."""
    for child in green.children:
        if isinstance(child, GreenToken):
            return child.leading_width
        return _first_leading_width(child)
    return 0
