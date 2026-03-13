"""Shared iRules context detection helpers."""

from __future__ import annotations

from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import TokenType


def find_enclosing_when_event(
    source: str,
    line: int,
    *,
    lines: list[str] | None = None,
) -> tuple[str | None, int]:
    """Find the enclosing ``when EVENT { ... }`` and return (event, anchor_line)."""
    if lines is None:
        lines = source.split("\n")
    if not lines:
        return (None, 0)

    idx = max(0, min(line, len(lines) - 1))

    def _scan_when_context(
        text: str,
        *,
        body_token=None,
    ) -> tuple[str, int] | None:
        best: tuple[str, int] | None = None

        for cmd in segment_commands(text, body_token):
            if not cmd.texts or cmd.texts[0] != "when" or len(cmd.texts) < 2:
                continue
            event_name = cmd.texts[1].upper()
            body_tok = next((tok for tok in cmd.arg_tokens if tok.type is TokenType.STR), None)
            if body_tok is None:
                continue
            if not (body_tok.start.line <= idx <= body_tok.end.line):
                continue

            best = (event_name, cmd.range.start.line)
            nested = _scan_when_context(body_tok.text, body_token=body_tok)
            if nested is not None:
                best = nested

        return best

    match = _scan_when_context(source)
    if match is not None:
        return match

    return (None, idx)
