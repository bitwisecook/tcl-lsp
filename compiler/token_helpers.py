"""Shared token-processing helpers for the compiler pipeline.

Small utilities for extracting word text from tokens and parsing
decimal integer literals.  Used by lowering, optimiser, and other
compiler modules that walk raw token streams.
"""

from __future__ import annotations

from compiler.parsing.green_tree import tokenise
from shared.tokens import Token, TokenType

from .eval_helpers import DECIMAL_INT_RE


def word_piece(tok: Token) -> str:
    """Return the source-level text fragment for a single token.

    Variables are prefixed with ``$`` and command substitutions are
    wrapped in ``[...]`` so that the result mirrors what the user wrote.

    The braced-vs-bare distinction is carried by *natural spelling* so
    that codegen can resolve each form to the bytecode tclsh 9 emits:

    * Braced ``${a(1)}`` loads the WHOLE literal name ``a(1)`` (the
      runtime resolves it as array element ``a(1)``): ``push "a(1)";
      loadStk``.  Reconstructed verbatim as ``${a(1)}``.
    * Bare ``$a(1)`` SPLITS into array ``a`` element ``1``:
      ``push "a"; push "1"; loadArrayStk``.  Kept bare as ``$a(1)`` so
      codegen takes the split path.  ``$a($i)`` substitutes the index.
    * Bare scalar ``$foo`` loads via the whole name; normalised to the
      ``${foo}`` form (equivalent spelling) so the resolvable-reference
      check recognises it.

    Names containing ``}`` cannot use the ``${...}`` wrapper (the first
    ``}`` would close it prematurely), so they fall back to ``$name``.
    """
    if tok.type is TokenType.VAR:
        # Detect braced ``${...}`` vs bare ``$name`` form from the
        # token span.  ``Token.end.offset`` is the position of the
        # *last* source byte covered by the token (inclusive — see
        # ``compiler/parsing/lexer.py``); ``tok.text`` excludes the
        # leading ``$`` for bare and excludes both braces for
        # ``${…}``.  Net delta is 0 for bare and 1 for braced
        # (the leading ``$`` and the closing ``}`` cancel against
        # the omitted ``${``+``}`` so only one source char is
        # un-accounted-for in the braced form).
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        if is_braced:
            # Braced form: reconstruct ``${name}`` verbatim so codegen
            # loads the whole literal name (``${a(1)}`` is the array
            # element ``a(1)`` loaded by whole name via loadStk; ``${foo}``
            # is a plain scalar).  A name containing ``}`` can't round-trip
            # through the wrapper, so keep it bare.
            if "}" in tok.text:
                return "$" + tok.text
            return f"${{{tok.text}}}"
        # Bare array-shaped ``$arr(idx)`` must stay bare so codegen splits
        # it into an array load (``push "arr"; push "idx"; loadArrayStk``).
        # This covers literal indices (``$a(1)``) and substituted ones
        # (``$a($i)`` / ``$numargErrors($cmd)`` — cmdAH-1.4 / 1.5) alike;
        # the ``${…}`` wrapper below would collapse the recursive
        # substitution into a single whole-name lookup.
        if "(" in tok.text and tok.text.endswith(")"):
            return "$" + tok.text
        # Bare scalar: normalise to ``${name}`` form.  Names with ``}``
        # (e.g. ``a(1[expr {3 - 1}])``) would cause the first ``}`` to
        # prematurely close the ``${...}`` form during runtime
        # substitution, so keep those bare.
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


def parse_command_words(
    text: str,
) -> tuple[list[str], list[Token], list[bool]] | None:
    """Parse a single Tcl command into ``(argv_texts, argv_tokens, argv_single)``.

    Returns ``None`` if *text* contains zero commands or more than one.
    Each word's text is reconstructed via :func:`word_piece`, so variable
    references are normalised to ``${name}`` form.
    """
    lex_tokens, _ = tokenise(text, 0, 0, 0)
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

    for tok in lex_tokens:
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


def parse_decimal_int(text: str) -> str | None:
    """Parse *text* as a decimal integer, returning its canonical ``str`` form.

    Returns ``None`` if *text* is not a valid decimal integer.
    """
    value = text.strip()
    if not DECIMAL_INT_RE.fullmatch(value):
        return None
    try:
        return str(int(value))
    except ValueError:
        return None
