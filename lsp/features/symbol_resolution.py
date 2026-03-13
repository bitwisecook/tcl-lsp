"""Shared symbol and scope resolution helpers for editor features."""

from __future__ import annotations

from core.analysis.semantic_model import Range, Scope
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType

_WORD_DELIMS = ' \t\n;{}[]"$'


def find_word_span_at_position(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> tuple[str, int, int] | None:
    """Extract the word and its [start, end) columns at the given position."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return None
    line_text = lines[line]
    if character >= len(line_text):
        return None

    start = character
    while start > 0 and line_text[start - 1] not in _WORD_DELIMS:
        start -= 1

    end = character
    while end < len(line_text) and line_text[end] not in _WORD_DELIMS:
        end += 1

    word = line_text[start:end]
    if not word:
        return None
    return (word, start, end)


def find_word_at_position(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> str | None:
    """Extract the word at the given position."""
    span = find_word_span_at_position(source, line, character, lines=lines)
    if span is None:
        return None
    return span[0]


def _command_tokens_for_prefix(prefix: str) -> tuple[list[Token], bool]:
    """Return tokens in the active command segment and whether cursor is at a new word."""
    lexer = TclLexer(prefix)
    tokens: list[Token] = []
    at_new_word = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break

        if tok.type is TokenType.SEP:
            at_new_word = True
            continue

        if tok.type is TokenType.EOL:
            # Real semicolon/newline resets command context; the synthetic EOF EOL
            # (empty text) should not change context.
            if tok.text:
                tokens = []
                at_new_word = True
            continue

        if tok.type is TokenType.COMMENT:
            continue

        tokens.append(tok)
        at_new_word = False

    return (tokens, at_new_word)


def find_command_context_in_line(
    line_text: str,
    character: int,
) -> tuple[str | None, str, int]:
    """Return (command, current_word, word_index) in the active command segment."""
    command, _args, current_word, word_index = find_command_context_details_in_line(
        line_text, character
    )
    return (command, current_word, word_index)


def find_command_context_details_in_line(
    line_text: str,
    character: int,
) -> tuple[str | None, list[str], str, int]:
    """Return (command, args, current_word, word_index) for active command segment."""
    col = min(max(character, 0), len(line_text))
    prefix = line_text[:col]
    tokens, at_new_word = _command_tokens_for_prefix(prefix)

    if not tokens:
        return (None, [], "", -1)

    if at_new_word:
        current_word = ""
        word_index = len(tokens)
    else:
        current_word = tokens[-1].text
        word_index = len(tokens) - 1

    command = tokens[0].text
    args = [tok.text for tok in tokens[1:]]
    return (command, args, current_word, word_index)


def find_command_context_at_position(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> tuple[str | None, str, int]:
    """Return command context for a source position."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return (None, "", -1)
    return find_command_context_in_line(lines[line], character)


def find_command_context_details_at_position(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> tuple[str | None, list[str], str, int]:
    """Return detailed command context for a source position."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return (None, [], "", -1)
    return find_command_context_details_in_line(lines[line], character)


def find_variable_completion_prefix(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> str | None:
    """Return the variable-name prefix after `$` at the cursor, if present."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return None
    line_text = lines[line]
    col = min(max(character, 0), len(line_text))
    prefix = line_text[:col]

    tokens, at_new_word = _command_tokens_for_prefix(prefix)
    if at_new_word or not tokens:
        return None

    tail = tokens[-1]
    if tail.type is TokenType.VAR:
        return tail.text
    if tail.text == "$":
        return ""
    return None


def find_var_at_position(
    source: str,
    line: int,
    character: int,
    *,
    lines: list[str] | None = None,
) -> str | None:
    """Check if position is on a $variable reference and return its name."""
    if lines is None:
        lines = source.split("\n")
    if line >= len(lines):
        return None
    line_text = lines[line]

    pos = min(character, len(line_text))
    while pos > 0 and line_text[pos - 1] not in ' \t\n;{}[]"':
        pos -= 1
    if pos > 0 and line_text[pos - 1] == "$":
        pos -= 1

    if pos < len(line_text) and line_text[pos] == "$":
        start = pos + 1
        end = start
        while end < len(line_text) and (line_text[end].isalnum() or line_text[end] in "_:"):
            end += 1
        var_name = line_text[start:end]
        if var_name:
            return var_name

    return None


def _range_contains_line(r: Range, line: int) -> bool:
    return r.start.line <= line <= r.end.line


def find_scope_at_line(scope: Scope, line: int) -> Scope:
    """Find the innermost lexical scope that contains *line*."""
    for child in scope.children:
        if child.body_range is not None and _range_contains_line(child.body_range, line):
            return find_scope_at_line(child, line)
        if child.kind == "proc":
            proc_def = scope.procs.get(child.name)
            if proc_def and _range_contains_line(proc_def.body_range, line):
                return find_scope_at_line(child, line)
    return scope
