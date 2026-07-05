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

"""Extract variable — replace a selected expression with a named variable."""

from __future__ import annotations

import re

from shared.position import offset_at_position

from . import RefactoringEdit, RefactoringResult

_WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")

# A whitespace-delimited binary operator marks an arithmetic/logical
# expression that must be wrapped in ``[expr { ... }]`` so the resulting
# ``set`` command stays a valid two-argument call.
_EXPR_OP_RE = re.compile(
    r"\s(?:\*\*|\+|-|\*|/|%|==|!=|<=|>=|<|>|&&|\|\||eq|ne|in|ni)\s",
)


def _looks_like_expr(text: str) -> bool:
    return bool(_EXPR_OP_RE.search(text))


def extract_variable(
    source: str,
    start_line: int,
    start_character: int,
    end_line: int,
    end_character: int,
    var_name: str = "result",
) -> RefactoringResult | None:
    """Extract the selected expression into a ``set`` assignment.

    Returns *None* when the selection is empty or only whitespace.
    """
    start = offset_at_position(source, start_line, start_character)
    end = offset_at_position(source, end_line, end_character)
    if end <= start:
        return None

    selected = source[start:end]
    if not selected.strip():
        return None

    # Work out indentation of the line containing the selection.
    lines = source.split("\n")
    if start_line >= len(lines):
        return None
    line_text = lines[start_line]
    indent = line_text[: len(line_text) - len(line_text.lstrip())]

    # Build the ``set var <value>`` insertion and the replacement reference.
    # A bare operator expression (``$a * $b``) is not a valid value word for
    # ``set`` — wrap it in ``[expr { ... }]``.  A selection that is already a
    # command substitution (``[cmd ...]``) or a single word is kept verbatim.
    stripped = selected.strip()
    if not stripped.startswith("[") and _looks_like_expr(stripped):
        value = f"[expr {{{stripped}}}]"
    else:
        value = selected
    assignment = f"{indent}set {var_name} {value}\n"
    replacement = f"${var_name}"

    set_edit = RefactoringEdit(
        start_line=start_line,
        start_character=0,
        end_line=start_line,
        end_character=0,
        new_text=assignment,
    )
    # Use original selection coordinates — apply() processes bottom-to-top
    # so the replacement runs before the insertion.
    replace_edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_character,
        end_line=end_line,
        end_character=end_character,
        new_text=replacement,
    )
    return RefactoringResult(
        title=f"Extract into variable '${var_name}'",
        edits=(set_edit, replace_edit),
        kind="refactor.extract",
    )
