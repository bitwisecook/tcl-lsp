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

"""Position-independent *green* layer of the Tcl concrete syntax tree (CST).

This is the canonical, lossless syntax representation the whole pipeline is
meant to ride on — the segmenter, AOT lowering to the bytecode VM and WASM, the
formatter, the minifier, and the per-command ``tcl`` / ``f5 irule`` tooling.
Today each of those re-lexes the bytes it cares about; the CST replaces that
with one tree everyone descends.

The design is the classic red-green split (Roslyn / rust-analyzer):

* **Green** (this module) is *position-independent*: a node knows only its
  *width* and its children, never an absolute offset.  Two structurally
  identical regions (e.g. two empty ``{}`` words) can therefore be the same
  object, and a green subtree is reusable verbatim across edits.
* **Red** (:mod:`compiler.parsing.syntax.red`) overlays a green tree with an
  anchoring and resolves absolute positions lazily, reproducing exactly the
  ``Token`` offsets/lines/columns the lexer emits.

Trivia (whitespace, end-of-line, comments) is **attached** to the adjacent
token as leading/trailing trivia rather than living as sibling tokens, so a
``Command`` / ``Word`` is pure syntax while the bytes between words are still
preserved.  Concatenating every element's :pyattr:`full_text` in document order
reproduces the source byte-for-byte — the losslessness the formatter and
minifier require.

See ``docs/design/compiler/syntax-tree.md``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Union

from shared.tokens import TokenType


class TriviaKind(Enum):
    """A piece of inter-token text that carries no syntactic value of its own."""

    WHITESPACE = auto()  # intra-line separators (lexer SEP, backslash-newline)
    EOL = auto()  # newline(s) / ``;`` command terminator (lexer EOL)
    COMMENT = auto()  # ``#`` … end-of-line comment (lexer COMMENT)


class SyntaxKind(Enum):
    """Structural role of an interior :class:`GreenNode`."""

    DOCUMENT = auto()  # the whole region: a sequence of commands
    COMMAND = auto()  # one Tcl command (one logical line)
    WORD = auto()  # one Tcl word: one or more abutting fragments


@dataclass(frozen=True, slots=True)
class GreenTrivia:
    """One whitespace / end-of-line / comment run, stored by its literal text."""

    kind: TriviaKind
    text: str

    @property
    def width(self) -> int:
        return len(self.text)


@dataclass(frozen=True, slots=True)
class GreenToken:
    """A leaf: one lexer word-fragment, with attached leading/trailing trivia.

    ``token_type`` is the lexer :class:`TokenType` of the fragment
    (``ESC`` / ``STR`` / ``CMD`` / ``VAR`` / ``EXPAND``); the trivia kinds
    ``SEP`` / ``EOL`` / ``COMMENT`` never appear here — they become
    :class:`GreenTrivia`.

    Two text fields are kept because both are needed and neither is derivable
    from the other without re-encoding the lexer's delimiter conventions:

    * :pyattr:`text` is the lexer's *inner* text (``abc`` for ``{abc}``), what
      ``Token.text`` carries downstream.
    * :pyattr:`raw` is the full source slice the fragment occupies
      (``{abc}``), what makes the tree lossless and defines :pyattr:`width`.

    :pyattr:`end_rel` is ``Token.end.offset - Token.start.offset`` — the
    position-independent shape that lets the red layer reconstruct the lexer's
    ``end`` (which, by the #527 convention, sits on the last *inner* character
    for a non-empty ``{…}`` but *on the closer* for an empty ``{}``).
    """

    token_type: TokenType
    text: str
    raw: str
    end_rel: int
    in_quote: bool = False
    leading: tuple[GreenTrivia, ...] = ()
    trailing: tuple[GreenTrivia, ...] = ()
    full_width: int = field(init=False, default=0, compare=False, repr=False)

    def __post_init__(self) -> None:
        lead = sum(t.width for t in self.leading)
        trail = sum(t.width for t in self.trailing)
        object.__setattr__(self, "full_width", lead + len(self.raw) + trail)

    @property
    def width(self) -> int:
        """Narrow width: just the fragment's own bytes."""
        return len(self.raw)

    @property
    def leading_width(self) -> int:
        return sum(t.width for t in self.leading)

    @property
    def trailing_width(self) -> int:
        return sum(t.width for t in self.trailing)

    @property
    def full_text(self) -> str:
        return (
            "".join(t.text for t in self.leading)
            + self.raw
            + "".join(t.text for t in self.trailing)
        )


GreenElement = Union["GreenNode", GreenToken]


@dataclass(frozen=True, slots=True)
class GreenNode:
    """An interior node: a ``DOCUMENT``, ``COMMAND``, or ``WORD``.

    ``expand_markers`` are the ``{*}`` ``EXPAND`` leaves when this is an expanded
    word; they precede the word's fragments in document order but are *not*
    ``children`` (the segmenter reports the expanded word's representative token
    as the first real fragment, not a ``{*}`` marker).  Normally there is one,
    but ``{*}{*}x`` lexes as two adjacent markers, so it is a tuple.

    ``trailing`` carries document-level dangling trivia — a trailing comment
    with no following command (``puts hi ;# bye``) attaches to nothing yet must
    still round-trip; it is appended here on the ``DOCUMENT`` node.
    """

    kind: SyntaxKind
    children: tuple[GreenElement, ...]
    expand_markers: tuple[GreenToken, ...] = ()
    trailing: tuple[GreenTrivia, ...] = ()
    # COMMAND only: byte offset of the command's faithful range end, relative to
    # its first token's raw start (so it stays position-independent).  Captures
    # the segmenter's word-boundary rule — including the stale-boundary quirk a
    # trailing ``{*}`` introduces — which is not derivable from node geometry.
    range_end_rel: int | None = None
    # COMMAND only: the comment(s) attached forward to this command, captured by
    # the segmenter's accumulation rule (which crosses dangling-{*} commands), so
    # it cannot be read off this node's own leading trivia.
    preceding_comment: str | None = None
    _full_width: int = field(init=False, default=0, compare=False, repr=False)

    def __post_init__(self) -> None:
        width = sum(c.full_width for c in self.children)
        width += sum(m.full_width for m in self.expand_markers)
        width += sum(t.width for t in self.trailing)
        object.__setattr__(self, "_full_width", width)

    @property
    def is_expand(self) -> bool:
        return bool(self.expand_markers)

    @property
    def full_width(self) -> int:
        return self._full_width

    @property
    def full_text(self) -> str:
        head = "".join(m.full_text for m in self.expand_markers)
        body = "".join(c.full_text for c in self.children)
        tail = "".join(t.text for t in self.trailing)
        return head + body + tail


# Light interning of trivia: whitespace runs and single newlines dominate a
# document and are position-independent, so sharing them is free memory.  The
# cache is unbounded but trivia variety is tiny (a handful of indentation
# widths), so it stays small in practice.
_trivia_cache: dict[tuple[TriviaKind, str], GreenTrivia] = {}


def trivia(kind: TriviaKind, text: str) -> GreenTrivia:
    """Return a shared :class:`GreenTrivia` for *(kind, text)*."""
    key = (kind, text)
    hit = _trivia_cache.get(key)
    if hit is None:
        hit = GreenTrivia(kind, text)
        _trivia_cache[key] = hit
    return hit
