"""Build a green CST from the lexer token stream.

This re-shapes the existing lexer output into the canonical tree rather than
introducing a second parser: it tokenises the region through
:func:`compiler.parsing.green_tree.tokenise` (sharing the analysis-scoped lex
memo) and groups the stream into commands and words, folding ``SEP`` / ``EOL`` /
``COMMENT`` tokens into attached :class:`GreenTrivia`.

The grouping mirrors :func:`compiler.parsing.command_segmenter._segment_raw`
exactly — same word-merging, same ``{*}`` handling, same continuation handling,
and the same ``word_boundary`` range rule (stale-boundary quirk included) — so
that segments derived from the tree are byte-identical to today's segmenter
(see :mod:`compiler.parsing.syntax.segment`).

Raw fragment text is recovered by *start-to-start tiling*: the lexer advances
its cursor monotonically, so ``source[tok[i].start : tok[i+1].start]`` is exactly
the bytes fragment *i* occupies — delimiters included — which sidesteps the
inner-end / empty-delimiter (#527) convention entirely.
"""

from __future__ import annotations

from dataclasses import replace

from shared.tokens import SourcePosition, TokenType

from ..green_tree import tokenise
from .green import GreenNode, GreenToken, GreenTrivia, SyntaxKind, TriviaKind, trivia

_Warnings = tuple[tuple[SourcePosition, str], ...]

_SEP_OR_EOL = (TokenType.SEP, TokenType.EOL)


def build_document(
    source: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
    *,
    insidequote: bool = False,
    virtual_insertions: dict[int, str] | None = None,
    line_starts: list[int] | None = None,
) -> tuple[GreenNode, _Warnings]:
    """Tokenise *source* and build its green ``DOCUMENT`` node.

    Returns ``(document, warnings)``; *warnings* is the lexer's non-fatal
    warning tuple, passed through unchanged for diagnostic emission.  *line_starts*
    is an optional pre-built line index for *source*, shared with the lexer to
    avoid an O(n) rebuild.
    """
    tokens, warnings = tokenise(
        source,
        base_offset,
        base_line,
        base_col,
        insidequote=insidequote,
        virtual_insertions=virtual_insertions,
        line_starts=line_starts,
    )
    n = len(tokens)

    def raw_of(i: int) -> str:
        lo = tokens[i].start.offset - base_offset
        hi = (tokens[i + 1].start.offset - base_offset) if i + 1 < n else len(source)
        return source[lo:hi]

    commands: list[GreenNode] = []
    cur_words: list[GreenNode] = []  # finished WORD nodes of the current command
    frag: list[GreenToken] = []  # fragments of the word currently being built
    pending: list[GreenTrivia] = []  # leading trivia awaiting the next fragment
    markers: list[GreenToken] = []  # {*} markers awaiting their word
    last_comment: str | None = None  # comment(s) accumulating for the next command
    prev_type = TokenType.EOL

    # Range tracking, mirroring _segment_raw's word_boundary rule.  All offsets
    # are region-relative (base subtracted) so the stored end stays anchor-free.
    first_region: int | None = None  # region offset of all_tokens[0].start
    last_end_region = 0  # region offset of the last content token's end
    word_boundary: int | None = None  # region offset after the last word fragment

    def finish_word() -> None:
        nonlocal frag, markers
        if not frag:
            return
        cur_words.append(GreenNode(SyntaxKind.WORD, tuple(frag), expand_markers=tuple(markers)))
        frag = []
        markers = []

    def reset_command() -> None:
        nonlocal cur_words, first_region, last_end_region, word_boundary
        cur_words = []
        first_region = None
        last_end_region = 0
        word_boundary = None

    def range_end_rel(eol_region: int | None) -> int | None:
        if first_region is None:
            return None
        if eol_region is not None and prev_type not in _SEP_OR_EOL:
            boundary = eol_region  # the EOL directly follows the last token
        else:
            boundary = word_boundary
        if boundary is not None and boundary - 1 >= first_region:
            end_region = boundary - 1
        else:
            end_region = last_end_region  # fallback: last token's inner end
        return end_region - first_region

    def close_command(terminator: GreenTrivia, eol_region: int | None, pc: str | None) -> None:
        nonlocal pending, markers, frag
        end_rel = range_end_rel(eol_region)
        # Trailing whitespace after the last token + the terminator attach to the
        # last token (document order) as trailing trivia, keeping the tree
        # lossless; the command's *range* comes from end_rel, not this trivia.
        trail = (*pending, terminator)
        pending = []
        extra: tuple[GreenToken, ...] = ()
        if frag:
            frag[-1] = replace(frag[-1], trailing=frag[-1].trailing + trail)
            finish_word()
        elif markers:
            markers[-1] = replace(markers[-1], trailing=markers[-1].trailing + trail)
            extra = tuple(markers)
            markers = []
        elif cur_words:
            last = cur_words[-1]
            kids = (
                *last.children[:-1],
                replace(last.children[-1], trailing=last.children[-1].trailing + trail),
            )
            cur_words[-1] = replace(last, children=kids)
        commands.append(
            GreenNode(
                SyntaxKind.COMMAND,
                (*cur_words, *extra),
                range_end_rel=end_rel,
                preceding_comment=pc,
            )
        )
        reset_command()

    for i, tok in enumerate(tokens):
        raw = raw_of(i)
        ttype = tok.type
        region = tok.start.offset - base_offset

        if ttype is TokenType.COMMENT:
            line = raw.lstrip("#").strip()
            last_comment = line if last_comment is None else last_comment + "\n" + line
            pending.append(trivia(TriviaKind.COMMENT, raw))
            continue
        if ttype is TokenType.SEP:
            if prev_type not in _SEP_OR_EOL:
                word_boundary = region
            pending.append(trivia(TriviaKind.WHITESPACE, raw))
            prev_type = TokenType.SEP
            continue
        if ttype is TokenType.EOL:
            eol_triv = trivia(TriviaKind.EOL, raw)
            if frag or cur_words:
                # A real command closes here: it takes the accumulated comment.
                close_command(eol_triv, region, last_comment)
                last_comment = None
            elif markers:
                # A dangling-{*} command closes but keeps no comment; the blank
                # line still resets accumulation (segmenter's argv-empty branch).
                if raw.count("\n") > 1:
                    last_comment = None
                close_command(eol_triv, region, None)
            else:
                if raw.count("\n") > 1:
                    last_comment = None
                pending.append(eol_triv)
            prev_type = TokenType.EOL
            continue
        # A backslash-newline that is a *separator* (between/after words, after a
        # ``{*}`` marker, mid-word) is lexed as ``SEP`` and folded above.  The
        # lexer emits ``\<newline>`` as an ``ESC`` token *only* as quoted-word
        # content (``"\<newline>"`` — terminated or not), where it is a real
        # fragment of that word, so it must fall through to the fragment path:
        # folding it would drop a token the lexer reports and lose the (possibly
        # only) fragment of the quoted word.

        leaf = GreenToken(
            token_type=ttype,
            text=tok.text,
            raw=raw,
            end_rel=tok.end.offset - tok.start.offset,
            in_quote=tok.in_quote,
            leading=tuple(pending),
        )
        pending = []
        if first_region is None:
            first_region = region
        last_end_region = tok.end.offset - base_offset

        if ttype is TokenType.EXPAND:
            # {*} ends any word in progress and marks the *next* word for
            # expansion; like the segmenter it advances prev_type to SEP without
            # touching word_boundary (the source of the stale-boundary quirk).
            finish_word()
            markers.append(leaf)
            prev_type = TokenType.SEP
            continue

        if prev_type in _SEP_OR_EOL:
            finish_word()
            frag = [leaf]
        else:
            frag.append(leaf)
        prev_type = ttype

    # A command left open at end-of-stream (only reachable via recovery — the
    # lexer otherwise emits a trailing EOL that closes the last command).
    if frag or cur_words or markers:
        end_rel = range_end_rel(None)
        pc = last_comment if (frag or cur_words) else None
        finish_word()  # consumes frag and its leading markers
        children = (*cur_words, *markers)
        if children:
            commands.append(
                GreenNode(SyntaxKind.COMMAND, children, range_end_rel=end_rel, preceding_comment=pc)
            )
            reset_command()

    document = GreenNode(SyntaxKind.DOCUMENT, tuple(commands), trailing=tuple(pending))
    return document, warnings
