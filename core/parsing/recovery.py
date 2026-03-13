"""Centralised error recovery via zero-width virtual tokens.

Detects imbalanced delimiters in a first parse and determines where to
inject virtual characters (``]``, ``}``, ``"``) so that a second parse
produces clean commands.  Both the analyser and semantic token provider
consume the clean parse — no surgery, no duplication, no position mapping.

Diagnostic messages use Tcl's exact error text so that the LSP reports
the same messages a user would see from ``tclsh``.

E201 — ``missing close-bracket`` (unterminated ``[``):
  - comment-break heuristic: ``#`` after an incomplete line signals ``]``
  - command-break heuristic: known command on the next line signals ``]``
  - brace-break heuristic: first ``{`` inside ``[`` signals ``]``

E202 — ``missing "`` (unterminated ``"``):
  - newline heuristic: ``"`` at end of line with a known command on the
    next non-blank line signals the quote should close immediately

E203 — ``missing close-brace`` (unterminated ``{``):
  - command heuristic: de-indented line starting with a known command
    signals ``}`` should be inserted before that line

E204 — ``extra characters after close-brace``
E205 — ``extra characters after close-quote``
E206 — ``missing close-brace for variable name``
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

from ..analysis.semantic_model import CodeFix, Diagnostic, Range, Severity
from ..common.ranges import position_from_relative
from ..parsing.command_segmenter import SegmentedCommand, segment_commands
from ..parsing.tokens import SourcePosition, Token, TokenType
from .known_commands import known_command_names

# Mapping from lexer warning messages to diagnostic codes.
_WARNING_CODE_MAP: dict[str, str] = {
    "extra characters after close-brace": "E204",
    "extra characters after close-quote": "E205",
    "missing close-brace for variable name": "E206",
    # These overlap with E201/E202/E203 but may be emitted as lexer
    # warnings for inner bodies where the recovery heuristics don't run.
    "missing close-bracket": "E201",
    'missing "': "E202",
    "missing close-brace": "E203",
}

log = logging.getLogger(__name__)


@dataclass(frozen=True, slots=True)
class VirtualToken:
    """A zero-width character the lexer should see at a source offset."""

    offset: int  # position in source (lexer-local, i.e. index into body text)
    char: str  # ']', '}', '{'
    diagnostic: Diagnostic  # the diagnostic to emit


def _base_offset_for(body_token: Token | None) -> int:
    """Return the base offset used by _make_lexer for *body_token*."""
    if body_token is None:
        return 0
    if body_token.type in (TokenType.STR, TokenType.CMD):
        return body_token.start.offset + 1
    return body_token.start.offset


def _cmd_text_position(
    tok: Token,
    text_idx: int,
) -> SourcePosition:
    """Compute absolute SourcePosition for ``tok.text[text_idx]``.

    ``tok.start`` points to the ``[`` delimiter.  The inner content
    starts at ``tok.start.offset + 1``.
    """
    return position_from_relative(
        tok.text,
        text_idx,
        base_line=tok.start.line,
        base_col=tok.start.character + 1,
        base_offset=tok.start.offset + 1,
    )


def _is_unterminated_cmd(tok: Token, source: str, base_offset: int) -> bool:
    """Return True when *tok* is a CMD token without a closing ``]``."""
    if tok.type is not TokenType.CMD:
        return False
    # The character immediately after the CMD text in the full source is
    # where ``]`` would be.  If it's not ``]``, the CMD is unterminated.
    close_local = tok.end.offset - base_offset + 1
    if 0 <= close_local < len(source) and source[close_local] == "]":
        return False
    return True


# E201 detectors


def _detect_missing_bracket_at_comment(
    tok: Token,
    source: str,
    base_offset: int,
) -> VirtualToken | None:
    """Detect ``]`` missing when a ``#`` comment follows the incomplete line.

    Scans the CMD token text line by line.  If a line starts with ``#``
    (ignoring leading whitespace), the previous line's content end is where
    ``]`` should be inserted.
    """
    text = tok.text
    lines = text.split("\n")
    if len(lines) < 2:
        return None

    # Look for a comment line (not the first line, which is part of the cmd).
    cumulative = 0
    for i, line in enumerate(lines):
        if i == 0:
            cumulative += len(line) + 1  # +1 for \n
            continue
        stripped = line.lstrip()
        if stripped.startswith("#"):
            # Found a comment.  Insert ] at the end of the previous line.
            # The end of the previous line's content (excluding trailing ws).
            prev_line = lines[i - 1]
            content_end = len(prev_line.rstrip())
            if i == 1:
                # The previous line is line 0 in cmd text
                insert_text_idx = content_end
            else:
                # Sum up lengths of lines 0..i-2 plus their \n separators,
                # then add content_end for line i-1.
                insert_text_idx = sum(len(lines[j]) + 1 for j in range(i - 1)) + content_end

            # Virtual ] at the corresponding local source offset.
            # CMD text starts at tok.start.offset + 1 in the full file,
            # which is local source index (tok.start.offset + 1 - base_offset).
            local_bracket_start = tok.start.offset - base_offset
            virtual_offset = local_bracket_start + 1 + insert_text_idx

            # Compute the newline offset for the virtual token.
            # The virtual ] goes at the position of the \n so that _parse_command
            # sees ] there and terminates.
            newline_text_idx = insert_text_idx
            virtual_offset = local_bracket_start + 1 + newline_text_idx

            # Diagnostic: highlight from [ to end of content on the incomplete line.
            diag_end = _cmd_text_position(tok, max(insert_text_idx - 1, 0))
            diag_range = Range(start=tok.start, end=diag_end)

            # CodeFix: insert ] at the content end.
            insert_pos = _cmd_text_position(tok, insert_text_idx)
            # Zero-width insertion: start == end.
            fix_range = Range(start=insert_pos, end=insert_pos)

            return VirtualToken(
                offset=virtual_offset,
                char="]",
                diagnostic=Diagnostic(
                    range=diag_range,
                    severity=Severity.ERROR,
                    code="E201",
                    message="missing close-bracket",
                    fixes=(
                        CodeFix(
                            range=fix_range,
                            new_text="]",
                            description="Insert missing ']' before comment",
                        ),
                    ),
                ),
            )
        # Non-empty, non-comment line — keep looking only if it's blank.
        if stripped:
            break
        cumulative += len(line) + 1

    return None


def _detect_missing_bracket_at_command(
    tok: Token,
    source: str,
    base_offset: int,
    known_commands: frozenset[str],
) -> VirtualToken | None:
    """Detect ``]`` missing when the next line starts with a known command.

    Scans the CMD token text line by line.  If a non-blank line (after
    line 0) starts with a known command, the previous line's content end
    is where ``]`` should be inserted.
    """
    text = tok.text
    lines = text.split("\n")
    if len(lines) < 2:
        return None

    cumulative = 0
    for i, line in enumerate(lines):
        if i == 0:
            cumulative += len(line) + 1  # +1 for \n
            continue
        stripped = line.lstrip()
        if not stripped:
            cumulative += len(line) + 1
            continue  # skip blank lines
        first_word = _extract_first_word(stripped)
        if first_word in known_commands:
            # Found a known command.  Insert ] at end of previous line.
            prev_line = lines[i - 1]
            content_end = len(prev_line.rstrip())
            if i == 1:
                insert_text_idx = content_end
            else:
                insert_text_idx = sum(len(lines[j]) + 1 for j in range(i - 1)) + content_end

            local_bracket_start = tok.start.offset - base_offset
            newline_text_idx = insert_text_idx
            virtual_offset = local_bracket_start + 1 + newline_text_idx

            # Diagnostic: highlight from [ to end of content on the incomplete line.
            diag_end = _cmd_text_position(tok, max(insert_text_idx - 1, 0))
            diag_range = Range(start=tok.start, end=diag_end)

            # CodeFix: insert ] at the content end.
            insert_pos = _cmd_text_position(tok, insert_text_idx)
            fix_range = Range(start=insert_pos, end=insert_pos)

            return VirtualToken(
                offset=virtual_offset,
                char="]",
                diagnostic=Diagnostic(
                    range=diag_range,
                    severity=Severity.ERROR,
                    code="E201",
                    message="missing close-bracket",
                    fixes=(
                        CodeFix(
                            range=fix_range,
                            new_text="]",
                            description="Insert missing ']' before command",
                        ),
                    ),
                ),
            )
        # Non-empty, non-command line — stop looking.
        break

    return None


def _detect_missing_bracket_at_brace(
    tok: Token,
    source: str,
    base_offset: int,
) -> VirtualToken | None:
    """Detect ``]`` missing when a ``{`` inside ``[`` swallowed the rest.

    This is the existing E201 heuristic: find the first ``{`` in the CMD
    text, step back past whitespace, and insert ``]`` there.
    """
    text = tok.text
    brace_idx = text.find("{")
    if brace_idx < 0:
        return None

    # Step back past whitespace before the brace.
    insert_idx = brace_idx
    while insert_idx > 0 and text[insert_idx - 1] in " \t":
        insert_idx -= 1

    # Check that there's actual content before the brace.
    content = text[:insert_idx].rstrip()
    if not content:
        return None

    # Local source offset for the virtual ].
    local_bracket_start = tok.start.offset - base_offset
    virtual_offset = local_bracket_start + 1 + insert_idx

    # Diagnostic range: from [ to end of content before the brace.
    content_end_idx = max(len(content) - 1, 0)
    diag_end = _cmd_text_position(tok, content_end_idx)
    diag_range = Range(start=tok.start, end=diag_end)

    # CodeFix: insert ] at the insert position.
    insert_pos = _cmd_text_position(tok, insert_idx)
    fix_range = Range(start=insert_pos, end=insert_pos)

    return VirtualToken(
        offset=virtual_offset,
        char="]",
        diagnostic=Diagnostic(
            range=diag_range,
            severity=Severity.ERROR,
            code="E201",
            message="missing close-bracket",
            fixes=(
                CodeFix(
                    range=fix_range,
                    new_text="]",
                    description="Insert missing ']' before '{'",
                ),
            ),
        ),
    )


def _detect_missing_bracket_no_heuristic(
    tok: Token,
) -> Diagnostic:
    """Fallback E201 when no heuristic can determine where ``]`` belongs.

    Emits a diagnostic highlighting just the opening ``[``.
    """
    return Diagnostic(
        range=Range(start=tok.start, end=tok.start),
        severity=Severity.ERROR,
        code="E201",
        message="missing close-bracket",
    )


def _extract_first_word(stripped: str) -> str:
    """Extract the first word from a stripped line."""
    end = 0
    while end < len(stripped) and stripped[end] not in " \t\n\r;{[":
        end += 1
    return stripped[:end]


# E202 detectors (unterminated ")


def _is_suspicious_quote(
    tok: Token,
    cmd: SegmentedCommand,
    source: str,
    base_offset: int,
) -> bool:
    """Return True when *tok* is an ESC from an unterminated ``"`` at EOL.

    The signal is:
      1. ESC token whose start position in source is ``"``
      2. Token text starts with ``\\n`` (nothing on the line after ``"``)
      3. Content after the newline is non-empty
      4. The command reaches EOF (unterminated quote swallows everything)
    """
    if tok.type is not TokenType.ESC:
        return False
    src_idx = tok.start.offset - base_offset
    if src_idx < 0 or src_idx >= len(source) or source[src_idx] != '"':
        return False
    if not tok.text.startswith("\n"):
        return False
    if not tok.text[1:].strip():
        return False
    # The command must reach EOF — a properly closed quote wouldn't.
    if cmd.all_tokens:
        last = cmd.all_tokens[-1]
        if last.end.offset < base_offset + len(source) - 1:
            return False
    return True


def _detect_missing_quote_at_newline(
    tok: Token,
    source: str,
    base_offset: int,
    known_commands: frozenset[str],
) -> VirtualToken | None:
    r"""Detect ``"`` missing when the next line starts with a known command.

    When ``set x "`` appears at end-of-line, the quote absorbs everything.
    If the first non-blank line after the ``"`` starts with a known command,
    insert a virtual ``"`` right after the opening one (producing ``""``).
    """
    text = tok.text
    lines = text.split("\n")
    if len(lines) < 2:
        return None

    for i, line in enumerate(lines):
        if i == 0:
            continue
        stripped = line.lstrip()
        if not stripped:
            continue  # skip blank lines
        first_word = _extract_first_word(stripped)
        if first_word in known_commands:
            # Virtual " right after the opening " → creates empty string "".
            local_quote_start = tok.start.offset - base_offset
            virtual_offset = local_quote_start + 1

            # Diagnostic: highlight just the opening ".
            diag_range = Range(start=tok.start, end=tok.start)

            # CodeFix: insert " right after the opening ".
            insert_pos = SourcePosition(
                line=tok.start.line,
                character=tok.start.character + 1,
                offset=tok.start.offset + 1,
            )
            fix_range = Range(start=insert_pos, end=insert_pos)

            return VirtualToken(
                offset=virtual_offset,
                char='"',
                diagnostic=Diagnostic(
                    range=diag_range,
                    severity=Severity.ERROR,
                    code="E202",
                    message='missing "',
                    fixes=(
                        CodeFix(
                            range=fix_range,
                            new_text='"',
                            description="Insert missing '\"' to close string",
                        ),
                    ),
                ),
            )
        # First non-blank line is not a known command — stop looking.
        break

    return None


def _detect_missing_quote_no_heuristic(
    tok: Token,
    source: str,
    base_offset: int,
) -> Diagnostic:
    """Fallback E202 when no heuristic can determine where ``"`` belongs."""
    return Diagnostic(
        range=Range(start=tok.start, end=tok.start),
        severity=Severity.ERROR,
        code="E202",
        message='missing "',
    )


# E203 detectors (unterminated {)


def _is_suspicious_str(
    tok: Token,
    source: str,
    base_offset: int,
) -> bool:
    """Return True when *tok* is a STR from an unterminated ``{``.

    The signal is:
      1. STR token whose start position in source is ``{``
      2. No closing ``}`` after the token end in source
      3. Token spans at least 2 lines
      4. Token text does not contain ``}`` (otherwise it's E103 territory:
         the brace was closed at the wrong level, not truly missing)
    """
    if tok.type is not TokenType.STR:
        return False
    src_idx = tok.start.offset - base_offset
    if src_idx < 0 or src_idx >= len(source) or source[src_idx] != "{":
        return False
    # Check that the closing } is missing.
    close_local = tok.end.offset - base_offset + 1
    if 0 <= close_local < len(source) and source[close_local] == "}":
        return False  # properly closed
    # If the text contains }, the brace was closed at the wrong nesting
    # level — that's E103 (stolen close brace), not E203.
    if "}" in tok.text:
        return False
    # Must span multiple lines.
    line_span = tok.end.line - tok.start.line
    if line_span < 2:
        return False
    return True


def _detect_missing_brace_at_command(
    tok: Token,
    source: str,
    base_offset: int,
    known_commands: frozenset[str],
) -> VirtualToken | None:
    """Detect ``}`` missing when a de-indented known command follows brace data.

    Scans the STR token text line by line.  When a line is de-indented
    relative to the first content line AND starts with a known command,
    ``}`` should be inserted at the ``\\n`` before that line.
    """
    text = tok.text
    lines = text.split("\n")
    if len(lines) < 3:
        return None

    # Determine indentation of the first content line.
    first_indent: int | None = None
    for line in lines[1:]:  # skip first line (may be empty right after {)
        stripped = line.lstrip()
        if stripped:
            first_indent = len(line) - len(stripped)
            break

    if first_indent is None:
        return None

    # Scan for a de-indented line starting with a known command.
    cumulative = 0
    for i, line in enumerate(lines):
        if i == 0:
            cumulative += len(line) + 1
            continue
        stripped = line.lstrip()
        if not stripped:
            cumulative += len(line) + 1
            continue
        indent = len(line) - len(stripped)
        if indent < first_indent:
            first_word = _extract_first_word(stripped)
            if first_word in known_commands:
                # Verify that brace content before this point is balanced
                # (no unmatched nested braces).
                content_before = text[:cumulative]
                brace_depth = 0
                for ch in content_before:
                    if ch == "{":
                        brace_depth += 1
                    elif ch == "}":
                        brace_depth -= 1
                if brace_depth != 0:
                    # Nested unmatched braces — can't recover with a single }.
                    cumulative += len(line) + 1
                    continue

                # Virtual } at the \n before this line.
                newline_text_idx = cumulative - 1
                local_brace_start = tok.start.offset - base_offset
                virtual_offset = local_brace_start + 1 + newline_text_idx

                # Diagnostic: highlight the opening {.
                diag_range = Range(start=tok.start, end=tok.start)

                # CodeFix: insert } at end of previous line content.
                # Use the \n position (same as virtual offset location).
                insert_pos = _cmd_text_position(tok, newline_text_idx)
                fix_range = Range(start=insert_pos, end=insert_pos)

                return VirtualToken(
                    offset=virtual_offset,
                    char="}",
                    diagnostic=Diagnostic(
                        range=diag_range,
                        severity=Severity.ERROR,
                        code="E203",
                        message="missing close-brace",
                        fixes=(
                            CodeFix(
                                range=fix_range,
                                new_text="}",
                                description="Insert missing '}' before command",
                            ),
                        ),
                    ),
                )
        cumulative += len(line) + 1

    return None


def _detect_missing_brace_no_heuristic(
    tok: Token,
    source: str,
    base_offset: int,
) -> Diagnostic:
    """Fallback E203 when no heuristic can determine where ``}`` belongs."""
    return Diagnostic(
        range=Range(start=tok.start, end=tok.start),
        severity=Severity.ERROR,
        code="E203",
        message="missing close-brace",
    )


# Virtual token detection


def _detect_all_virtual_tokens(
    commands: list[SegmentedCommand],
    source: str,
    base_offset: int,
) -> tuple[list[VirtualToken], list[Diagnostic]]:
    """Scan commands for unterminated delimiters and return virtual tokens + fallback diagnostics."""
    virtuals: list[VirtualToken] = []
    fallback_diags: list[Diagnostic] = []
    known_cmds: frozenset[str] | None = None  # lazy

    for cmd in commands:
        for tok in cmd.all_tokens:
            # E201: unterminated [
            if _is_unterminated_cmd(tok, source, base_offset):
                # Priority: comment-break > command-break > brace-break.
                vt = _detect_missing_bracket_at_comment(tok, source, base_offset)
                if vt is None:
                    if known_cmds is None:
                        try:
                            known_cmds = known_command_names()
                        except Exception:
                            log.debug("recovery: failed to load known commands", exc_info=True)
                            known_cmds = frozenset()
                    vt = _detect_missing_bracket_at_command(
                        tok,
                        source,
                        base_offset,
                        known_cmds,
                    )
                if vt is None:
                    vt = _detect_missing_bracket_at_brace(tok, source, base_offset)
                if vt is not None:
                    virtuals.append(vt)
                else:
                    fallback_diags.append(
                        _detect_missing_bracket_no_heuristic(tok),
                    )

            # E202: unterminated "
            elif _is_suspicious_quote(tok, cmd, source, base_offset):
                if known_cmds is None:
                    try:
                        known_cmds = known_command_names()
                    except Exception:
                        log.debug("recovery: failed to load known commands", exc_info=True)
                        continue
                vt = _detect_missing_quote_at_newline(
                    tok,
                    source,
                    base_offset,
                    known_cmds,
                )
                if vt is not None:
                    virtuals.append(vt)
                else:
                    fallback_diags.append(
                        _detect_missing_quote_no_heuristic(tok, source, base_offset),
                    )

            # E203: unterminated {
            elif _is_suspicious_str(tok, source, base_offset):
                if known_cmds is None:
                    try:
                        known_cmds = known_command_names()
                    except Exception:
                        log.debug("recovery: failed to load known commands", exc_info=True)
                        continue
                vt = _detect_missing_brace_at_command(
                    tok,
                    source,
                    base_offset,
                    known_cmds,
                )
                if vt is not None:
                    virtuals.append(vt)
                else:
                    fallback_diags.append(
                        _detect_missing_brace_no_heuristic(tok, source, base_offset),
                    )

    return virtuals, fallback_diags


def compute_virtual_insertions(
    source: str,
    body_token: Token | None = None,
) -> dict[int, str]:
    """Compute virtual token insertions for error recovery.

    Does a first parse, detects imbalances, and returns the insertions
    dict that can be passed to :class:`TclLexer`.  Returns an empty dict
    when no recovery is needed.
    """
    commands = segment_commands(source, body_token)
    if not commands:
        return {}

    base_offset = _base_offset_for(body_token)
    virtuals, _ = _detect_all_virtual_tokens(commands, source, base_offset)

    if not virtuals:
        return {}

    return {vt.offset: vt.char for vt in virtuals}


# Lexer warning messages
# (E201/E202/E203).  These are suppressed to avoid duplicate diagnostics
# when the recovery module already detected the same issue.
_RECOVERY_HANDLED_MESSAGES: frozenset[str] = frozenset(
    {
        "missing close-bracket",
        'missing "',
        "missing close-brace",
    }
)


def _lexer_warnings_to_diagnostics(
    warnings: list[tuple[SourcePosition, str]],
) -> list[Diagnostic]:
    """Convert raw lexer warnings into LSP diagnostics.

    Each warning is ``(position, message)`` where *message* matches the
    Tcl error text exactly.  The diagnostic code is looked up from
    :data:`_WARNING_CODE_MAP`.

    Warnings that overlap with the recovery heuristics (E201–E203) are
    suppressed because the recovery module already emits those diagnostics
    with better position information and quick-fixes.
    """
    diagnostics: list[Diagnostic] = []
    for pos, message in warnings:
        if message in _RECOVERY_HANDLED_MESSAGES:
            continue  # already covered by E201/E202/E203 heuristics
        code = _WARNING_CODE_MAP.get(message, "E200")
        diagnostics.append(
            Diagnostic(
                range=Range(start=pos, end=pos),
                severity=Severity.ERROR,
                code=code,
                message=message,
            )
        )
    return diagnostics


def segment_with_recovery(
    source: str,
    body_token: Token | None = None,
) -> tuple[list[SegmentedCommand], list[Diagnostic]]:
    """Parse source, detect imbalances, re-parse with virtual tokens.

    1. ``segment_commands(source, body_token)`` — first parse
    2. Scan for unterminated CMD tokens — find virtual tokens
    3. ``segment_commands(source, body_token, virtual_insertions=...)`` — clean re-parse
    4. Return ``(clean_commands, diagnostics)``

    When no imbalances are detected, the first parse result is returned
    directly (zero overhead for clean files).

    Lexer warnings (e.g. "extra characters after close-brace") are also
    harvested and converted to diagnostics.
    """
    lexer_warnings: list[tuple[SourcePosition, str]] = []
    commands = segment_commands(
        source,
        body_token,
        collect_warnings=lexer_warnings,
    )

    if not commands:
        return commands, _lexer_warnings_to_diagnostics(lexer_warnings)

    base_offset = _base_offset_for(body_token)
    virtuals, fallback_diags = _detect_all_virtual_tokens(
        commands,
        source,
        base_offset,
    )

    warning_diags = _lexer_warnings_to_diagnostics(lexer_warnings)

    if not virtuals:
        return commands, fallback_diags + warning_diags

    # Re-parse with virtual tokens injected.
    virtual_insertions = {vt.offset: vt.char for vt in virtuals}
    reparse_warnings: list[tuple[SourcePosition, str]] = []
    clean_commands = segment_commands(
        source,
        body_token,
        virtual_insertions=virtual_insertions,
        collect_warnings=reparse_warnings,
    )
    diagnostics = (
        [vt.diagnostic for vt in virtuals]
        + fallback_diags
        + warning_diags
        + _lexer_warnings_to_diagnostics(reparse_warnings)
    )

    return clean_commands, diagnostics
