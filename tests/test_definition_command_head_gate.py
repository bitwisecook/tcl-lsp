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
