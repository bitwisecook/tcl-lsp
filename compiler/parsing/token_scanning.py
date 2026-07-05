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

"""Canonical green-tree-backed token-scanning helpers.

This is the single home for the small "walk a word/region's tokens" helpers
that several pipeline stages used to re-implement on top of a raw
:class:`~compiler.parsing.lexer.TclLexer`.  Routing them here means each
``(region, mode)`` is tokenised once per :func:`green_tree_scope` (the leaf
``tokenise`` memo) instead of once per consumer.

Home rationale: it lives under ``compiler.parsing`` so every concern that
needs it — ``server``, ``tooling``, ``analyser``, ``dialects`` — can import it
without an import-linter carve-out (``dialects/`` may import
``compiler.parsing.*`` but not ``compiler.var_refs`` / ``compiler.token_helpers``).
For the same reason it must **not** import the command registry at module
scope (that would create a ``parsing`` ↔ ``registry`` cycle); any registry-aware
helper imports it lazily inside the function body, mirroring
``syntax.descend.descend_command``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from shared.tokens import Token, TokenType

from .green_tree import tokenise

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator


def word_piece(
    tok: Token,
    *,
    bare_arrays_split: bool = True,
    normalise_var_braces: bool = True,
) -> str:
    """Return the source-level text fragment for a single token.

    Variables are prefixed with ``$`` and command substitutions wrapped in
    ``[...]`` so the result mirrors what the user wrote.  The braced-vs-bare
    distinction is carried by *natural spelling* (no ``$={}`` marker): tclsh 9
    resolves a braced ``${a(1)}`` by its whole literal name, while a bare
    ``$a(1)`` splits into an array load.  Two flags reproduce the three
    historically divergent reconstructions this subsumes:

    * ``bare_arrays_split`` (default ``True``) — keep a bare array-shaped name
      ``$a(1)`` / ``$a($i)`` bare so codegen splits it into an array load.  With
      ``False`` only a *substituted*-index bare array stays bare; a literal
      ``$a(1)`` normalises to ``${a(1)}`` (the segmenter's reconstruction).
    * ``normalise_var_braces`` (default ``True``) — normalise a bare scalar
      ``$foo`` to the equivalent ``${foo}``.  Pass ``False`` for verbatim
      round-tripping (the expr-argument matcher wants exactly what the user
      typed); then every bare word stays ``$name``.

    A name containing ``}`` is always emitted as ``$name`` so the first ``}``
    cannot prematurely close a ``${…}`` wrapper.
    """
    if tok.type is TokenType.VAR:
        # ``Token.end.offset`` is the last source byte (inclusive); ``tok.text``
        # excludes the leading ``$`` (bare) or both braces (``${…}``).  Net
        # delta is 0 for bare and 1 for braced.
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        if is_braced:
            if "}" in tok.text:
                return "$" + tok.text
            return f"${{{tok.text}}}"
        if not normalise_var_braces:
            return "$" + tok.text
        is_array = "(" in tok.text and tok.text.endswith(")")
        if is_array:
            if bare_arrays_split:
                return "$" + tok.text
            if "$" in tok.text or "[" in tok.text:
                return "$" + tok.text
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


# --- green-tree-backed token scanners -------------------------------------
#
# Each routes through ``tokenise`` so a region scanned by several consumers in
# one ``green_tree_scope`` is lexed once.  ``tokenise(text, 0, 0, 0)`` yields a
# token stream byte-identical to ``TclLexer(text)`` (verified); the absolute
# variants pass the owning token's content base through unchanged.


def extract_scalar_var_name(text: str) -> str | None:
    """Return the scalar name when *text* is exactly one ``$var`` / ``${var}``.

    The whole word must be a single variable substitution naming a scalar (no
    array index) with the conventional name shape (letter/underscore start,
    word chars and ``::`` separators).  Returns ``None`` otherwise.  This is the
    token-based replacement for the simple-var-word regexes — stricter than the
    regex on malformed input, identical on valid input.
    """
    tokens, _ = tokenise(text, 0, 0, 0)
    tok = tokens[0] if tokens else None
    if tok is None or tok.type is not TokenType.VAR or tok.start.offset != 0:
        return None
    name = tok.text
    if text not in (f"${name}", f"${{{name}}}"):
        return None
    if not name or not (name[0].isalpha() or name[0] == "_"):
        return None
    if not all(ch.isalnum() or ch in "_:" for ch in name):
        return None
    return name


def is_simple_scalar_var_word(text: str) -> bool:
    """Return ``True`` when *text* is exactly one ``$var`` / ``${var}`` scalar
    variable reference (no array index), named with the conventional shape.

    Token-based replacement for the simple-var-word regexes: it follows
    Tcl's real variable-name rules (only ``::`` is a namespace separator, a
    lone ``:`` ends the name), so it is stricter than the old regex on
    malformed input and identical on valid input.
    """
    return extract_scalar_var_name(text) is not None


def scan_command_substitutions(
    text: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> list[Token]:
    """Return the ``[...]`` command-substitution (``CMD``) tokens in *text*.

    Replaces the ``TclLexer(text).tokenise_all()`` + ``CMD``-filter loop the
    embedded-command scanners hand-rolled.  Positions are absolute at the given
    anchoring, so callers that recurse into a nested word pass that word's
    content base.
    """
    tokens, _ = tokenise(text, base_offset, base_line, base_col)
    return [t for t in tokens if t.type is TokenType.CMD]


def single_command_substitution(text: str) -> Token | None:
    """Return the sole ``CMD`` token when *text* is exactly one ``[cmd]`` word.

    The first token must be a ``CMD`` and only ``EOL`` / ``SEP`` may follow
    (i.e. ``text`` is a single bare command substitution, not a braced literal
    that merely contains brackets).  Returns ``None`` otherwise.
    """
    tokens, _ = tokenise(text, 0, 0, 0)
    if not tokens or tokens[0].type is not TokenType.CMD:
        return None
    for tok in tokens[1:]:
        if tok.type not in (TokenType.EOL, TokenType.SEP):
            return None
    return tokens[0]


def parse_single_command(
    text: str,
    *,
    piece: Callable[[Token], str] = word_piece,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Parse a single Tcl command into ``(argv_texts, argv_tokens, argv_single)``.

    Returns ``None`` if *text* contains zero commands or more than one.  Each
    word's text is reconstructed via *piece* (default :func:`word_piece` —
    natural spelling: bare scalars normalise to ``${name}``, bare arrays stay
    bare); pass another reconstructor — e.g. ``lambda t: t.text`` for the raw
    inner text — when a caller wants a different spelling.  *base_offset* /
    *base_line* / *base_col* anchor the lexer so each returned token carries
    absolute positions (pass the content base when *text* is a region substring).
    """
    tokens, _ = tokenise(text, base_offset, base_line, base_col)
    commands: list[tuple[list[str], list[Token], list[bool]]] = []
    argv_texts: list[str] = []
    argv_tokens: list[Token] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL

    def flush() -> None:
        nonlocal argv_texts, argv_tokens, argv_single
        if argv_texts:
            commands.append((argv_texts, argv_tokens, argv_single))
        argv_texts = []
        argv_tokens = []
        argv_single = []

    for tok in tokens:
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type is TokenType.EOL:
            flush()
            prev_type = tok.type
            continue
        if tok.type is TokenType.EXPAND:
            # ``{*}`` is a word-prefix *marker*, not a fragment: it carries no
            # text and is not the representative token of the word it expands.
            # The lexer only emits it immediately before that word (a bare/
            # trailing ``{*}`` lexes as a literal ``STR``), so treating it as a
            # word boundary makes the following real token start a fresh word —
            # matching the CST, which models ``{*}`` as a separate expand_marker
            # (C Tcl ``tclParse.c`` TCL_TOKEN_EXPAND_WORD), so the word's
            # representative token and ``single`` flag agree with the CommandTokens
            # the segmenter attaches.
            prev_type = TokenType.SEP
            continue
        frag = piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(frag)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += frag
                argv_single[-1] = False
            else:
                argv_texts.append(frag)
                argv_tokens.append(tok)
                argv_single.append(True)
        prev_type = tok.type

    flush()
    if len(commands) != 1:
        return None
    return commands[0]


def extract_single_expr_argument(
    cmd_text: str,
    *,
    expr_aliases: frozenset[str] | None = None,
) -> str | None:
    """Return the expr argument if *cmd_text* is exactly ``expr <one-arg>``.

    When *expr_aliases* is provided, those names are also accepted as ``expr``.
    Returns ``None`` for multi-command bodies (``expr X; cmd2``) — without the
    trailing-EOL check the lexer would break at the first ``;`` and silently
    discard the second command.  Words round-trip *verbatim* (no normalisation)
    so the extracted argument is exactly what the user typed.
    """
    tokens, _ = tokenise(cmd_text, 0, 0, 0)
    argv_texts: list[str] = []
    argv_single: list[bool] = []
    prev_type = TokenType.EOL
    saw_eol = False

    for tok in tokens:
        if tok.type is TokenType.EOL:
            saw_eol = True
            prev_type = tok.type
            continue
        if tok.type in (TokenType.SEP, TokenType.COMMENT):
            prev_type = tok.type
            continue
        if saw_eol:
            return None
        piece = word_piece(tok, normalise_var_braces=False)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_single.append(True)
        else:
            argv_texts[-1] += piece
            argv_single[-1] = False
        prev_type = tok.type

    if len(argv_texts) != 2:
        return None
    cmd_name = argv_texts[0]
    is_expr = cmd_name == "expr" or (expr_aliases is not None and cmd_name in expr_aliases)
    if not is_expr or not argv_single[1]:
        return None
    return argv_texts[1]


def iter_region_words(
    text: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> Iterator[tuple[Token, str, bool]]:
    """Yield ``(first_token, word_text, is_word_start)`` for each word in *text*.

    A "word" is the run of adjacent non-separator tokens between ``SEP`` / ``EOL``
    boundaries (so ``a$b[c]`` is one word).  ``first_token`` is the leading token
    of the word (useful for absolute positions); ``word_text`` is the
    source-faithful reconstruction via :func:`word_piece` (no codegen marker);
    ``is_word_start`` is ``True`` for the first word after a separator.  Backs the
    switch-body / case-list element splitters that hand-rolled this loop.
    """
    tokens, _ = tokenise(text, base_offset, base_line, base_col)
    prev_type = TokenType.EOL
    first_tok: Token | None = None
    word = ""
    word_start = True

    for tok in tokens:
        if tok.type is TokenType.COMMENT:
            continue
        if tok.type in (TokenType.SEP, TokenType.EOL):
            if first_tok is not None:
                yield first_tok, word, word_start
                first_tok = None
                word = ""
            word_start = True
            prev_type = tok.type
            continue
        if tok.type is TokenType.EXPAND:
            # ``{*}`` marker: not a fragment and not the word's representative
            # token.  It only appears immediately before the word it expands, so
            # treat it as a word boundary — the following real token becomes the
            # word's ``first_token`` (matching the CST's expand_marker model).
            prev_type = TokenType.SEP
            continue
        piece = word_piece(tok, bare_arrays_split=False)
        if first_tok is None:
            first_tok = tok
            word = piece
            word_start = prev_type in (TokenType.SEP, TokenType.EOL)
        else:
            word += piece
        prev_type = tok.type
    if first_tok is not None:
        yield first_tok, word, word_start
