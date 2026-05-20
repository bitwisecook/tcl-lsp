"""Cross-document go-to-definition fires only on command heads.

The cross-document proc fallback in ``on_definition`` is gated on the
cursor sitting on a command head (a call site) via the analyser's
``command_invocations``, so an argument word that collides with a
sibling proc/class name in another file does not produce a
false-positive jump.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from server.server import _position_is_command_head

# ``foo`` is both a proc call (head, line 1) and a bare argument word
# (line 2, ``set foo 1`` writes a variable named foo).
SRC = "proc foo {} {}\nfoo bar\nset foo 1\n"


def _col_of(line_text: str, needle: str) -> int:
    return line_text.index(needle)


def test_cursor_on_call_head_is_command_head():
    analysis = analyse(SRC)
    # Line 1: "foo bar" — cursor on the ``foo`` head.
    assert _position_is_command_head(analysis, 1, 0) is True


def test_cursor_on_argument_word_is_not_command_head():
    analysis = analyse(SRC)
    # Line 2: "set foo 1" — cursor on the ``foo`` argument (the var name),
    # not the ``set`` head.  This must NOT count as a command head, so the
    # cross-document fallback won't jump to the sibling proc ``foo``.
    col = _col_of("set foo 1", "foo")
    assert _position_is_command_head(analysis, 2, col) is False
    # The head of line 2 (``set``) IS a command head.
    assert _position_is_command_head(analysis, 2, 0) is True


def test_none_analysis_is_not_command_head():
    assert _position_is_command_head(None, 0, 0) is False
