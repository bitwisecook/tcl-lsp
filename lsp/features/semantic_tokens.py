"""Semantic token provider for Tcl source — implementation in _semantic_tokens/ package."""

from ._semantic_tokens import (
    SEMANTIC_TOKEN_MODIFIERS,
    # Public API (lsp/server.py, tests)
    SEMANTIC_TOKEN_TYPES,
    compute_semantic_tokens_edits,
    precompute_chunk_tokens,
    semantic_tokens_full,
)

__all__ = [
    "SEMANTIC_TOKEN_TYPES",
    "SEMANTIC_TOKEN_MODIFIERS",
    "semantic_tokens_full",
    "compute_semantic_tokens_edits",
    "precompute_chunk_tokens",
]
