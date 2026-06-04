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

from shared.tokens import Token, TokenType

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
