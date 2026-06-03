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

from shared.diagnostic import Range
from shared.document_buffer import DocumentBuffer
from shared.hashing import stable_text_hash

from .known_commands import known_command_names

if TYPE_CHECKING:
    from compiler.registry.command_registry import CommandRegistry

from shared.tokens import SourcePosition, Token, TokenType

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
        # Distinguish bare ``$a(idx)`` from braced ``${a(idx)}``:
        # ``Token.end.offset`` is the end of the source span, ``tok.text``
        # excludes the leading ``$`` and the braces.  Bare form leaves
        # ``end-start == len(text)``; braced adds 1 (the ``${`` and ``}``
        # against the omitted ``${`` net to one unaccounted byte).
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        # Bare ``$arr(idx)`` with a *substituted* index (``$x`` or ``[cmd]``
        # inside the parens) must round-trip verbatim — wrapping in braces
        # would disable array-element interpretation and turn the recursive
        # substitution into a literal scalar lookup
        # (cmdAH-1.4 / 1.5 ``$numargErrors($cmd)``).
        if (
            not is_braced
            and "(" in tok.text
            and tok.text.endswith(")")
            and ("$" in tok.text or "[" in tok.text)
        ):
            return "$" + tok.text
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


def _base_position_for(body_token: Token | None) -> tuple[int, int, int]:
    """Return ``(base_offset, base_line, base_col)`` for lexing *body_token*.

    Mirrors the offsets the lexer is anchored at: a braced/bracketed body
    starts one character past the opening delimiter; an ESC (recovery) body
    or top-level source starts at the token's own position.
    """
    if body_token is None:
        return (0, 0, 0)
    if body_token.type in (TokenType.STR, TokenType.CMD):
        return (
            body_token.start.offset + 1,
            body_token.start.line,
            body_token.start.character + 1,
        )
    return (
        body_token.start.offset,
        body_token.start.line,
        body_token.start.character,
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
    """Segment without error recovery — the inner loop.

    Builds the canonical green concrete syntax tree for the region and derives
    the ``SegmentedCommand`` list from it.  The output is byte-identical to the
    former hand-rolled token loop (verified over the real-world corpus, 120k
    randomised differential cases, and nested-body anchoring); the tree is the
    single representation that the formatter, minifier, AOT lowering, and the
    per-command tooling are migrating onto.  See
    :mod:`compiler.parsing.syntax`.

    The import is deferred to break the ``syntax`` ↔ ``command_segmenter`` cycle
    (the tree's segment derivation reuses ``SegmentedCommand`` and ``_word_piece``
    from this module).
    """
    from .syntax import build_document, segments_from_document

    base_offset, base_line, base_col = _base_position_for(body_token)
    document, warnings = build_document(
        source,
        base_offset,
        base_line,
        base_col,
        virtual_insertions=virtual_insertions,
    )
    if collect_warnings is not None:
        collect_warnings.extend(warnings)
    return segments_from_document(document, base_offset, base_line, base_col, text=source)


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


def tile_commands(
    commands: list[SegmentedCommand],
    source: str,
    *,
    start_index: int = 0,
    final_end: int | None = None,
) -> list[TopLevelChunk]:
    """Tile *commands* into :class:`TopLevelChunk`s over *source*.

    Each chunk spans from its command's start to the next command's start
    (so chunks tile contiguously), with the last command extending to
    *final_end* (defaulting to ``len(source)``).  ``source`` must be the full
    document text — command offsets are absolute, so the hash slice
    ``source[start:cmd_end]`` is taken against absolute positions.

    Hashes cover only the command text (not trailing whitespace) so that
    appending a new command does not invalidate the previous chunk's hash.
    ``range.end.offset`` is the command's last character (inclusive), so the
    hash slice is end-*exclusive* at ``cmd_end + 1`` — otherwise a change to
    only the final character of a command would leave its hash unchanged and
    the dirty-chunk detection would miss the edit.
    """
    chunks: list[TopLevelChunk] = []
    n = len(commands)
    last = final_end if final_end is not None else len(source)
    for i, cmd in enumerate(commands):
        start = cmd.range.start.offset
        cmd_end = cmd.range.end.offset
        tile_end = commands[i + 1].range.start.offset if i + 1 < n else last
        cmd_text = source[start : cmd_end + 1]
        chunks.append(
            TopLevelChunk(
                index=start_index + i,
                start_offset=start,
                end_offset=tile_end,
                source_hash=stable_text_hash(cmd_text),
                commands=(cmd,),
            )
        )
    return chunks


def segment_top_level_chunks(source: str) -> list[TopLevelChunk]:
    """Split *source* into top-level chunks, one per command.

    Each chunk records the byte range and a hash of the raw source text
    it covers.  Downstream consumers compare hashes between edits to
    identify the first changed chunk.
    """
    return tile_commands(segment_commands(source), source)


def find_first_dirty_chunk(
    old_chunks: list[TopLevelChunk],
    new_chunks: list[TopLevelChunk],
) -> int:
    """Return the index of the first chunk that differs between two versions.

    Compares both the ``source_hash`` (command text) **and** ``start_offset``
    (absolute position) pairwise.  Position matters because a chunk's cached IR
    / analyser snapshot / diagnostics carry *absolute* source positions: a chunk
    whose text is unchanged but which *moved* (e.g. a blank line or comment
    inserted above it, a ``# tcl-dialect:`` directive added) would otherwise be
    reused with stale line numbers.  Returns the length of the shorter list when
    all shared chunks match (i.e. one version has extra chunks appended — an
    append shifts no existing chunk's offset, so that case stays incremental).
    """
    for i in range(min(len(old_chunks), len(new_chunks))):
        if (
            old_chunks[i].source_hash != new_chunks[i].source_hash
            or old_chunks[i].start_offset != new_chunks[i].start_offset
        ):
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
        # Resolve absolute line/character at recovery_offset so the lexer's
        # base_line / base_col reflect the original source rather than (0, 0)
        # — otherwise every recovered token reports its position as if the
        # slice started a fresh file. Offsets are already absolute via
        # base_offset.
        buf = DocumentBuffer.from_source(source)
        recovery_pos = buf.offset_to_position(recovery_offset)
        end_pos = buf.offset_to_position(recovery_offset + len(remaining))
        recovered = _segment_raw(
            remaining,
            body_token=Token(
                type=TokenType.ESC,
                text=remaining,
                start=recovery_pos,
                end=end_pos,
            ),
            collect_warnings=collect_warnings,
        )
        commands.extend(recovered)

    return commands
