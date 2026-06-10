"""Shared iRules context detection helpers."""

from __future__ import annotations

from compiler.parsing.command_segmenter import segment_commands
from shared.tokens import TokenType


def find_enclosing_when_event(
    source: str,
    line: int,
    *,
    lines: list[str] | None = None,
    embedded_rules: list | None = None,
) -> tuple[str | None, int]:
    """Find the enclosing ``when EVENT { ... }`` and return (event, anchor_line).

    When *embedded_rules* is provided (conf-wrapped mode), the search
    is scoped to the rule body containing *line*.
    """
    if lines is None:
        lines = source.split("\n")
    if not lines:
        return (None, 0)

    idx = max(0, min(line, len(lines) - 1))

    # Conf-wrapped mode: scope to the rule body containing the cursor.
    scan_source = source
    scan_idx = idx
    if embedded_rules:
        from shared.document_buffer import DocumentBuffer

        buf = DocumentBuffer.from_source(source)
        for rule in embedded_rules:
            body_start = buf.offset_to_position(rule.body_start_offset)
            body_end = buf.offset_to_position(max(rule.body_end_offset - 1, rule.body_start_offset))
            if body_start.line <= idx <= body_end.line:
                # Cursor is inside this rule body.
                scan_source = rule.body
                scan_idx = idx - body_start.line
                break
        else:
            # Cursor is outside any rule body.
            return (None, idx)

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
            if not (body_tok.start.line <= scan_idx <= body_tok.end.line):
                continue

            best = (event_name, cmd.range.start.line)
            nested = _scan_when_context(body_tok.text, body_token=body_tok)
            if nested is not None:
                best = nested

        return best

    match = _scan_when_context(scan_source)
    if match is not None:
        if embedded_rules and scan_source != source:
            # Shift the anchor line back to file coordinates.
            from shared.document_buffer import DocumentBuffer

            buf = DocumentBuffer.from_source(source)
            for rule in embedded_rules:
                body_start = buf.offset_to_position(rule.body_start_offset)
                body_end = buf.offset_to_position(
                    max(rule.body_end_offset - 1, rule.body_start_offset)
                )
                if body_start.line <= idx <= body_end.line:
                    return (match[0], match[1] + body_start.line)
        return match

    return (None, idx)
