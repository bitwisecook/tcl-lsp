"""Convert unbraced expr arguments to braced form."""

from __future__ import annotations

from . import RefactoringEdit, RefactoringResult
from ._spans import find_command_at


def brace_expr(
    source: str,
    line: int,
    character: int,
) -> RefactoringResult | None:
    """Convert ``expr "..."`` or ``expr $a + $b`` to ``expr {$a + $b}``.

    Returns *None* when the expr is already braced or not applicable.
    """
    cmd = find_command_at(source, line, character, predicate="expr")
    if cmd is None:
        return None

    return _brace_expr_command(source, cmd)


def _brace_expr_command(source: str, cmd) -> RefactoringResult | None:
    """Brace an ``expr`` command's arguments."""
    if len(cmd.texts) < 2:
        return None

    first_arg = cmd.argv[1]
    last_arg = cmd.argv[-1]

    # Get the raw source span of all arguments after "expr".
    raw_start = first_arg.start.offset
    raw_end = last_arg.end.offset + 1
    # Widen to include closing delimiter (quote/brace) if present.
    if raw_end < len(source) and source[raw_start] in ('"', "{"):
        close = '"' if source[raw_start] == '"' else "}"
        if source[raw_end] == close:
            raw_end += 1
    raw = source[raw_start:raw_end]

    # Already braced.
    if raw.startswith("{") and raw.endswith("}"):
        return None

    # Extract content and wrap in braces.
    if raw.startswith('"') and raw.endswith('"'):
        braced = "{" + raw[1:-1] + "}"
    else:
        braced = "{" + raw + "}"

    # Build edit that replaces the full raw span including delimiters.
    # Convert the byte offsets back to line/character coordinates.
    lines = source.split("\n")
    start_line, start_char = _offset_to_pos(lines, raw_start)
    end_line, end_char = _offset_to_pos(lines, raw_end)

    edit = RefactoringEdit(
        start_line=start_line,
        start_character=start_char,
        end_line=end_line,
        end_character=end_char,
        new_text=braced,
    )

    return RefactoringResult(
        title="Brace expr for safety and performance",
        edits=(edit,),
        kind="refactor.rewrite",
    )


def _offset_to_pos(lines: list[str], offset: int) -> tuple[int, int]:
    """Convert a byte offset to (line, character)."""
    pos = 0
    for i, line in enumerate(lines):
        end = pos + len(line) + 1  # +1 for newline
        if offset <= end:
            return (i, offset - pos)
        pos = end
    return (len(lines) - 1, offset - pos)
