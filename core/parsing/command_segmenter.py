"""Shared command segmentation for the Tcl token stream.

Splits a flat token stream into per-command structures at EOL boundaries.
Both the analyser and lowerer consume these structures instead of running
their own parallel token-iteration loops.

Includes error recovery: when an unclosed brace or bracket causes the lexer
to swallow the rest of the file, the segmenter scans forward for a command
separator followed by a known command name and resumes segmentation there.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from enum import Enum, auto
from typing import TYPE_CHECKING

from ..analysis.semantic_model import Range
from ..common.ranges import range_from_tokens
from .known_commands import known_command_names

if TYPE_CHECKING:
    from ..commands.registry.command_registry import CommandRegistry
from .lexer import TclLexer
from .tokens import SourcePosition, Token, TokenType

log = logging.getLogger(__name__)


class UnclosedDelimiter(Enum):
    """Which delimiter was left unclosed in a partial command."""

    BRACE = auto()  # {
    BRACKET = auto()  # [
    QUOTE = auto()  # "


def _word_piece(tok: Token) -> str:
    """Return the source-level text fragment for a single token.

    Variables are prefixed with ``$`` and command substitutions are
    wrapped in ``[...]`` so that the result mirrors what the user wrote.
    """
    if tok.type is TokenType.VAR:
        # Use ${name} form only when the name doesn't contain '}'.
        # Names with '}' (e.g. array indices with braced expressions
        # like ``a(1[expr {3 - 1}])``) would cause the first '}' to
        # prematurely close the ${...} form during runtime substitution.
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


# Minimum number of lines a suspicious STR token must span to trigger
# recovery.  Tuned to avoid false positives on multi-line string literals
# that are genuinely part of a single command.
_RECOVERY_LINE_THRESHOLD = 3


@dataclass(slots=True)
class SegmentedCommand:
    """A single Tcl command parsed from the token stream."""

    range: Range
    argv: list[Token]
    texts: list[str]
    single_token_word: list[bool]
    all_tokens: list[Token]
    preceding_comment: str | None = None
    is_partial: bool = False
    partial_delimiter: UnclosedDelimiter | None = None
    expand_word: list[bool] | None = None  # {*} expansion on each word
    subcommand: str | None = None  # resolved subcommand name when known

    @property
    def name(self) -> str:
        return self.texts[0] if self.texts else ""

    @property
    def args(self) -> list[str]:
        return self.texts[1:]

    @property
    def arg_tokens(self) -> list[Token]:
        return self.argv[1:]

    @property
    def arg_single_token(self) -> list[bool]:
        return self.single_token_word[1:]


def _make_lexer(
    source: str,
    body_token: Token | None,
    virtual_insertions: dict[int, str] | None = None,
) -> TclLexer:
    """Create a lexer with appropriate base offsets for *body_token*."""
    if body_token is None:
        return TclLexer(source, virtual_insertions=virtual_insertions)
    if body_token.type in (TokenType.STR, TokenType.CMD):
        return TclLexer(
            source,
            base_offset=body_token.start.offset + 1,
            base_line=body_token.start.line,
            base_col=body_token.start.character + 1,
            virtual_insertions=virtual_insertions,
        )
    return TclLexer(
        source,
        base_offset=body_token.start.offset,
        base_line=body_token.start.line,
        base_col=body_token.start.character,
        virtual_insertions=virtual_insertions,
    )


def _find_recovery_offset(
    token_text: str,
    token_start_offset: int,
    known_commands: frozenset[str],
) -> int | None:
    """Find a byte offset in the original source where parsing can resume.

    Scans the inner text of a suspiciously large token line by line.
    Returns the source offset of the first line whose leading word is a
    known command, or ``None`` if no recovery point is found.
    """
    # inner_offset tracks position within token_text.
    # +1 accounts for the opening delimiter ({ or [) in the source.
    inner_offset = 0
    for i, line in enumerate(token_text.split("\n")):
        if i == 0:
            # Skip the first line — it's part of the broken command.
            inner_offset += len(line) + 1
            continue
        stripped = line.lstrip()
        if stripped:
            # Extract the first word (up to whitespace or special char).
            end = 0
            while end < len(stripped) and stripped[end] not in " \t\n\r;{[":
                end += 1
            first_word = stripped[:end]
            if first_word in known_commands:
                # source_offset = token_start_offset(the '{') + 1(for content)
                #               + inner_offset + leading_whitespace
                leading_ws = len(line) - len(stripped)
                return token_start_offset + 1 + inner_offset + leading_ws
        inner_offset += len(line) + 1

    return None


_TOKEN_TYPE_TO_DELIMITER = {
    TokenType.STR: UnclosedDelimiter.BRACE,
    TokenType.CMD: UnclosedDelimiter.BRACKET,
    TokenType.ESC: UnclosedDelimiter.QUOTE,
}


def _has_suspicious_token(
    cmd: SegmentedCommand,
    source_len: int,
) -> tuple[Token, UnclosedDelimiter] | None:
    """Return info about a token that looks like an unclosed delimiter.

    A token is suspicious when it spans many lines AND its end offset
    reaches the end of the source — meaning the delimiter was never closed
    and the lexer consumed everything to EOF.

    Checks STR (unclosed ``{``), CMD (unclosed ``[``), and ESC (unclosed ``"``).
    """
    for tok in cmd.all_tokens:
        delimiter = _TOKEN_TYPE_TO_DELIMITER.get(tok.type)
        if delimiter is None:
            continue
        # CMD tokens reaching EOF are always unterminated — valid [...]
        # always ends with ].  No line-span threshold needed.
        if delimiter is UnclosedDelimiter.BRACKET:
            if tok.end.offset >= source_len - 1:
                return tok, delimiter
            continue
        line_span = tok.end.line - tok.start.line
        if line_span < _RECOVERY_LINE_THRESHOLD:
            continue
        # The token must reach EOF — properly closed delimiters end before EOF.
        if tok.end.offset >= source_len - 1:
            return tok, delimiter
    return None


def _segment_raw(
    source: str,
    body_token: Token | None,
    virtual_insertions: dict[int, str] | None = None,
    collect_warnings: list[tuple[SourcePosition, str]] | None = None,
) -> list[SegmentedCommand]:
    """Segment without error recovery — the inner loop."""
    lexer = _make_lexer(source, body_token, virtual_insertions)

    commands: list[SegmentedCommand] = []
    argv: list[Token] = []
    texts: list[str] = []
    single: list[bool] = []
    expand: list[bool] = []
    all_tokens: list[Token] = []
    prev_type = TokenType.EOL
    last_comment: str | None = None
    next_expand = False
    has_expand = False

    while True:
        tok = lexer.get_token()
        if tok is None:
            break

        if tok.type is TokenType.COMMENT:
            last_comment = tok.text.lstrip("#").strip()
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EXPAND:
            # {*} prefix — mark the next word for expansion
            all_tokens.append(tok)
            next_expand = True
            has_expand = True
            # Set prev_type to SEP so the next token starts a new word
            prev_type = TokenType.SEP
            continue
        if tok.type is TokenType.EOL:
            if argv:
                commands.append(
                    SegmentedCommand(
                        range=range_from_tokens(all_tokens),
                        argv=argv,
                        texts=texts,
                        single_token_word=single,
                        all_tokens=all_tokens,
                        preceding_comment=last_comment,
                        expand_word=expand if has_expand else None,
                    )
                )
                last_comment = None
            argv = []
            texts = []
            single = []
            expand = []
            all_tokens = []
            has_expand = False
            next_expand = False
            prev_type = tok.type
            continue

        all_tokens.append(tok)
        piece = _word_piece(tok)

        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
            texts.append(piece)
            single.append(True)
            expand.append(next_expand)
            next_expand = False
        else:
            if texts:
                texts[-1] += piece
                single[-1] = False
                # Keep argv token range aligned to the full Tcl word span.
                prev = argv[-1]
                argv[-1] = Token(
                    type=prev.type,
                    text=prev.text,
                    start=prev.start,
                    end=tok.end,
                )
            else:
                argv.append(tok)
                texts.append(piece)
                single.append(True)
                expand.append(next_expand)
                next_expand = False

        prev_type = tok.type

    # Trailing command without final EOL.
    if argv:
        commands.append(
            SegmentedCommand(
                range=range_from_tokens(all_tokens),
                argv=argv,
                texts=texts,
                single_token_word=single,
                all_tokens=all_tokens,
                preceding_comment=last_comment,
                expand_word=expand if has_expand else None,
            )
        )

    # Harvest non-fatal warnings from the lexer for diagnostic emission.
    if collect_warnings is not None:
        collect_warnings.extend(lexer.warnings)

    return commands


@dataclass(frozen=True, slots=True)
class TopLevelChunk:
    """A region of top-level source mapped to its commands.

    Used for incremental re-analysis: when an edit changes only one
    chunk, everything before it can be reused from the cache.
    """

    index: int
    start_offset: int
    end_offset: int  # exclusive
    source_hash: int  # hash of the raw text slice
    commands: tuple[SegmentedCommand, ...]


def segment_top_level_chunks(source: str) -> list[TopLevelChunk]:
    """Split *source* into top-level chunks, one per command.

    Each chunk records the byte range and a hash of the raw source text
    it covers.  Downstream consumers compare hashes between edits to
    identify the first changed chunk.
    """
    commands = segment_commands(source)
    chunks: list[TopLevelChunk] = []
    for i, cmd in enumerate(commands):
        start = cmd.range.start.offset
        cmd_end = cmd.range.end.offset
        # Extend end to include any trailing whitespace/newlines up to
        # the start of the next command (so chunks tile the source).
        if i + 1 < len(commands):
            tile_end = commands[i + 1].range.start.offset
        else:
            tile_end = len(source)
        # Hash only the command text (start..cmd_end), not the
        # trailing whitespace, so appending a new command doesn't
        # invalidate the previous chunk's hash.
        cmd_text = source[start:cmd_end]
        chunks.append(
            TopLevelChunk(
                index=i,
                start_offset=start,
                end_offset=tile_end,
                source_hash=hash(cmd_text),
                commands=(cmd,),
            )
        )
    return chunks


def find_first_dirty_chunk(
    old_chunks: list[TopLevelChunk],
    new_chunks: list[TopLevelChunk],
) -> int:
    """Return the index of the first chunk that differs between two versions.

    Compares ``source_hash`` values pairwise.  Returns the length of the
    shorter list when all shared chunks match (i.e. one version has extra
    chunks appended).
    """
    for i in range(min(len(old_chunks), len(new_chunks))):
        if old_chunks[i].source_hash != new_chunks[i].source_hash:
            return i
    return min(len(old_chunks), len(new_chunks))


def _resolve_subcommands(
    commands: list[SegmentedCommand],
    registry: CommandRegistry,
) -> None:
    """Tag each segmented command with its resolved subcommand name."""
    for cmd in commands:
        if len(cmd.texts) < 2:
            continue
        spec = registry.get_any(cmd.texts[0])
        if spec is None or not spec.subcommands:
            continue
        candidate = cmd.texts[1]
        if candidate in spec.subcommands:
            cmd.subcommand = candidate


def segment_commands(
    source: str,
    body_token: Token | None = None,
    *,
    known_commands: frozenset[str] | None = None,
    virtual_insertions: dict[int, str] | None = None,
    collect_warnings: list[tuple[SourcePosition, str]] | None = None,
    registry_snapshot: CommandRegistry | None = None,
    recovery: bool = True,
) -> list[SegmentedCommand]:
    """Split a token stream into per-command structures at EOL boundaries.

    When *known_commands* is provided (or the REGISTRY is available),
    the segmenter attempts error recovery on commands that appear to
    contain an unclosed delimiter (``{``, ``[``, or ``"``): it scans
    forward for a line starting with a known command name and resumes
    segmentation from there.  Set *recovery* to ``False`` to disable
    this heuristic while keeping subcommand resolution.

    When *virtual_insertions* is provided, the lexer sees zero-width
    virtual characters at the specified offsets (used by error recovery
    to inject missing delimiters).

    When *collect_warnings* is provided, non-fatal lexer warnings
    (e.g. "extra characters after close-brace") are appended to it.
    """
    commands = _segment_raw(source, body_token, virtual_insertions, collect_warnings)

    if not commands:
        return commands

    # Resolve subcommands when a registry snapshot is provided.
    if registry_snapshot is not None:
        _resolve_subcommands(commands, registry_snapshot)

    if not recovery:
        return commands

    # Only attempt recovery for top-level segmentation (no body_token)
    # to avoid false positives on legitimate multi-line string arguments.
    if body_token is not None:
        return commands

    # Check the last command for a token that looks like an unclosed delimiter.
    last_cmd = commands[-1]
    result = _has_suspicious_token(last_cmd, len(source))
    if result is None:
        return commands

    suspicious_tok, delimiter = result

    if known_commands is None:
        try:
            known_commands = known_command_names()
        except Exception:
            log.debug("segmenter: failed to load known commands for recovery", exc_info=True)
            return commands

    recovery_offset = _find_recovery_offset(
        suspicious_tok.text,
        suspicious_tok.start.offset,
        known_commands,
    )
    if recovery_offset is None:
        return commands

    # Mark the broken command as partial.
    last_cmd.is_partial = True
    last_cmd.partial_delimiter = delimiter

    # Truncate the partial command: create a synthetic range ending
    # just before the recovery point.
    partial_end = SourcePosition(
        line=suspicious_tok.start.line,
        character=suspicious_tok.start.character,
        offset=suspicious_tok.start.offset,
    )
    last_cmd.range = Range(start=last_cmd.range.start, end=partial_end)

    # Re-segment from the recovery point in the original source.
    remaining = source[recovery_offset:]
    if remaining.strip():
        recovered = _segment_raw(
            remaining,
            body_token=Token(
                type=TokenType.ESC,
                text=remaining,
                start=SourcePosition(line=0, character=0, offset=recovery_offset),
                end=SourcePosition(line=0, character=0, offset=recovery_offset + len(remaining)),
            ),
            collect_warnings=collect_warnings,
        )
        commands.extend(recovered)

    return commands
