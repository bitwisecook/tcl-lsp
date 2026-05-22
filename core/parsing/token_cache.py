"""Per-analysis tokenisation memo shared across re-lexing subsystems.

The analysis pipeline tokenises the same source bytes many times: the
segmenter, the lowerer, ``compiler_checks``, and several analysis passes
each build their own :class:`~core.parsing.lexer.TclLexer` over overlapping
regions.  A proc body, for example, is segmented once by the lowerer and
re-lexed again by ``compiler_checks._process_text`` — independently.

This module provides a context-scoped memo that all those consumers consult
through :func:`tokenise_cached`.  The first consumer to lex a region pays the
tokenisation cost; every later consumer of the *same* region (same bytes,
same base offset, same mode) reuses the cached token list.

Correctness rules:

- The cache is keyed by ``(base_offset, base_line, base_col, insidequote,
  text)``.  ``text`` is part of the key so that two distinct substrings that
  happen to be lexed at the same base offset (e.g. two different proc bodies
  both lexed at base offset 0 by refactoring helpers) never collide.
- Tokens are immutable (frozen dataclasses), so the cached list is shared
  read-only.  Consumers build their own derived structures and never mutate
  the returned list or its tokens.
- A region lexed with ``virtual_insertions`` (error-recovery's injected
  delimiters) is **never** cached — the insertions change the output and are
  request-specific.
- The active cache lives in a :class:`contextvars.ContextVar`, so it is
  per-thread / per-async-context and bounded to the lifetime of the
  :func:`token_cache_scope` block.  Lexer-affecting context (the active
  dialect, strict-quoting) is stable for the duration of one scope, so a
  fresh cache per scope cannot mix outputs produced under different modes.
"""

from __future__ import annotations

from contextlib import contextmanager
from contextvars import ContextVar
from typing import TYPE_CHECKING

from .lexer import TclLexer

if TYPE_CHECKING:
    from collections.abc import Iterator

    from .tokens import SourcePosition, Token

# A cached entry is the token list plus the lexer warnings harvested while
# producing it.  Both are shared read-only across consumers.
_Entry = tuple["list[Token]", "list[tuple[SourcePosition, str]]"]
_Key = tuple[int, int, int, bool, str]


class TokenStreamCache:
    """A per-analysis memo of tokenised regions.

    Lifetime is bounded by :func:`token_cache_scope`; the dict is discarded
    when the scope exits, so memory is bounded by the regions lexed within a
    single analysis pass.
    """

    __slots__ = ("_entries",)

    def __init__(self) -> None:
        self._entries: dict[_Key, _Entry] = {}

    def get(self, key: _Key) -> _Entry | None:
        return self._entries.get(key)

    def put(self, key: _Key, entry: _Entry) -> None:
        self._entries[key] = entry


_active_cache: ContextVar[TokenStreamCache | None] = ContextVar(
    "tcl_token_stream_cache", default=None
)


@contextmanager
def token_cache_scope() -> Iterator[TokenStreamCache]:
    """Activate a shared tokenisation memo for the enclosed block.

    Reentrant: if a cache is already active (an outer scope), the same cache
    is reused so nested analysis stages keep sharing rather than shadowing
    each other.  The cache is cleared (reset) only by the outermost scope.
    """
    existing = _active_cache.get()
    if existing is not None:
        yield existing
        return
    cache = TokenStreamCache()
    token = _active_cache.set(cache)
    try:
        yield cache
    finally:
        _active_cache.reset(token)


def active_token_cache() -> TokenStreamCache | None:
    """Return the cache active in the current context, or ``None``."""
    return _active_cache.get()


def tokenise_cached(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    *,
    insidequote: bool = False,
    virtual_insertions: dict[int, str] | None = None,
) -> _Entry:
    """Tokenise *text* at the given base position, consulting the active memo.

    Returns ``(tokens, warnings)`` where *tokens* is the full token stream
    (``TclLexer.tokenise_all()``) and *warnings* are the non-fatal lexer
    warnings collected while producing it.  Both are shared read-only — do
    not mutate them.

    When no cache is active, or *virtual_insertions* is provided, the region
    is lexed directly without caching.
    """
    cache = _active_cache.get()
    if cache is None or virtual_insertions:
        return _lex(text, base_offset, base_line, base_col, insidequote, virtual_insertions)

    key: _Key = (base_offset, base_line, base_col, insidequote, text)
    hit = cache.get(key)
    if hit is not None:
        return hit
    entry = _lex(text, base_offset, base_line, base_col, insidequote, None)
    cache.put(key, entry)
    return entry


def _lex(
    text: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    insidequote: bool,
    virtual_insertions: dict[int, str] | None,
) -> _Entry:
    lexer = TclLexer(
        text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
        virtual_insertions=virtual_insertions,
    )
    if insidequote:
        lexer.insidequote = True
    tokens = lexer.tokenise_all()
    return tokens, lexer.warnings
