"""Extract variable — replace a selected expression with a named variable."""

from __future__ import annotations

import re

from ..common.position import offset_at_position
from . import RefactoringEdit, RefactoringResult

_WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


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

    # Build the ``set var [expr]`` insertion and the replacement reference.
    # If the selection looks like a command substitution [cmd ...], keep it.
    # Otherwise wrap in [expr { ... }] only if it contains operators.
    assignment = f"{indent}set {var_name} {selected}\n"
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
