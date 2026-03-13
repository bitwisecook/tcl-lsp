"""Helpers for argv token span reconstruction."""

from __future__ import annotations

from .tokens import Token


def widen_argv_tokens_to_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each Tcl word's full token span."""
    if not argv or not all_tokens:
        return argv

    widened: list[Token] = []
    arg_i = 0
    word_start = argv[arg_i]
    word_end = word_start
    next_start = argv[arg_i + 1].start.offset if arg_i + 1 < len(argv) else None

    for tok in all_tokens:
        while next_start is not None and tok.start.offset >= next_start:
            widened.append(
                Token(
                    type=word_start.type,
                    text=word_start.text,
                    start=word_start.start,
                    end=word_end.end,
                )
            )
            arg_i += 1
            if arg_i >= len(argv):
                return widened
            word_start = argv[arg_i]
            word_end = word_start
            next_start = argv[arg_i + 1].start.offset if arg_i + 1 < len(argv) else None
        word_end = tok

    widened.append(
        Token(
            type=word_start.type,
            text=word_start.text,
            start=word_start.start,
            end=word_end.end,
        )
    )
    return widened
