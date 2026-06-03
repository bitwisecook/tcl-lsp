"""Derive byte-identical ``SegmentedCommand``s from the green CST.

This is the bridge that lets :mod:`compiler.parsing.command_segmenter` produce
its public ``SegmentedCommand`` shape from the canonical tree instead of its own
token loop, without changing a single downstream consumer.  Every field —
``range``, ``argv`` (with compound-word merging), ``texts``, ``single_token_word``,
``all_tokens`` (including ``{*}`` markers), ``preceding_comment``, and
``expand_word`` — is reconstructed to match ``_segment_raw`` exactly.
"""

from __future__ import annotations

from shared.diagnostic import Range
from shared.tokens import Token, TokenType

from ..command_segmenter import SegmentedCommand, _word_piece
from .green import GreenNode, SyntaxKind
from .red import SyntaxNode, SyntaxToken, SyntaxTree


def _command_words(cmd: SyntaxNode) -> list[SyntaxNode]:
    return [c for c in cmd.children() if isinstance(c, SyntaxNode) and c.kind is SyntaxKind.WORD]


def _command_segment(cmd: SyntaxNode, words: list[SyntaxNode]) -> SegmentedCommand:

    argv: list[Token] = []
    texts: list[str] = []
    single: list[bool] = []
    expand: list[bool] = []
    braced: list[bool] = []
    quoted: list[bool] = []

    for word in words:
        frags = [c for c in word.children() if isinstance(c, SyntaxToken)]
        first, last = frags[0], frags[-1]
        # The representative word token: type/text/start from the first fragment,
        # end from the last — the segmenter's compound-word merge.
        argv.append(
            Token(
                type=first.token_type,
                text=first.text,
                start=first.start_position,
                end=last.end_position,
            )
        )
        texts.append("".join(_word_piece(f.to_token()) for f in frags))
        single.append(len(frags) == 1)
        expand.append(bool(word.green.expand_markers))
        # Per-word shape for the formatter / minifier: a braced word's first
        # fragment is a STR ({…}); a quoted word's first fragment's raw begins
        # with the opening double-quote.
        braced.append(first.token_type is TokenType.STR)
        quoted.append(first.green.raw.startswith('"'))

    syntax_tokens = list(cmd.tokens())
    all_tokens = [st.to_token() for st in syntax_tokens]
    # The segmenter records expansion for the command whenever it saw any {*},
    # including a dangling one that expands no word.
    has_expand = any(t.type is TokenType.EXPAND for t in all_tokens)

    # The command starts at its first token — the ``{*}`` marker when present,
    # else the first fragment.  Its end is the relative width the build stored
    # from the segmenter's word_boundary rule, resolved against that first token.
    first_st = syntax_tokens[0]
    start = first_st.start_position
    end = cmd.tree.position_at(first_st.raw_start + (cmd.green.range_end_rel or 0))
    return SegmentedCommand(
        range=Range(start=start, end=end),
        argv=argv,
        texts=texts,
        single_token_word=single,
        all_tokens=all_tokens,
        preceding_comment=cmd.green.preceding_comment,
        expand_word=expand if has_expand else None,
        braced_word=braced,
        quoted_word=quoted,
    )


def segments_from_tree(tree: SyntaxTree) -> list[SegmentedCommand]:
    """Produce the ``SegmentedCommand`` list for an anchored CST."""
    out: list[SegmentedCommand] = []
    for cmd in tree.root.children():
        if not (isinstance(cmd, SyntaxNode) and cmd.kind is SyntaxKind.COMMAND):
            continue
        words = _command_words(cmd)
        # A command of only dangling {*} markers (no real word) is discarded by
        # the segmenter — it produces no SegmentedCommand.
        if words:
            out.append(_command_segment(cmd, words))
    return out


def segments_from_document(
    document: GreenNode,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
    *,
    text: str | None = None,
    line_starts: list[int] | None = None,
) -> list[SegmentedCommand]:
    """Anchor *document* and produce its ``SegmentedCommand`` list.

    *text* is the region's source and *line_starts* its line index; passing them
    lets the red layer skip rebuilding the text (identical by losslessness) and
    the line index (identical to the lexer's).
    """
    return segments_from_tree(
        SyntaxTree(document, base_offset, base_line, base_col, text=text, line_starts=line_starts)
    )
