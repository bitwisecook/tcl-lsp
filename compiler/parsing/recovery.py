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

from shared.codes import diag
from shared.diagnostic import CodeFix, Diagnostic, Range, Severity
from shared.ranges import position_from_relative
from shared.tokens import SourcePosition, Token, TokenType

from .command_segmenter import SegmentedCommand, segment_commands
from .known_commands import known_command_names

# Module-level registrations for codes emitted from multiple detector functions.
diag("E201", "Parser recovery — unclosed brace.", section="error", internal=True)
diag("E202", "Parser recovery — unclosed bracket.", section="error", internal=True)
diag("E203", "Parser recovery — unclosed quote.", section="error", internal=True)

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
    # Empty command substitution ``[]``: end.offset points AT the ``]``
    # itself (there are no content characters) so the +1 overshoots.
    if not tok.text:
        close_local -= 1
        if 0 <= close_local < len(source) and source[close_local] == "]":
            return False
    return True


# E201 detectors


def _bracket_insert_inert(text: str, idx: int) -> bool:
    """True when offset *idx* in a CMD's content is inside an open brace/quote word.

    A ``]`` inserted at such a position is *literal* — the content of an
    unclosed brace or quoted word — so it cannot terminate the ``[`` and the
    recovered command stays incomplete.  The comment- and command-break
    heuristics therefore *veto* a candidate landing here — the "syntactic
    validates; veto if the offset is inert" rule of the recovery design
    (``docs/design/compiler/error-recovery-rust-port.md``).  Grounded in
    C Tcl 9.0.3: for ``set x [foo {bar`` / ``puts baz}`` the ``puts`` line is
    inside the balanced brace word ``{bar … baz}``, so ``info complete {set x
    [foo {bar]…}`` is ``0`` (incomplete) while the end-insert is ``1``.

    Mirrors the one Tcl rule that decides this: ``"`` and ``{`` only *open* a
    quoted or braced word when they are the **first character of a word** (rules
    8 and 9 of the dodekalogue).  A ``"`` or ``{`` mid-word — ``foo abc"``,
    ``a{b``, ``${var}`` — is an ordinary literal and must not count, otherwise a
    genuine script-level command-break (``set x [foo abc"`` / ``puts done``,
    which C Tcl 9.0.3 *completes* via ``set x [foo abc"]``) would be wrongly
    suppressed.  Backslash escapes the next character in every context; a stray
    ``}`` at script level closes nothing.
    """
    brace_depth = 0
    in_quote = False
    at_word_start = True  # the first character of the content begins a word
    i = 0
    n = min(idx, len(text))
    while i < n:
        c = text[i]
        if in_quote:  # only \ and the closing " are significant
            if c == "\\":
                i += 2
                continue
            if c == '"':
                in_quote = False
            i += 1
            continue
        if brace_depth > 0:  # braces nest; only \ { } are significant
            if c == "\\":
                i += 2
                continue
            if c == "{":
                brace_depth += 1
            elif c == "}":
                brace_depth -= 1
            i += 1
            continue
        # Script level: no open brace/quote word.
        if c == "\\":
            at_word_start = False
            i += 2
            continue
        if c in " \t\n;[":
            # Word/command separators ('[' begins a nested command word too).
            at_word_start = True
            i += 1
            continue
        if at_word_start and c == '"':
            in_quote = True
        elif at_word_start and c == "{":
            brace_depth += 1
        at_word_start = False
        i += 1
    return brace_depth > 0 or in_quote


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
        if not stripped:
            cumulative += len(line) + 1
            continue
        # A '#' inside an open brace/quote word is literal, not a comment, and a
        # ] inserted at the previous line's end would be inert (see
        # _bracket_insert_inert): veto it and keep scanning until the word
        # closes — the legitimate plain-text comment-break still fires.
        if _bracket_insert_inert(text, cumulative):
            cumulative += len(line) + 1
            continue
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
        # Script-level, non-blank, non-comment line — stop looking.
        break

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

    A line that begins *inside* an open brace/quote word is content, not a
    command position — a known-command word there (e.g. ``puts`` inside the
    balanced brace of ``[foo {bar`` / ``puts baz}``) is brace text and a ``]``
    inserted before it is inert, leaving the command incomplete (confirmed
    against C Tcl 9.0.3).  Such candidates are vetoed and scanning continues
    until the word closes, so the legitimate plain-text command-break
    (``[foo bar`` / ``puts done``) keeps its fix.  See ``_bracket_insert_inert``
    and ``docs/design/compiler/error-recovery-rust-port.md``.
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
        # Veto: inside an open brace/quote word the line is content, not a
        # command — keep scanning past it rather than inserting an inert ].
        if _bracket_insert_inert(text, cumulative):
            cumulative += len(line) + 1
            continue
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
    *,
    min_line_span: int = 2,
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
    # Must span multiple lines.  The default threshold (3 lines, i.e. span >= 2)
    # is conservative because a braced *value* is often legitimately multi-line;
    # callers that know the brace is an expression relax it to span >= 1.
    line_span = tok.end.line - tok.start.line
    if line_span < min_line_span:
        return False
    return True


def _detect_missing_brace_at_command(
    tok: Token,
    source: str,
    base_offset: int,
    known_commands: frozenset[str],
    *,
    require_dedent: bool = True,
) -> VirtualToken | None:
    """Detect ``}`` missing when a known command follows the brace content.

    Scans the STR token text line by line and inserts ``}`` at the ``\\n``
    before a line that starts with a known command.  By default the line must
    also be *de-indented* relative to the first content line — the conservative
    rule for braces, whose content is often multi-line data where a
    command-looking word is data, not a real command.

    With ``require_dedent=False`` the de-indent condition is dropped (any
    following known-command line triggers, as for ``[`` recovery).  This is used
    only for ``ArgRole.EXPR`` braces: an expression's content is structured, so a
    bare known-command word at the start of a following line cannot be expression
    syntax and is a strong "forgotten close-brace" signal.
    """
    text = tok.text
    lines = text.split("\n")
    # Need the opening line plus at least one following line to close before.
    # (A trailing newline must not change the outcome: ``if {$x > 5\nset`` and
    # ``...\nset\n`` recover identically.)  The de-indent path naturally finds
    # nothing on two lines; the relaxed expr path closes before the command on
    # the second line.
    if len(lines) < 2:
        return None

    # Determine indentation of the first content line (only needed when the
    # de-indent condition applies).
    first_indent: int | None = None
    if require_dedent:
        for line in lines[1:]:  # skip first line (may be empty right after {)
            stripped = line.lstrip()
            if stripped:
                first_indent = len(line) - len(stripped)
                break
        if first_indent is None:
            return None

    # Scan for a (de-indented) line starting with a known command.
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
        if not require_dedent or (first_indent is not None and indent < first_indent):
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


def _brace_arg_is_expr(cmd: SegmentedCommand) -> bool:
    """True when the unterminated trailing ``{`` is a known ``ArgRole.EXPR`` arg.

    Uses the command registry to classify the brace's argument (``if {…``,
    ``while {…``, ``expr {…``).  Conservative: ``False`` for unknown/user
    commands and any non-expression role, so it can only *enable* the extra
    EXPR-brace recovery, never disable an existing one.
    """
    name = cmd.name
    args = list(cmd.args)
    if not name or not args:
        return False
    try:
        from compiler.registry import REGISTRY
        from compiler.registry.runtime import ArgRole, _resolve_arg_roles

        if REGISTRY.get_any(name) is None:
            return False
        roles, base = _resolve_arg_roles(name, args)
    except Exception:
        log.debug("recovery: arg-role classification failed", exc_info=True)
        return False
    last = len(args) - 1
    rs = roles.get(last - base, frozenset()) or roles.get(last, frozenset())
    return ArgRole.EXPR in rs


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

            # E203: unterminated {  (relaxed span; a 2-line brace only qualifies
            # when it is an expression — see below)
            elif _is_suspicious_str(tok, source, base_offset, min_line_span=1):
                is_expr = _brace_arg_is_expr(cmd)
                line_span = tok.end.line - tok.start.line
                if line_span < 2 and not is_expr:
                    # A 2-line braced *value* is too likely intentional to treat
                    # as unterminated — leave it alone (no E203), as before.
                    continue
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
                if vt is None and is_expr:
                    # A braced expression (if/while/expr {…): a known command on a
                    # following line can't be expr syntax, so close before it even
                    # without a de-indent — recovering cases the conservative
                    # brace rule declines.
                    vt = _detect_missing_brace_at_command(
                        tok,
                        source,
                        base_offset,
                        known_cmds,
                        require_dedent=False,
                    )
                if vt is not None:
                    virtuals.append(vt)
                else:
                    fallback_diags.append(
                        _detect_missing_brace_no_heuristic(tok, source, base_offset),
                    )

    return virtuals, fallback_diags


@dataclass(frozen=True, slots=True)
class RecoveryDetection:
    """The result of one recovery-detection pass over a region.

    A single first parse plus heuristic scan yields *everything* recovery needs,
    so the token path and the diagnostic path can share it rather than each
    re-detecting:

    - :attr:`insertions` — ``offset -> closer`` for re-lexing the recovered
      token/command stream (what the old ``compute_virtual_insertions`` returned).
    - :attr:`virtuals` — the recovery points, each carrying the rich
      :class:`VirtualToken.diagnostic` (range + quick-fix) the LSP surfaces.
    - :attr:`fallback_diags` — diagnostics for unterminated delimiters where no
      heuristic matched (no insertion, an over-running token is kept).
    - :attr:`commands` / :attr:`lexer_warnings` — the first-parse commands and
      raw lexer warnings, reused by :func:`segment_with_recovery` so it need not
      re-segment for the no-recovery case.
    """

    commands: list[SegmentedCommand]
    insertions: dict[int, str]
    virtuals: list[VirtualToken]
    fallback_diags: list[Diagnostic]
    lexer_warnings: list[tuple[SourcePosition, str]]


def detect_recovery(
    source: str,
    body_token: Token | None = None,
) -> RecoveryDetection:
    """Detect unterminated delimiters and decide their recovery in one pass.

    The single detection both the tokeniser and the analyser build on: it parses
    *source* once, scans for unterminated ``[``/``{``/``"``, and returns the
    insertions to re-lex with *and* the diagnostics (with quick-fixes) to show.
    """
    lexer_warnings: list[tuple[SourcePosition, str]] = []
    commands = segment_commands(source, body_token, collect_warnings=lexer_warnings)
    if not commands:
        return RecoveryDetection([], {}, [], [], lexer_warnings)

    base_offset = _base_offset_for(body_token)
    virtuals, fallback_diags = _detect_all_virtual_tokens(commands, source, base_offset)
    insertions = {vt.offset: vt.char for vt in virtuals}
    return RecoveryDetection(commands, insertions, virtuals, fallback_diags, lexer_warnings)


def compute_virtual_insertions(
    source: str,
    body_token: Token | None = None,
) -> dict[int, str]:
    """Compute virtual token insertions for error recovery.

    Thin offset-only view over :func:`detect_recovery`, kept for callers that
    only need the insertions to re-lex with and not the diagnostics.  Returns an
    empty dict when no recovery is needed.
    """
    return detect_recovery(source, body_token).insertions


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


def assemble_recovery_diagnostics(
    det: RecoveryDetection,
    reparse_warnings: list[tuple[SourcePosition, str]],
) -> list[Diagnostic]:
    """Build the LSP diagnostics for a recovery detection.

    The single place the recovery diagnostic set is assembled, so the token path
    and the analyser path surface byte-identical diagnostics: the rich
    per-recovery diagnostics (with quick-fixes), the no-heuristic fallbacks, the
    first-parse lexer warnings, and any warnings the recovered re-lex produced.
    *reparse_warnings* is empty when nothing was inserted.
    """
    diags = [vt.diagnostic for vt in det.virtuals] + list(det.fallback_diags)
    diags += _lexer_warnings_to_diagnostics(det.lexer_warnings)
    if reparse_warnings:
        diags += _lexer_warnings_to_diagnostics(reparse_warnings)
    return _dedupe_diagnostics(diags)


def _dedupe_diagnostics(diags: list[Diagnostic]) -> list[Diagnostic]:
    """Drop exact-duplicate diagnostics, preserving first-seen order.

    The first parse and the recovered re-parse can each emit the same lexer
    warning (e.g. "extra characters after close-quote") for an unchanged region,
    which would otherwise surface as two identical squiggles.  Two diagnostics
    that match in code, range, message *and* quick-fixes are the same diagnostic.
    """
    seen: set[tuple] = set()
    out: list[Diagnostic] = []
    for d in diags:
        key = (
            d.code,
            d.range.start.offset,
            d.range.end.offset,
            d.message,
            tuple(
                (f.range.start.offset, f.range.end.offset, f.new_text, f.description)
                for f in (d.fixes or ())
            ),
        )
        if key in seen:
            continue
        seen.add(key)
        out.append(d)
    return out


def segment_with_recovery(
    source: str,
    body_token: Token | None = None,
) -> tuple[list[SegmentedCommand], list[Diagnostic]]:
    """Parse source, detect imbalances, re-parse with virtual tokens.

    1. ``detect_recovery(source, body_token)`` — first parse + heuristic scan
    2. ``segment_commands(source, body_token, virtual_insertions=...)`` — clean re-parse
    3. Return ``(clean_commands, diagnostics)``

    When no imbalances are detected, the first parse result is returned
    directly (zero overhead for clean files).

    Lexer warnings (e.g. "extra characters after close-brace") are also
    harvested and converted to diagnostics.
    """
    det = detect_recovery(source, body_token)

    if not det.commands or not det.virtuals:
        return det.commands, assemble_recovery_diagnostics(det, [])

    # Re-parse with virtual tokens injected.
    reparse_warnings: list[tuple[SourcePosition, str]] = []
    clean_commands = segment_commands(
        source,
        body_token,
        virtual_insertions=det.insertions,
        collect_warnings=reparse_warnings,
    )
    return clean_commands, assemble_recovery_diagnostics(det, reparse_warnings)
