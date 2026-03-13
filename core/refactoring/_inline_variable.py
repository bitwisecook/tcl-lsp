"""Inline variable — replace a single-use variable with its value."""

from __future__ import annotations

import re

from ..analysis.analyser import analyse
from ..common.position import offset_at_position
from . import RefactoringEdit, RefactoringResult
from ._spans import command_span_offsets, find_command_at, offsets_to_position, token_end_offset

_SET_RE = re.compile(r"^\s*set\s+(\S+)\s+(.+)$", re.DOTALL)
_PLAIN_VAR_RE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)\b")
_BRACED_VAR_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")


def inline_variable(
    source: str,
    line: int,
    character: int,
) -> RefactoringResult | None:
    """Inline the variable defined at the cursor position.

    Only inlines when:
    - The cursor is on a ``set var value`` command.
    - The variable has exactly one reference after the definition.

    Returns *None* when inlining is not safe.
    """
    result = analyse(source)

    # Find the ``set`` command at the cursor (recursing into nested bodies).
    cmd = find_command_at(source, line, character, predicate="set")
    if cmd is None:
        return None
    if len(cmd.texts) < 3:
        return None  # ``set var`` (read form) — nothing to inline

    var_name = cmd.texts[1]
    # Extract the raw source text for the value word.  Token start.offset
    # includes the opening delimiter (quote/brace) but end.offset points
    # to the last content character, excluding the closing delimiter.
    # Widen end to include the closing delimiter when present.
    value_tok = cmd.argv[2]
    val_start = value_tok.start.offset
    val_end = token_end_offset(source, value_tok)
    value_text = source[val_start:val_end]

    # Find the variable definition in the semantic model.
    var_def = None
    for scope in _walk_scopes(result.global_scope):
        if var_name in scope.variables:
            vd = scope.variables[var_name]
            if vd.definition_range.start.line == cmd.range.start.line:
                var_def = vd
                break
    if var_def is None:
        return None

    # Only inline if used exactly once.
    refs = var_def.references
    if len(refs) != 1:
        return None

    ref = refs[0]

    # Build edits: delete the ``set`` command and replace the reference.
    cmd_start, cmd_end = command_span_offsets(source, cmd)
    if cmd_end < len(source) and source[cmd_end] == "\n":
        cmd_end += 1
    set_start_line, set_start_char, set_end_line, set_end_char = offsets_to_position(
        source,
        cmd_start,
        cmd_end,
    )
    delete_edit = RefactoringEdit(
        start_line=set_start_line,
        start_character=set_start_char,
        end_line=set_end_line,
        end_character=set_end_char,
        new_text="",
    )

    # The reference range points to the variable name; we need to include
    # the ``$`` prefix (or ``${`` prefix).
    ref_name_start = offset_at_position(source, ref.start.line, ref.start.character)
    ref_name_end = offset_at_position(source, ref.end.line, ref.end.character + 1)
    ref_start = ref_name_start
    ref_end = ref_name_end
    if ref_name_start < len(source) and source[ref_name_start] == "$":
        if (
            ref_name_start + 1 < len(source)
            and source[ref_name_start + 1] == "{"
            and ref_name_end < len(source)
            and source[ref_name_end] == "}"
        ):
            ref_end = ref_name_end + 1
    elif (
        ref_name_start >= 2
        and source[ref_name_start - 2 : ref_name_start] == "${"
        and ref_name_end < len(source)
        and source[ref_name_end] == "}"
    ):
        ref_start = ref_name_start - 2
        ref_end = ref_name_end + 1
    elif ref_name_start >= 1 and source[ref_name_start - 1] == "$":
        ref_start = ref_name_start - 1
    ref_line, ref_char, ref_end_line, ref_end_char = offsets_to_position(source, ref_start, ref_end)

    replace_edit = RefactoringEdit(
        start_line=ref_line,
        start_character=ref_char,
        end_line=ref_end_line,
        end_character=ref_end_char,
        new_text=value_text,
    )

    return RefactoringResult(
        title=f"Inline variable '${var_name}'",
        edits=(delete_edit, replace_edit),
        kind="refactor.inline",
    )


def _walk_scopes(scope):
    """Yield *scope* and all descendant scopes."""
    yield scope
    for child in scope.children:
        yield from _walk_scopes(child)
