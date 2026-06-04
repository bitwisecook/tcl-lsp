"""Lossless, context-aware green token tree for the analysis pipeline.

Tcl tokenises differently depending on context: a braced body is opaque
until re-lexed as a script, a command substitution is opaque until its inner
text is re-lexed, double-quoted text suppresses word splitting, and
expressions use a separate tokeniser.  Because of this the pipeline cannot
flatten the document once and reuse one stream everywhere — each region must
be tokenised in its own *mode*.

This module is the shared substrate for that.  A :class:`GreenNode` owns the
token stream for one region, tagged with the :class:`Mode` it was lexed in,
and lazily *descends* into the opaque ``{...}`` / ``[...]`` leaves it
contains — re-lexing each child region once and memoising the result on the
parent.  The segmenter, ``compiler_checks``, and ``var_refs`` all consume the
same descent rather than each re-lexing overlapping regions.

This subsumes the flat tokenisation memo that preceded it (the old
``token_cache`` module): a region interned in the active scope is a
:class:`GreenNode`, and the per-node ``_descended`` map is the memo for
nesting.  See ``docs/design/compiler/green-token-tree.md``.

Correctness rules carried over from the flat memo:

- A region is identified by ``(base_offset, base_line, base_col, mode, text)``.
  ``text`` is part of the key so two distinct substrings lexed at the same
  base offset (e.g. two proc bodies both scanned at base 0 by ``var_refs``)
  never collide.
- Tokens are immutable (frozen dataclasses); a node's stream is shared
  read-only.  Consumers build their own derived structures and never mutate
  the returned tuples or their tokens.
- A region lexed with ``virtual_insertions`` (error recovery's injected
  delimiters) is **never** interned — the insertions change the output and
  are request-specific.
- The active scope lives in a :class:`contextvars.ContextVar`, so it is
  per-thread / per-async-context and bounded to the lifetime of the
  :func:`green_tree_scope` block.  Lexer-affecting context (dialect,
  strict-quoting) is stable for the duration of one scope, so a fresh scope
  cannot mix outputs produced under different modes.
"""

from __future__ import annotations

from contextlib import contextmanager
from contextvars import ContextVar
from dataclasses import dataclass, field, replace
from enum import Enum, auto
from typing import TYPE_CHECKING

from shared.tokens import SourcePosition, TokenType

from .lexer import TclLexer

if TYPE_CHECKING:
    from collections.abc import Iterator

    from shared.tokens import Token

_Warnings = tuple[tuple[SourcePosition, str], ...]


class Mode(Enum):
    """The tokenisation mode a region was lexed in.

    The mode is load-bearing: re-tokenising a region in the wrong mode
    silently corrupts analysis, so every node records the mode explicitly.
    """

    SCRIPT = auto()  # full Tcl tokenisation: $ / [ active, comments at command start
    QUOTED = auto()  # inside "...": substitutions active, no word splitting/comments
    EXPR = auto()  # expr argument / condition — opaque here, parsed by expr_lexer
    RAW = auto()  # braced data word — never descended as script unless asked


class NodeKind(Enum):
    """Structural role of a node in the tree."""

    ROOT = auto()  # top-level document region
    BRACED = auto()  # a {...} body re-lexed as script
    BRACKETED = auto()  # a [...] command substitution's inner text
    QUOTED = auto()  # a "..." region
    EXPR = auto()  # an expression region
    ERROR = auto()  # a recovery / unterminated region


# STR tokens come from braced words and CMD tokens from command substitutions;
# both anchor their inner text one byte past the opening delimiter.
_OPAQUE_TYPES = (TokenType.STR, TokenType.CMD)


@dataclass(slots=True)
class GreenNode:
    """One tokenised region of source, with lazy memoised descent.

    A node holds the token stream for its region (``tokens``), tagged with the
    :class:`Mode` it was lexed in.  ``descend(token)`` re-lexes the inner text
    of an opaque ``{...}`` / ``[...]`` leaf within this region as a child node,
    memoising it so every later consumer reuses it.

    Positions on ``tokens`` are absolute (anchored at ``base_offset`` /
    ``base_line`` / ``base_col``); ``width`` is the byte length of the region's
    text, used to locate and offset-shift nodes during incremental update.
    """

    kind: NodeKind
    mode: Mode
    text: str
    base_offset: int
    base_line: int
    base_col: int
    insidequote: bool
    width: int
    tokens: tuple[Token, ...]
    warnings: _Warnings
    _descended: dict[tuple[int, Mode], GreenNode] = field(default_factory=dict)

    def descend(self, token: Token, mode: Mode = Mode.SCRIPT) -> GreenNode:
        """Return the child node for an opaque ``token`` within this region.

        For ``STR`` / ``CMD`` tokens the child covers the inner text anchored
        one byte past the opening delimiter; for other tokens (e.g. an ESC
        recovery body) the child is anchored at the token's own position.

        The child shares the interned tokenisation of its region (so a body
        descended here and tokenised elsewhere is lexed once), but its *kind*
        is resolved in this parent's context: an opaque token whose closing
        delimiter is missing in this region yields an :attr:`NodeKind.ERROR`
        node — the lossless representation of an unterminated/recovered region
        (phase 5).  Memoised on this node keyed by ``(token offset, mode)``.
        """
        key = (token.start.offset, mode)
        cached = self._descended.get(key)
        if cached is not None:
            return cached
        child = _descend(token, mode, self.text, self.base_offset)
        self._descended[key] = child
        return child


def _delimiter_terminated(token: Token, text: str, base_offset: int) -> bool:
    """Return whether *token*'s closing delimiter is present in *text*.

    *text* is the region containing *token* and *base_offset* its absolute
    anchor.  The opening delimiter sits at ``token.start.offset`` and the inner
    content occupies the ``len(token.text)`` bytes after it, so a terminated
    ``{...}`` / ``[...]`` has its closing brace/bracket immediately past that.
    Deriving the closer position from the token's *length* (rather than
    ``token.end.offset``) is what classifies empty regions correctly: for an
    empty ``{}`` / ``[]`` the lexer reports ``token.end.offset`` *at* the
    closer, which ``end.offset + 1`` would skip over.
    """
    idx = token.start.offset + 1 + len(token.text) - base_offset
    if idx < 0 or idx >= len(text):
        return False
    close = "}" if token.type is TokenType.STR else "]"
    return text[idx] == close


def _descend(token: Token, mode: Mode, context_text: str, context_base: int) -> GreenNode:
    """Build the child node for *token*, resolving ERROR kind in context.

    *context_text* / *context_base* describe the region containing *token*
    (used only to decide whether the delimiter is terminated).  The child's
    token stream is obtained through the shared intern index, so descent does
    not re-lex a region another consumer already tokenised; only the
    context-dependent ``kind`` is applied per call via a thin wrapper.
    """
    if token.type in _OPAQUE_TYPES:
        b_off = token.start.offset + 1
        b_line = token.start.line
        b_col = token.start.character + 1
        structural = NodeKind.BRACKETED if token.type is TokenType.CMD else NodeKind.BRACED
        kind = (
            structural
            if _delimiter_terminated(token, context_text, context_base)
            else NodeKind.ERROR
        )
    else:
        b_off = token.start.offset
        b_line = token.start.line
        b_col = token.start.character
        kind = NodeKind.ERROR
    shared = node_for(token.text, b_off, b_line, b_col, mode=mode)
    if shared.kind is kind:
        return shared
    # Context-tagged wrapper: same interned tokens/warnings, region-specific
    # kind.  The shared node's own kind is whatever its first interner set, so
    # it is never relied on for the ERROR distinction.
    return replace(shared, kind=kind)


def _lex(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    insidequote: bool,
    virtual_insertions: dict[int, str] | None,
    line_starts: list[int] | None = None,
) -> tuple[list[Token], list[tuple[SourcePosition, str]]]:
    lexer = TclLexer(
        text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
        virtual_insertions=virtual_insertions,
        line_starts=line_starts,
    )
    if insidequote:
        lexer.insidequote = True
    tokens = lexer.tokenise_all()
    return tokens, lexer.warnings


def _build_node(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    mode: Mode,
    kind: NodeKind,
    line_starts: list[int] | None = None,
) -> GreenNode:
    insidequote = mode is Mode.QUOTED
    tokens, warnings = _lex(text, base_offset, base_line, base_col, insidequote, None, line_starts)
    return GreenNode(
        kind=kind,
        mode=mode,
        text=text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
        insidequote=insidequote,
        width=len(text),
        tokens=tuple(tokens),
        warnings=tuple(warnings),
    )


# --- analysis-scoped intern index --------------------------------------------
#
# Cross-consumer sharing (the segmenter scanning a body, ``compiler_checks``
# descending the same body, the lowerer segmenting it) needs a common lookup
# keyed by region identity, because those callers reach a region independently
# rather than always descending from one parent.  The scope holds that intern
# index; ``descend`` registers its children in it so a standalone lookup of the
# same region returns the already-built node.

_Key = tuple[int, int, int, Mode, str]


class GreenTreeScope:
    """A per-analysis intern index of :class:`GreenNode` regions.

    Lifetime is bounded by :func:`green_tree_scope`; the index is discarded
    when the scope exits, so memory is bounded by the regions lexed within a
    single analysis pass.
    """

    __slots__ = ("_index",)

    def __init__(self) -> None:
        self._index: dict[_Key, GreenNode] = {}

    def intern(self, key: _Key, node: GreenNode) -> None:
        self._index[key] = node

    def get(self, key: _Key) -> GreenNode | None:
        return self._index.get(key)


_active_scope: ContextVar[GreenTreeScope | None] = ContextVar("tcl_green_tree_scope", default=None)


@contextmanager
def green_tree_scope() -> Iterator[GreenTreeScope]:
    """Activate a shared green-tree intern index for the enclosed block.

    Reentrant: an inner scope reuses the active outer scope so nested analysis
    stages keep sharing rather than shadowing each other.  The index is reset
    only by the outermost scope.
    """
    existing = _active_scope.get()
    if existing is not None:
        yield existing
        return
    scope = GreenTreeScope()
    token = _active_scope.set(scope)
    try:
        yield scope
    finally:
        _active_scope.reset(token)


def active_scope() -> GreenTreeScope | None:
    """Return the green-tree scope active in the current context, or ``None``."""
    return _active_scope.get()


def node_for(
    text: str,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
    *,
    mode: Mode = Mode.SCRIPT,
    kind: NodeKind = NodeKind.ROOT,
    virtual_insertions: dict[int, str] | None = None,
    line_starts: list[int] | None = None,
) -> GreenNode:
    """Return the :class:`GreenNode` for *text* at the given anchoring.

    Consults the active scope's intern index; the first caller of a region
    pays the tokenisation cost and every later caller of the *same* region
    reuses the node.  Regions lexed with *virtual_insertions* are never
    interned (the insertions are request-specific).

    *line_starts* is an optional pre-built line index for *text*; it is a pure
    optimisation hint (the lexer would build an identical one) and is not part of
    the intern key.
    """
    scope = _active_scope.get()
    if scope is None or virtual_insertions:
        if virtual_insertions:
            insidequote = mode is Mode.QUOTED
            tokens, warnings = _lex(
                text, base_offset, base_line, base_col, insidequote, virtual_insertions, line_starts
            )
            return GreenNode(
                kind=kind,
                mode=mode,
                text=text,
                base_offset=base_offset,
                base_line=base_line,
                base_col=base_col,
                insidequote=insidequote,
                width=len(text),
                tokens=tuple(tokens),
                warnings=tuple(warnings),
            )
        return _build_node(text, base_offset, base_line, base_col, mode, kind, line_starts)

    key: _Key = (base_offset, base_line, base_col, mode, text)
    hit = scope.get(key)
    if hit is not None:
        return hit
    node = _build_node(text, base_offset, base_line, base_col, mode, kind, line_starts)
    scope.intern(key, node)
    return node


def tokenise(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    *,
    insidequote: bool = False,
    virtual_insertions: dict[int, str] | None = None,
    line_starts: list[int] | None = None,
) -> tuple[tuple[Token, ...], _Warnings]:
    """Tokenise *text* at the given anchoring, consulting the green-tree scope.

    Returns ``(tokens, warnings)`` for the region.  This is the leaf primitive
    consumers use when they want the token stream directly rather than a node;
    it shares the same intern index as :func:`node_for`, so a region tokenised
    here and descended elsewhere is lexed once.  *line_starts* is an optional
    pre-built line index for *text* (a pure optimisation hint).
    """
    mode = Mode.QUOTED if insidequote else Mode.SCRIPT
    node = node_for(
        text,
        base_offset,
        base_line,
        base_col,
        mode=mode,
        virtual_insertions=virtual_insertions,
        line_starts=line_starts,
    )
    return node.tokens, node.warnings
