from __future__ import annotations

import logging

from compiler.parsing.expr_lexer import ExprTokenType
from compiler.parsing.token_positions import token_content_base
from compiler.parsing.tokens import SourcePosition, Token, TokenType
from compiler.registry.runtime import (
    ArgRole,
    arg_indices_for_role,
)
from shared.ranges import position_from_offset

from ._constants import (
    _ESCAPE_RE,
    _LANGUAGE_KEYWORDS,
    _OPERATORS,
    _TYPE_INDEX,
)

log = logging.getLogger(__name__)


def _classify_token(tok_type: TokenType, text: str, *, is_command_name: bool) -> int | None:
    """Return semantic token type index, or None to skip this token."""
    match tok_type:
        case TokenType.VAR:
            return _TYPE_INDEX["variable"]
        case TokenType.CMD:
            return None  # command contents handled by recursive tokenisation
        case TokenType.STR:
            return _TYPE_INDEX["string"]
        case TokenType.COMMENT:
            return _TYPE_INDEX["comment"]
        case TokenType.ESC:
            if is_command_name:
                if text in _LANGUAGE_KEYWORDS:
                    return _TYPE_INDEX["keyword"]
                if text in _OPERATORS:
                    return _TYPE_INDEX["operator"]
                # Command aliases (interp alias) also land here as
                # "function" — this is correct since they are user-defined,
                # matching proc styling.
                return _TYPE_INDEX["function"]
            # Check if it's a number
            try:
                int(text)
                return _TYPE_INDEX["number"]
            except ValueError:
                pass
            try:
                float(text)
                return _TYPE_INDEX["number"]
            except ValueError:
                pass
            return _TYPE_INDEX["string"]
        case _:
            return None


def _arg_indices(cmd_name: str, argv_texts: list[str], role: ArgRole) -> set[int]:
    """Return argument indices (0-based, after command name) for a given role."""
    return arg_indices_for_role(cmd_name, argv_texts, role)


def _classify_expr_token(tok_type: ExprTokenType, text: str) -> int | None:
    """Return semantic token type index for expression tokens."""
    match tok_type:
        case ExprTokenType.VARIABLE:
            return _TYPE_INDEX["variable"]
        case ExprTokenType.NUMBER:
            return _TYPE_INDEX["number"]
        case ExprTokenType.OPERATOR | ExprTokenType.PAREN_OPEN | ExprTokenType.PAREN_CLOSE:
            return _TYPE_INDEX["operator"]
        case ExprTokenType.TERNARY_Q | ExprTokenType.TERNARY_C | ExprTokenType.COMMA:
            return _TYPE_INDEX["operator"]
        case ExprTokenType.FUNCTION:
            return _TYPE_INDEX["function"]
        case ExprTokenType.BOOL:
            return _TYPE_INDEX["keyword"]
        case ExprTokenType.STRING:
            return _TYPE_INDEX["string"]
        case _:
            return None


def _append_token(
    out: list[tuple[int, int, int, int, int]],
    *,
    line: int,
    char: int,
    length: int,
    type_idx: int,
    modifiers: int = 0,
) -> None:
    """Append a token if it has a positive length."""
    if length <= 0:
        return
    out.append((line, char, length, type_idx, modifiers))


def _append_text_token(
    out: list[tuple[int, int, int, int, int]],
    *,
    start: SourcePosition,
    text: str,
    type_idx: int,
    modifiers: int = 0,
) -> None:
    """Append semantic token segments, splitting safely across lines."""
    if not text:
        return

    line = start.line
    char = start.character
    parts = text.split("\n")
    for i, part in enumerate(parts):
        _append_token(
            out,
            line=line,
            char=char,
            length=len(part),
            type_idx=type_idx,
            modifiers=modifiers,
        )
        if i < len(parts) - 1:
            line += 1
            char = 0


def _emit_namespace_qualified(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
    type_idx: int,
    modifiers: int = 0,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> None:
    """Split ``NS::cmd`` into a namespace token and a command token."""
    text = tok.text
    idx = text.rfind("::")
    ns_part = text[: idx + 2]
    cmd_part = text[idx + 2 :]

    base_offset, _base_line, _base_col = token_content_base(tok)
    ns_start = position_from_offset(
        base_offset + 0,
        line_starts,
        source_len,
    )
    _append_text_token(
        out,
        start=ns_start,
        text=ns_part,
        type_idx=_TYPE_INDEX["namespace"],
        modifiers=modifiers,
    )
    if cmd_part:
        cmd_start = position_from_offset(
            base_offset + idx + 2,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=cmd_start,
            text=cmd_part,
            type_idx=type_idx,
            modifiers=modifiers,
        )


def _emit_string_with_escapes(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
    *,
    line_starts: list[int] | tuple[int, ...] = (),
    source_len: int = 0,
) -> bool:
    """Sub-tokenize an ESC token to highlight backslash escape sequences.

    Returns True when at least one escape was emitted, False otherwise.
    """
    text = tok.text
    matches = list(_ESCAPE_RE.finditer(text))
    if not matches:
        return False

    base_offset, _base_line, _base_col = token_content_base(tok)
    pos = 0

    for match in matches:
        if match.start() > pos:
            before_start = position_from_offset(
                base_offset + pos,
                line_starts,
                source_len,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )
        esc_start = position_from_offset(
            base_offset + match.start(),
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=esc_start,
            text=match.group(),
            type_idx=_TYPE_INDEX["escape"],
        )
        pos = match.end()

    if pos < len(text):
        rest_start = position_from_offset(
            base_offset + pos,
            line_starts,
            source_len,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos:],
            type_idx=_TYPE_INDEX["string"],
        )
    return True
