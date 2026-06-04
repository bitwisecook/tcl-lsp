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
``green_tree.descend_command``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from shared.tokens import Token, TokenType

from .green_tree import tokenise

if TYPE_CHECKING:
    from collections.abc import Iterator

# Compiler-internal marker for a braced scalar whose name is array-shaped.
# ``${a(1)}`` in source refers to a *scalar* named ``a(1)`` — braces suppress
# array parsing — unlike bare ``$a(1)`` (array ``a`` element ``1``).
# :func:`word_piece` reconstructs the braced-scalar word as ``$={name}`` so the
# rest of the pipeline can tell the two apart (codegen emits push + loadStk for
# the marked form rather than an array load).  It is a *variable reference*,
# never a literal — analyses must decode it, not treat it as opaque text.
BRACED_SCALAR_MARKER_PREFIX = "$={"


def contains_braced_scalar_marker(text: str) -> bool:
    """True if *text* embeds a ``$={name}`` braced-scalar variable marker."""
    return BRACED_SCALAR_MARKER_PREFIX in text


def word_piece(
    tok: Token,
    *,
    array_codegen_marker: bool = True,
    normalise_var_braces: bool = True,
) -> str:
    """Return the source-level text fragment for a single token.

    Variables are prefixed with ``$`` and command substitutions are wrapped in
    ``[...]`` so that the result mirrors what the user wrote.  Two flags select
    the three historically-divergent reconstructions this subsumes:

    * ``array_codegen_marker`` (default ``True``) — emit the ``$={name}`` marker
      for a *braced* array-shaped name (``${a(1)}``), which is a scalar, not an
      array access.  Codegen/lowering/optimiser want this; the parsing-layer
      reconstructions pass ``False``.
    * ``normalise_var_braces`` (default ``True``) — normalise a bare,
      non-substituted ``$name`` / ``$a(1)`` to the equivalent ``${name}`` form.
      Pass ``False`` for *verbatim* round-tripping (the expr-argument matcher
      wants exactly what the user typed).

    A bare ``$arr(idx)`` whose index is *substituted* (``$x`` / ``[cmd]`` inside
    the parens) always round-trips verbatim regardless of the flags — wrapping
    it in ``${…}`` would collapse the recursive substitution into a literal
    scalar lookup (cmdAH-1.4 / 1.5).  A name containing ``}`` is also emitted as
    ``$name`` so the first ``}`` cannot prematurely close a ``${…}`` form.
    """
    if tok.type is TokenType.VAR:
        # ``Token.end.offset`` is the last source byte (inclusive); ``tok.text``
        # excludes the leading ``$`` (bare) or both braces (``${…}``).  Net
        # delta is 0 for bare and 1 for braced.
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        if array_codegen_marker and is_braced and "(" in tok.text and tok.text.endswith(")"):
            return f"{BRACED_SCALAR_MARKER_PREFIX}{tok.text}}}"
        if (
            not is_braced
            and "(" in tok.text
            and tok.text.endswith(")")
            and ("$" in tok.text or "[" in tok.text)
        ):
            return "$" + tok.text
        if "}" in tok.text:
            return "$" + tok.text
        if not normalise_var_braces and not is_braced:
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
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Parse a single Tcl command into ``(argv_texts, argv_tokens, argv_single)``.

    Returns ``None`` if *text* contains zero commands or more than one.  Each
    word's text is reconstructed via :func:`word_piece` (codegen-marker form),
    so variable references are normalised to ``${name}``.
    """
    tokens, _ = tokenise(text, 0, 0, 0)
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
        piece = word_piece(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv_texts.append(piece)
            argv_tokens.append(tok)
            argv_single.append(True)
        else:
            if argv_texts:
                argv_texts[-1] += piece
                argv_single[-1] = False
            else:
                argv_texts.append(piece)
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
        piece = word_piece(tok, array_codegen_marker=False, normalise_var_braces=False)
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
        piece = word_piece(tok, array_codegen_marker=False)
        if first_tok is None:
            first_tok = tok
            word = piece
            word_start = prev_type in (TokenType.SEP, TokenType.EOL)
        else:
            word += piece
        prev_type = tok.type
    if first_tok is not None:
        yield first_tok, word, word_start
