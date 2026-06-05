"""A client-side mirror of an LSP text document, for edit-tracking stress tests.

The server tracks an open document by applying each ``didChange`` content
change to its own buffer.  To *punish* that tracking we need to drive the
same incremental edits the server applies and independently know the text
that should result — then compare the two views (via a feature whose output
is a pure function of the buffer, e.g. diagnostics or document symbols).

``TextMirror`` is that independent oracle.  It applies LSP range edits using
the same UTF-16 position semantics the protocol mandates (the default
position encoding), so an edit the test computes against the mirror lands on
exactly the same characters the server's codec resolves.  This is the only
place a second copy of the apply-change algorithm is allowed to live: it
exists precisely so a divergence between it and the server is a test failure.
"""

from __future__ import annotations

import random
from dataclasses import dataclass


def _utf16_len(s: str) -> int:
    """Length of ``s`` in UTF-16 code units (astral chars count as two)."""
    return sum(2 if ord(ch) > 0xFFFF else 1 for ch in s)


@dataclass
class Edit:
    """One LSP incremental edit: replace ``[start, end)`` with ``text``.

    Positions are ``(line, utf16_character)`` pairs in the document's
    coordinate space *before* the edit is applied.
    """

    start: tuple[int, int]
    end: tuple[int, int]
    text: str

    def as_content_change(self) -> dict:
        return {
            "range": {
                "start": {"line": self.start[0], "character": self.start[1]},
                "end": {"line": self.end[0], "character": self.end[1]},
            },
            "text": self.text,
        }


class TextMirror:
    """A document whose text mutates exactly as the server's buffer should."""

    def __init__(self, text: str = "") -> None:
        self.text = text

    # -- coordinate helpers ------------------------------------------------

    @property
    def lines(self) -> list[str]:
        # Keep line terminators implicit; split on '\n' the way LSP lines work.
        return self.text.split("\n")

    def _offset(self, line: int, character: int) -> int:
        """Resolve a UTF-16 ``(line, character)`` to a Python string offset."""
        lines = self.text.split("\n")
        line = max(0, min(line, len(lines) - 1))
        offset = 0
        for i in range(line):
            offset += len(lines[i]) + 1  # +1 for the '\n'
        # Walk the target line counting UTF-16 code units until we reach character.
        col_u16 = 0
        within = lines[line]
        idx = 0
        while idx < len(within) and col_u16 < character:
            col_u16 += 2 if ord(within[idx]) > 0xFFFF else 1
            idx += 1
        return offset + idx

    def position_at(self, offset: int) -> tuple[int, int]:
        """Inverse of ``_offset``: Python offset -> ``(line, utf16_char)``."""
        prefix = self.text[:offset]
        line = prefix.count("\n")
        last_nl = prefix.rfind("\n")
        col = prefix[last_nl + 1 :]
        return line, _utf16_len(col)

    # -- mutation ----------------------------------------------------------

    def apply(self, edit: Edit) -> None:
        if edit.start == edit.end == (0, 0) and edit.text == self.text:
            return
        start = self._offset(*edit.start)
        end = self._offset(*edit.end)
        self.text = self.text[:start] + edit.text + self.text[end:]

    def apply_all(self, edits: list[Edit]) -> None:
        for edit in edits:
            self.apply(edit)


def random_edit(mirror: TextMirror, rng: random.Random) -> Edit:
    """Produce one plausible editor edit against ``mirror``'s current text.

    The mix is deliberately hostile to naive trackers: zero-width insertions,
    multi-character deletions, replacements that span line boundaries, edits
    at the very end of the buffer, and newline insertion/removal.
    """
    text = mirror.text
    n = len(text)
    kind = rng.random()
    if n == 0 or kind < 0.45:
        # Insertion at a random offset (possibly end-of-buffer, zero-width).
        off = rng.randint(0, n)
        pos = mirror.position_at(off)
        snippet = rng.choice(
            ["x", "set y 1\n", "}", "{", "  ", "\n", "puts $z", "# c\n", "é", "\U0001f600"]
        )
        return Edit(pos, pos, snippet)
    if kind < 0.8 and n > 1:
        # Deletion / replacement of a random span.
        a = rng.randint(0, n - 1)
        b = rng.randint(a + 1, min(n, a + 1 + rng.randint(1, 12)))
        start = mirror.position_at(a)
        end = mirror.position_at(b)
        replacement = "" if rng.random() < 0.6 else rng.choice(["q", "\n", "Z9"])
        return Edit(start, end, replacement)
    # Append at end of buffer.
    pos = mirror.position_at(n)
    return Edit(pos, pos, rng.choice(["\nset w 0", "puts done\n", ";"]))
